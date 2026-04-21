//! [`PluginManager`] 服务主体：编排安装 / 启用 / 禁用 / 卸载的生命周期状态机。
//!
//! 关键设计：
//! - Manager 持有 runtime trait 对象映射（`PluginKind -> Arc<dyn PluginRuntime>`），
//!   runtime 自身由 `cycms-plugin-native` / `cycms-plugin-wasm` 实现并在 Kernel
//!   bootstrap 时注入。无 runtime 时 enable 返回明确错误，install / list / uninstall
//!   仍可运行（数据面不依赖 runtime）。
//! - 插件目录约定 `<plugins_root>/<plugin_name>/`，manifest 在目录内的 `plugin.toml`，
//!   `plugin.entry` 是相对插件目录的实现文件路径；enable 时按此约定重建绝对路径。
//! - install 过程不依赖跨子系统数据库事务，而是按迁移 / 权限 / 记录写入的顺序推进；
//!   任一步失败都会触发显式补偿回滚，保证不会留下半安装状态。

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{PluginContext, ServiceRegistry};
use cycms_settings::SettingsManager;
use semver::Version;
use serde_json::json;
use tracing::{info, warn};

use crate::discovery::DiscoveredPlugin;
use crate::frontend_manifest::{ADMIN_SHELL_SDK_VERSION, load_frontend_manifest};
use crate::frontend_snapshot::{
    AdminExtensionBootstrap, AdminExtensionDiagnostics, BootstrapFieldRendererContribution,
    BootstrapMenuContribution, BootstrapPlugin, BootstrapRouteContribution,
    BootstrapSettingsContribution, BootstrapSettingsPage, BootstrapSlotContribution,
    ExtensionDiagnostic, FrontendRuntimeState, ResolvedPluginAsset, build_frontend_runtime_state,
    extension_revision_token, frontend_runtime_state, insert_frontend_runtime_state,
    plugin_admin_full_path, resolve_plugin_asset, validate_cross_plugin_conflicts,
};
use crate::manifest::{PluginKind, PluginManifest};
use crate::model::{NewPluginRow, PluginRecord, PluginStatus};
use crate::repository::PluginRepository;
use crate::resolver::{check_host_compatibility, reverse_dependencies, topological_order};
use crate::runtime::PluginRuntime;

/// `PluginManager` 构造入参，封装 Kernel 侧的不可变配置。
pub struct PluginManagerConfig {
    /// 当前宿主 `cycms` 版本。
    pub cycms_version: Version,
    /// 插件根目录（对应 `cycms.toml` 的 `[plugins] directory`）。
    pub plugins_root: PathBuf,
    /// Kernel 注入的运行时列表；同一种 `kind` 仅保留最后一个实现。
    pub runtimes: Vec<Arc<dyn PluginRuntime>>,
}

/// 对外统一视图（Req 10.6），由 `list` / `install` / `enable` 等方法返回。
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub status: PluginStatus,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
}

impl PluginInfo {
    fn from_record(record: &PluginRecord) -> Self {
        let manifest: Option<PluginManifest> = serde_json::from_value(record.manifest.clone()).ok();
        let dependencies = manifest
            .as_ref()
            .map(|m| {
                let mut names: Vec<String> = m.dependencies.keys().cloned().collect();
                names.sort();
                names
            })
            .unwrap_or_default();
        let permissions = manifest
            .as_ref()
            .map(|m| {
                m.permission_definitions()
                    .into_iter()
                    .map(|d| format!("{}.{}.{}", d.domain, d.resource, d.action))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            name: record.name.clone(),
            version: record.version.clone(),
            kind: record.kind,
            status: record.status,
            dependencies,
            permissions,
        }
    }
}

/// 插件生命周期管理器门面。
pub struct PluginManager {
    repository: PluginRepository,
    migration_engine: Arc<MigrationEngine>,
    permission_engine: Arc<PermissionEngine>,
    settings_manager: Arc<SettingsManager>,
    service_registry: Arc<ServiceRegistry>,
    event_bus: Arc<EventBus>,
    runtimes: RwLock<HashMap<PluginKind, Arc<dyn PluginRuntime>>>,
    plugin_context: Arc<PluginContext>,
    cycms_version: Version,
    plugins_root: PathBuf,
}

impl PluginManager {
    /// 构造 `PluginManager`。初始 runtime 按 `kind` 去重（后注入的覆盖先注入的），
    /// 之后仍可用 [`PluginManager::install_runtime`] 动态追加或替换。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<DatabasePool>,
        migration_engine: Arc<MigrationEngine>,
        permission_engine: Arc<PermissionEngine>,
        settings_manager: Arc<SettingsManager>,
        service_registry: Arc<ServiceRegistry>,
        event_bus: Arc<EventBus>,
        plugin_context: Arc<PluginContext>,
        config: PluginManagerConfig,
    ) -> Self {
        let mut runtimes_map: HashMap<PluginKind, Arc<dyn PluginRuntime>> = HashMap::new();
        for rt in config.runtimes {
            runtimes_map.insert(rt.kind(), rt);
        }
        Self {
            repository: PluginRepository::new(db),
            migration_engine,
            permission_engine,
            settings_manager,
            service_registry,
            event_bus,
            runtimes: RwLock::new(runtimes_map),
            plugin_context,
            cycms_version: config.cycms_version,
            plugins_root: config.plugins_root,
        }
    }

    /// 注册或替换一个运行时（按 `kind` 去重）。Kernel 调用此方法
    /// 注入 Native / Wasm runtime；测试场景亦可用于挂载 mock 实现。
    pub fn install_runtime(&self, runtime: Arc<dyn PluginRuntime>) {
        let mut guard = self
            .runtimes
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        guard.insert(runtime.kind(), runtime);
    }

    /// 安装一个从磁盘发现的插件：校验 → 迁移 → 注册权限 → 插入记录。
    ///
    /// 成功后插件处于 `disabled` 状态，调用方需显式 [`Self::enable`] 才会 load。
    ///
    /// # Errors
    /// - [`cycms_core::Error::ValidationError`]：宿主不兼容 / 依赖缺失 / 版本不匹配
    /// - [`cycms_core::Error::Conflict`]：同名插件已安装
    /// - [`cycms_core::Error::Internal`]：DB / 迁移失败
    pub async fn install(&self, source: &DiscoveredPlugin) -> Result<PluginInfo> {
        self.install_as(source, None).await
    }

    /// 启用已安装插件：依赖检查 → runtime.load → 状态翻转 → `ServiceRegistry` 放行。
    ///
    /// # Errors
    /// - [`cycms_core::Error::NotFound`]：未找到插件
    /// - [`cycms_core::Error::ValidationError`]：依赖未启用
    /// - [`cycms_core::Error::PluginError`]：对应 `kind` 无已注册 runtime
    /// - runtime.load 返回的任何错误
    pub async fn enable(&self, name: &str) -> Result<()> {
        self.enable_as(name, None).await
    }

    /// 启用已安装插件，并把操作者写入 lifecycle 事件。
    ///
    /// # Errors
    /// - [`cycms_core::Error::NotFound`]：未找到插件
    /// - [`cycms_core::Error::ValidationError`]：依赖未启用
    /// - [`cycms_core::Error::PluginError`]：对应 `kind` 无已注册 runtime
    /// - runtime.load 返回的任何错误
    pub async fn enable_as(&self, name: &str, actor_id: Option<&str>) -> Result<()> {
        let record = self.require_record(name).await?;
        self.activate_record(&record, true, true, actor_id).await
    }

    /// 根据数据库中的持久化状态恢复所有已启用插件。
    ///
    /// 该方法在进程启动时使用：按依赖拓扑顺序加载 runtime，但不重复写 DB 状态、
    /// 不重复发布 lifecycle 事件。
    ///
    /// # Errors
    /// 任一已启用插件的 manifest 解析、依赖校验或 runtime load 失败时返回错误。
    pub async fn restore_enabled_plugins(&self) -> Result<()> {
        let records = self.repository.list().await?;
        let mut manifests = Vec::new();
        let mut enabled_records = BTreeMap::<String, PluginRecord>::new();

        for record in records {
            if record.status != PluginStatus::Enabled {
                continue;
            }
            let manifest = Self::manifest_from_record(&record)?;
            manifests.push(manifest);
            enabled_records.insert(record.name.clone(), record);
        }

        let order = topological_order(&manifests)?;
        for plugin_name in order {
            let record = enabled_records
                .get(&plugin_name)
                .ok_or_else(|| Error::Internal {
                    message: format!("enabled plugin record missing during restore: {plugin_name}"),
                    source: None,
                })?;
            self.activate_record(record, false, false, None).await?;
        }

        Ok(())
    }

    /// 禁用已启用插件。存在 enabled 依赖方时，只在 `force` 为 `true` 时级联禁用，
    /// 否则返回 [`cycms_core::Error::Conflict`]。
    ///
    /// # Errors
    /// - [`cycms_core::Error::NotFound`]：未找到插件
    /// - [`cycms_core::Error::Conflict`]：存在依赖方且未开启 `force`
    /// - runtime.unload 返回的任何错误
    pub async fn disable(&self, name: &str, force: bool) -> Result<()> {
        self.disable_as(name, force, None).await
    }

    /// 禁用已启用插件，并把操作者写入 lifecycle 事件。
    ///
    /// # Errors
    /// - [`cycms_core::Error::NotFound`]：未找到插件
    /// - [`cycms_core::Error::Conflict`]：存在依赖方且未开启 `force`
    /// - runtime.unload 返回的任何错误
    pub async fn disable_as(&self, name: &str, force: bool, actor_id: Option<&str>) -> Result<()> {
        Box::pin(self.disable_internal(name, force, actor_id.map(ToOwned::to_owned))).await
    }

    async fn disable_internal(
        &self,
        name: &str,
        force: bool,
        actor_id: Option<String>,
    ) -> Result<()> {
        let record = self.require_record(name).await?;
        if record.status == PluginStatus::Disabled {
            return Ok(());
        }

        let dependents = self.enabled_dependents_of(name).await?;
        if !dependents.is_empty() {
            if !force {
                return Err(Error::Conflict {
                    message: format!(
                        "plugin {name} has enabled dependents: {dependents:?}; \
                         pass force=true to cascade disable"
                    ),
                });
            }
            for dep in dependents {
                Box::pin(self.disable_internal(&dep, true, actor_id.clone())).await?;
            }
        }

        let runtime = self.runtime_for(record.kind)?;
        runtime.unload(name).await?;

        self.repository
            .update_status(name, PluginStatus::Disabled)
            .await?;
        self.service_registry.set_unavailable(name);
        info!(plugin = %name, "plugin disabled");
        self.publish_event(EventKind::PluginDisabled, &record, actor_id.as_deref());
        Ok(())
    }

    /// 卸载插件：若处于 enabled 则先级联 disable，再执行 down migration、
    /// 注销权限 / settings schema，最后删除记录。
    ///
    /// # Errors
    /// 底层任一步骤失败均向上抛出。
    pub async fn uninstall(&self, name: &str) -> Result<()> {
        self.uninstall_as(name, None).await
    }

    /// 卸载插件，并把操作者写入 lifecycle 事件。
    ///
    /// # Errors
    /// 插件不存在、级联禁用失败、down migration 失败、仓库删除失败时返回错误。
    pub async fn uninstall_as(&self, name: &str, actor_id: Option<&str>) -> Result<()> {
        let record = self.require_record(name).await?;
        if record.status == PluginStatus::Enabled {
            self.disable_as(name, true, actor_id).await?;
        }
        let manifest = Self::manifest_from_record(&record)?;
        self.run_plugin_down_migrations(name, &manifest).await?;

        let removed_perms = self
            .permission_engine
            .unregister_permissions_by_source(name)
            .await
            .unwrap_or_else(|e| {
                warn!(plugin = %name, error = %e, "failed to unregister permissions");
                0
            });
        let _ = self
            .settings_manager
            .unregister_schema(name)
            .await
            .unwrap_or_else(|e| {
                warn!(plugin = %name, error = %e, "failed to unregister settings schema");
                false
            });

        self.repository.delete(name).await?;
        info!(
            plugin = %name,
            removed_permissions = removed_perms,
            "plugin uninstalled"
        );
        self.publish_event(EventKind::PluginUninstalled, &record, actor_id);
        Ok(())
    }

    /// 安装插件，并把操作者写入 lifecycle 事件。
    ///
    /// # Errors
    /// - [`cycms_core::Error::ValidationError`]：宿主不兼容 / 依赖缺失 / 版本不匹配
    /// - [`cycms_core::Error::Conflict`]：同名插件已安装
    /// - [`cycms_core::Error::Internal`]：DB / 迁移 / manifest 序列化失败
    pub async fn install_as(
        &self,
        source: &DiscoveredPlugin,
        actor_id: Option<&str>,
    ) -> Result<PluginInfo> {
        let manifest = &source.manifest;
        check_host_compatibility(manifest, &self.cycms_version)?;
        self.check_dependencies_installed(manifest).await?;

        if self
            .repository
            .find_by_name(&manifest.plugin.name)
            .await?
            .is_some()
        {
            return Err(Error::Conflict {
                message: format!("plugin {} already installed", manifest.plugin.name),
            });
        }

        if let Err(err) = self.run_plugin_up_migrations(source).await {
            self.rollback_failed_install(source, false).await?;
            return Err(err);
        }
        let mut should_rollback_permissions = false;

        let defs = manifest.permission_definitions();
        if !defs.is_empty() {
            if let Err(err) = self
                .permission_engine
                .register_permissions(&manifest.plugin.name, defs)
                .await
            {
                self.rollback_failed_install(source, false).await?;
                return Err(err);
            }
            should_rollback_permissions = true;
        }

        let manifest_value = match self.build_manifest_value(source) {
            Ok(value) => value,
            Err(err) => {
                self.rollback_failed_install(source, should_rollback_permissions)
                    .await?;
                return Err(err);
            }
        };
        let row = NewPluginRow {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            kind: manifest.plugin.kind,
            manifest: manifest_value,
        };
        let record = match self.repository.insert(row).await {
            Ok(record) => record,
            Err(err) => {
                self.rollback_failed_install(source, should_rollback_permissions)
                    .await?;
                return Err(err);
            }
        };
        info!(plugin = %record.name, version = %record.version, "plugin installed");
        self.publish_event(EventKind::PluginInstalled, &record, actor_id);
        Ok(PluginInfo::from_record(&record))
    }

    /// 列出所有已安装插件。返回顺序与 [`PluginRepository::list`] 一致（按 name 升序）。
    ///
    /// # Errors
    /// DB 故障 → [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<PluginInfo>> {
        let records = self.repository.list().await?;
        Ok(records.iter().map(PluginInfo::from_record).collect())
    }

    pub async fn admin_extension_bootstrap(
        &self,
        user_id: &str,
        roles: &[String],
    ) -> Result<AdminExtensionBootstrap> {
        let records = self.repository.list().await?;
        let enabled_states = Self::enabled_frontend_states(&records)?;
        let state_refs: Vec<&FrontendRuntimeState> = enabled_states.iter().collect();
        let revision = extension_revision_token(&state_refs)?;

        let mut diagnostics = Vec::new();
        let mut plugins = Vec::new();
        for state in enabled_states {
            if let Some(diagnostic) = Self::frontend_diagnostic(&state) {
                diagnostics.push(diagnostic);
            }
            if !state.compatibility.compatible {
                continue;
            }

            let plugin = self
                .bootstrap_plugin_for_user(user_id, roles, &state)
                .await?;
            if !plugin.is_empty() {
                plugins.push(plugin);
            }
        }

        plugins.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(AdminExtensionBootstrap {
            revision,
            shell_sdk_version: ADMIN_SHELL_SDK_VERSION.to_owned(),
            plugins,
            diagnostics,
        })
    }

    pub async fn admin_extension_diagnostics(&self) -> Result<AdminExtensionDiagnostics> {
        let records = self.repository.list().await?;
        let all_states = Self::frontend_states(&records)?;
        let state_refs: Vec<&FrontendRuntimeState> =
            all_states.iter().map(|(_, state)| state).collect();
        let revision = extension_revision_token(&state_refs)?;
        let diagnostics = all_states
            .into_iter()
            .filter_map(|(_, state)| Self::frontend_diagnostic(&state))
            .collect();

        Ok(AdminExtensionDiagnostics {
            revision,
            diagnostics,
        })
    }

    pub async fn resolve_frontend_asset(
        &self,
        plugin_name: &str,
        version: &str,
        url_hash: &str,
        asset_path: &str,
    ) -> Result<Option<ResolvedPluginAsset>> {
        let Some(record) = self.repository.find_by_name(plugin_name).await? else {
            return Ok(None);
        };
        if record.status != PluginStatus::Enabled {
            return Ok(None);
        }
        let Some(state) = frontend_runtime_state(&record.manifest)? else {
            return Ok(None);
        };

        resolve_plugin_asset(
            &self.plugins_root.join(plugin_name),
            &state,
            version,
            url_hash,
            asset_path,
        )
    }

    async fn activate_record(
        &self,
        record: &PluginRecord,
        persist_status: bool,
        emit_event: bool,
        actor_id: Option<&str>,
    ) -> Result<()> {
        let manifest = Self::manifest_from_record(record)?;
        self.check_dependencies_enabled(&manifest).await?;
        self.validate_frontend_enablement(record).await?;

        let runtime = self.runtime_for(record.kind)?;
        let already_loaded = runtime
            .loaded_plugins()
            .into_iter()
            .any(|loaded| loaded == record.name);
        if !already_loaded {
            let entry_path = self.resolve_entry_path(&record.name, &manifest);
            runtime
                .load(&manifest, &entry_path, Arc::clone(&self.plugin_context))
                .await?;
        }

        if persist_status && record.status != PluginStatus::Enabled {
            self.repository
                .update_status(&record.name, PluginStatus::Enabled)
                .await?;
        }

        self.service_registry.set_available(&record.name);

        if emit_event {
            info!(plugin = %record.name, "plugin enabled");
            self.publish_event(EventKind::PluginEnabled, record, actor_id);
        } else if !already_loaded {
            info!(plugin = %record.name, "plugin restored from persisted enabled state");
        }

        Ok(())
    }

    async fn require_record(&self, name: &str) -> Result<PluginRecord> {
        self.repository
            .find_by_name(name)
            .await?
            .ok_or_else(|| Error::NotFound {
                message: format!("plugin not found: {name}"),
            })
    }

    fn runtime_for(&self, kind: PluginKind) -> Result<Arc<dyn PluginRuntime>> {
        let guard = self.runtimes.read().unwrap_or_else(PoisonError::into_inner);
        guard.get(&kind).cloned().ok_or_else(|| Error::PluginError {
            message: format!("no runtime registered for kind {kind}"),
            source: None,
        })
    }

    fn manifest_from_record(record: &PluginRecord) -> Result<PluginManifest> {
        serde_json::from_value(record.manifest.clone()).map_err(|e| Error::Internal {
            message: format!("deserialize manifest for plugin {}: {e}", record.name),
            source: None,
        })
    }

    fn resolve_entry_path(&self, plugin_name: &str, manifest: &PluginManifest) -> PathBuf {
        self.plugins_root
            .join(plugin_name)
            .join(&manifest.plugin.entry)
    }

    fn build_manifest_value(&self, source: &DiscoveredPlugin) -> Result<serde_json::Value> {
        let mut manifest_value =
            serde_json::to_value(&source.manifest).map_err(|err| Error::Internal {
                message: format!("serialize manifest: {err}"),
                source: None,
            })?;

        if let Some(spec) = &source.manifest.frontend {
            let frontend_manifest = load_frontend_manifest(&source.directory, spec)?;
            let frontend_state = build_frontend_runtime_state(
                &source.directory,
                &source.manifest,
                frontend_manifest,
            )?;
            insert_frontend_runtime_state(&mut manifest_value, &frontend_state)?;
        }

        Ok(manifest_value)
    }

    async fn validate_frontend_enablement(&self, record: &PluginRecord) -> Result<()> {
        let Some(current_state) = frontend_runtime_state(&record.manifest)? else {
            return Ok(());
        };

        if current_state.required && !current_state.compatibility.compatible {
            return Err(Error::ValidationError {
                message: current_state
                    .compatibility
                    .reason
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "plugin {} frontend is incompatible with admin shell {}",
                            record.name, ADMIN_SHELL_SDK_VERSION
                        )
                    }),
                details: None,
            });
        }

        if !current_state.compatibility.compatible {
            return Ok(());
        }

        let records = self.repository.list().await?;
        let mut states = Vec::new();
        for other in records {
            if other.name == record.name || other.status != PluginStatus::Enabled {
                continue;
            }
            if let Some(state) = frontend_runtime_state(&other.manifest)?
                && state.compatibility.compatible
            {
                states.push(state);
            }
        }
        states.push(current_state);

        let refs: Vec<&FrontendRuntimeState> = states.iter().collect();
        validate_cross_plugin_conflicts(&refs)
    }

    fn frontend_states(
        records: &[PluginRecord],
    ) -> Result<Vec<(PluginStatus, FrontendRuntimeState)>> {
        let mut states = Vec::new();
        for record in records {
            if let Some(state) = frontend_runtime_state(&record.manifest)? {
                states.push((record.status, state));
            }
        }
        Ok(states)
    }

    fn enabled_frontend_states(records: &[PluginRecord]) -> Result<Vec<FrontendRuntimeState>> {
        let mut states = Vec::new();
        for record in records {
            if record.status != PluginStatus::Enabled {
                continue;
            }
            if let Some(state) = frontend_runtime_state(&record.manifest)? {
                states.push(state);
            }
        }
        Ok(states)
    }

    fn frontend_diagnostic(state: &FrontendRuntimeState) -> Option<ExtensionDiagnostic> {
        if state.compatibility.compatible {
            return None;
        }

        Some(ExtensionDiagnostic {
            plugin_name: state.snapshot.plugin_name.clone(),
            plugin_version: state.snapshot.plugin_version.clone(),
            severity: if state.required {
                "error".to_owned()
            } else {
                "warning".to_owned()
            },
            code: "shell_sdk_incompatible".to_owned(),
            message: state.compatibility.reason.clone().unwrap_or_else(|| {
                format!(
                    "plugin {} frontend is incompatible with admin shell {}",
                    state.snapshot.plugin_name, ADMIN_SHELL_SDK_VERSION
                )
            }),
        })
    }

    async fn bootstrap_plugin_for_user(
        &self,
        user_id: &str,
        roles: &[String],
        state: &FrontendRuntimeState,
    ) -> Result<BootstrapPlugin> {
        let plugin_name = state.snapshot.plugin_name.as_str();
        let plugin_version = state.snapshot.plugin_version.as_str();

        let mut menus = Vec::new();
        for menu in &state.snapshot.menus {
            if !self
                .has_all_permissions(user_id, roles, &menu.required_permissions)
                .await?
            {
                continue;
            }
            menus.push(BootstrapMenuContribution {
                id: menu.id.clone(),
                label: menu.label.clone(),
                zone: menu.zone.clone(),
                icon: menu.icon.clone(),
                order: menu.order,
                to: menu.to.clone(),
                full_path: plugin_admin_full_path(plugin_name, &menu.to),
                required_permissions: menu.required_permissions.clone(),
            });
        }
        menus.sort_by(|left, right| {
            left.zone
                .cmp(&right.zone)
                .then(left.order.cmp(&right.order))
                .then(left.label.cmp(&right.label))
                .then(left.id.cmp(&right.id))
        });

        let mut routes = Vec::new();
        for route in &state.snapshot.routes {
            if !self
                .has_all_permissions(user_id, roles, &route.required_permissions)
                .await?
            {
                continue;
            }
            let (module_url, styles) = Self::asset_urls_for(state, &route.module_asset_id)?;
            routes.push(BootstrapRouteContribution {
                id: route.id.clone(),
                path: route.path.clone(),
                full_path: plugin_admin_full_path(plugin_name, &route.path),
                module_url,
                styles,
                kind: route.kind.clone(),
                title: route.title.clone(),
                required_permissions: route.required_permissions.clone(),
                r#match: route.r#match.clone(),
            });
        }
        routes.sort_by(|left, right| left.path.cmp(&right.path).then(left.id.cmp(&right.id)));

        let mut slots = Vec::new();
        for slot in &state.snapshot.slots {
            if !self
                .has_all_permissions(user_id, roles, &slot.required_permissions)
                .await?
            {
                continue;
            }
            let (module_url, styles) = Self::asset_urls_for(state, &slot.module_asset_id)?;
            slots.push(BootstrapSlotContribution {
                id: slot.id.clone(),
                slot: slot.slot.clone(),
                order: slot.order,
                module_url,
                styles,
                required_permissions: slot.required_permissions.clone(),
                r#match: slot.r#match.clone(),
            });
        }
        slots.sort_by(|left, right| {
            left.slot
                .cmp(&right.slot)
                .then(left.order.cmp(&right.order))
                .then(left.id.cmp(&right.id))
        });

        let mut field_renderers = Vec::new();
        for renderer in &state.snapshot.field_renderers {
            if !self
                .has_all_permissions(user_id, roles, &renderer.required_permissions)
                .await?
            {
                continue;
            }
            let (module_url, styles) = Self::asset_urls_for(state, &renderer.module_asset_id)?;
            field_renderers.push(BootstrapFieldRendererContribution {
                id: renderer.id.clone(),
                type_name: renderer.type_name.clone(),
                module_url,
                styles,
                required_permissions: renderer.required_permissions.clone(),
            });
        }
        field_renderers.sort_by(|left, right| {
            left.type_name
                .cmp(&right.type_name)
                .then(left.id.cmp(&right.id))
        });

        let settings = if let Some(settings) = &state.snapshot.settings {
            if self
                .has_all_permissions(user_id, roles, &settings.required_permissions)
                .await?
            {
                let custom_page = if let Some(page) = &settings.custom_page {
                    let (module_url, styles) = Self::asset_urls_for(state, &page.module_asset_id)?;
                    Some(BootstrapSettingsPage {
                        path: page.path.clone(),
                        full_path: plugin_admin_full_path(plugin_name, &page.path),
                        module_url,
                        styles,
                    })
                } else {
                    None
                };
                Some(BootstrapSettingsContribution {
                    namespace: settings.namespace.clone(),
                    required_permissions: settings.required_permissions.clone(),
                    custom_page,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(BootstrapPlugin {
            name: plugin_name.to_owned(),
            version: plugin_version.to_owned(),
            menus,
            routes,
            slots,
            field_renderers,
            settings,
        })
    }

    async fn has_all_permissions(
        &self,
        user_id: &str,
        roles: &[String],
        codes: &[String],
    ) -> Result<bool> {
        for code in codes {
            if !self
                .permission_engine
                .check_permission(user_id, roles, code, None)
                .await?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn asset_urls_for(
        state: &FrontendRuntimeState,
        asset_id: &str,
    ) -> Result<(String, Vec<String>)> {
        let asset = state
            .snapshot
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or_else(|| Error::Internal {
                message: format!(
                    "plugin {} frontend snapshot references missing asset {}",
                    state.snapshot.plugin_name, asset_id
                ),
                source: None,
            })?;

        let module_file = Self::file_for_path(state, &asset.module_path)?;
        let mut styles = Vec::with_capacity(asset.style_paths.len());
        for style_path in &asset.style_paths {
            let style_file = Self::file_for_path(state, style_path)?;
            styles.push(Self::asset_url(
                &state.snapshot.plugin_name,
                &state.snapshot.plugin_version,
                style_file,
            ));
        }

        Ok((
            Self::asset_url(
                &state.snapshot.plugin_name,
                &state.snapshot.plugin_version,
                module_file,
            ),
            styles,
        ))
    }

    fn file_for_path<'a>(
        state: &'a FrontendRuntimeState,
        path: &str,
    ) -> Result<&'a crate::frontend_snapshot::NormalizedFrontendFile> {
        state
            .snapshot
            .files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| Error::Internal {
                message: format!(
                    "plugin {} frontend snapshot references missing file {}",
                    state.snapshot.plugin_name, path
                ),
                source: None,
            })
    }

    fn asset_url(
        plugin_name: &str,
        version: &str,
        file: &crate::frontend_snapshot::NormalizedFrontendFile,
    ) -> String {
        format!(
            "/api/v1/plugin-assets/{plugin_name}/{version}/{}/{path}",
            file.url_hash,
            path = file.path
        )
    }

    async fn check_dependencies_installed(&self, manifest: &PluginManifest) -> Result<()> {
        for (dep_name, spec) in &manifest.dependencies {
            let Some(dep_record) = self.repository.find_by_name(dep_name).await? else {
                if spec.optional {
                    continue;
                }
                return Err(Error::ValidationError {
                    message: format!(
                        "plugin {} depends on missing plugin {dep_name}",
                        manifest.plugin.name
                    ),
                    details: None,
                });
            };
            let installed_ver =
                Version::parse(&dep_record.version).map_err(|e| Error::Internal {
                    message: format!("stored version {} for {dep_name}: {e}", dep_record.version),
                    source: None,
                })?;
            let req = manifest
                .parsed_dep_requirements()
                .remove(dep_name)
                .expect("dep_name came from manifest.dependencies");
            if !req.matches(&installed_ver) {
                if spec.optional {
                    continue;
                }
                return Err(Error::ValidationError {
                    message: format!(
                        "plugin {} requires {dep_name} {}, but installed version is {}",
                        manifest.plugin.name, spec.version, dep_record.version
                    ),
                    details: None,
                });
            }
        }
        Ok(())
    }

    async fn check_dependencies_enabled(&self, manifest: &PluginManifest) -> Result<()> {
        for (dep_name, spec) in &manifest.dependencies {
            if spec.optional {
                continue;
            }
            let dep_record = self
                .repository
                .find_by_name(dep_name)
                .await?
                .ok_or_else(|| Error::ValidationError {
                    message: format!(
                        "plugin {} depends on missing plugin {dep_name}",
                        manifest.plugin.name
                    ),
                    details: None,
                })?;
            if dep_record.status != PluginStatus::Enabled {
                return Err(Error::ValidationError {
                    message: format!(
                        "plugin {} requires {dep_name} to be enabled first",
                        manifest.plugin.name
                    ),
                    details: None,
                });
            }
        }
        Ok(())
    }

    async fn enabled_dependents_of(&self, plugin_name: &str) -> Result<Vec<String>> {
        let all = self.repository.list().await?;
        let manifests: Vec<PluginManifest> = all
            .into_iter()
            .filter(|r| r.status == PluginStatus::Enabled && r.name != plugin_name)
            .filter_map(|r| Self::manifest_from_record(&r).ok())
            .collect();
        Ok(reverse_dependencies(plugin_name, &manifests))
    }

    async fn run_plugin_up_migrations(&self, source: &DiscoveredPlugin) -> Result<()> {
        for rel in &source.manifest.migrations {
            let dir = source.directory.join(rel);
            if !dir.exists() {
                warn!(plugin = %source.manifest.plugin.name, path = %dir.display(), "plugin migration dir missing, skipping");
                continue;
            }
            self.migration_engine
                .run_plugin_migrations(&source.manifest.plugin.name, &dir)
                .await?;
        }
        Ok(())
    }

    async fn run_plugin_down_migrations(
        &self,
        plugin_name: &str,
        manifest: &PluginManifest,
    ) -> Result<()> {
        for rel in &manifest.migrations {
            let dir = self.plugins_root.join(plugin_name).join(rel);
            if !dir.exists() {
                warn!(plugin = %plugin_name, path = %dir.display(), "plugin migration dir missing on uninstall, skipping rollback");
                continue;
            }

            let rollback_count = self
                .migration_engine
                .applied_versions_in_dir(plugin_name, &dir)
                .await?
                .len();
            if rollback_count == 0 {
                continue;
            }

            self.migration_engine
                .rollback(plugin_name, &dir, rollback_count)
                .await?;
        }
        Ok(())
    }

    async fn rollback_failed_install(
        &self,
        source: &DiscoveredPlugin,
        permissions_registered: bool,
    ) -> Result<()> {
        let mut rollback_failures = Vec::new();

        if permissions_registered
            && let Err(err) = self
                .permission_engine
                .unregister_permissions_by_source(&source.manifest.plugin.name)
                .await
        {
            rollback_failures.push(format!("unregister permissions: {err}"));
        }

        if let Err(err) = self
            .run_plugin_down_migrations(&source.manifest.plugin.name, &source.manifest)
            .await
        {
            rollback_failures.push(format!("rollback migrations: {err}"));
        }

        if rollback_failures.is_empty() {
            info!(plugin = %source.manifest.plugin.name, "rolled back failed plugin install");
            Ok(())
        } else {
            Err(Error::Internal {
                message: format!(
                    "failed to rollback plugin install for {}: {}",
                    source.manifest.plugin.name,
                    rollback_failures.join("; ")
                ),
                source: None,
            })
        }
    }

    fn publish_event(&self, kind: EventKind, record: &PluginRecord, actor_id: Option<&str>) {
        let event = Event::new(kind).with_payload(json!({
            "name": record.name,
            "version": record.version,
            "kind": record.kind.as_str(),
        }));
        let event = if let Some(actor_id) = actor_id {
            event.with_actor(actor_id)
        } else {
            event
        };
        self.event_bus.publish(event);
    }
}

impl PluginManifest {
    /// 已校验后的依赖 version range 表。
    ///
    /// # Panics
    /// 仅当 manifest 未经过 [`PluginManifest::validate`] 时才会 panic。
    #[must_use]
    pub fn parsed_dep_requirements(&self) -> HashMap<String, semver::VersionReq> {
        self.dependencies
            .iter()
            .map(|(n, spec)| {
                (
                    n.clone(),
                    semver::VersionReq::parse(&spec.version).expect("validated"),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::PluginInfo;
    use crate::manifest::PluginKind;
    use crate::model::{PluginRecord, PluginStatus};
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn plugin_info_extracts_dependencies_and_permissions_alphabetically() {
        let manifest_value = json!({
            "plugin": {
                "name": "blog",
                "version": "0.1.0",
                "kind": "native",
                "entry": "blog.so",
            },
            "compatibility": { "cycms": ">=0.1.0" },
            "dependencies": {
                "zeta": { "version": "^0.1" },
                "auth": { "version": "^0.2" },
            },
            "permissions": {
                "definitions": [
                    { "domain": "blog", "resource": "post", "action": "create" },
                    { "domain": "blog", "resource": "post", "action": "update", "scope": "own" },
                ],
            },
        });
        let record = PluginRecord {
            id: "r1".into(),
            name: "blog".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Native,
            status: PluginStatus::Disabled,
            manifest: manifest_value,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let info = PluginInfo::from_record(&record);
        assert_eq!(info.dependencies, vec!["auth", "zeta"]);
        assert_eq!(
            info.permissions,
            vec!["blog.post.create", "blog.post.update"]
        );
    }

    #[test]
    fn plugin_info_handles_malformed_manifest_as_empty() {
        let record = PluginRecord {
            id: "r1".into(),
            name: "blog".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Native,
            status: PluginStatus::Disabled,
            manifest: json!({ "garbage": true }),
            installed_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let info = PluginInfo::from_record(&record);
        assert!(info.dependencies.is_empty());
        assert!(info.permissions.is_empty());
    }
}
