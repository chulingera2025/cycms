//! [`PluginManager`] 服务主体：编排安装 / 启用 / 禁用 / 卸载的生命周期状态机。
//!
//! 关键设计：
//! - Manager 持有 runtime trait 对象映射（`PluginKind -> Arc<dyn PluginRuntime>`），
//!   runtime 自身由 `cycms-plugin-native` / `cycms-plugin-wasm` 实现并在 Kernel
//!   bootstrap 时注入。无 runtime 时 enable 返回明确错误，install / list / uninstall
//!   仍可运行（数据面不依赖 runtime）。
//! - 插件目录约定 `<plugins_root>/<plugin_name>/`，manifest 在目录内的 `plugin.toml`，
//!   `plugin.entry` 是相对插件目录的实现文件路径；enable 时按此约定重建绝对路径。
//! - v0.1 的 install 不做数据库事务：migration / permission 注册 / 行插入按顺序推进，
//!   任一步失败直接返回错误，依赖调用方显式 uninstall 清理。
//!   TODO!!! 任务后续（v0.2）引入 saga 模式实现失败自动回滚。

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
use crate::manifest::{PluginKind, PluginManifest};
use crate::model::{NewPluginRow, PluginRecord, PluginStatus};
use crate::repository::PluginRepository;
use crate::resolver::{check_host_compatibility, reverse_dependencies, topological_order};
use crate::runtime::PluginRuntime;

/// `PluginManager` 构造入参，封装 Kernel 侧的不可变配置。
pub struct PluginManagerConfig {
    /// 当前宿主 `cycms` 版本，供 Req 20.2 兼容性校验。
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

    /// 注册或替换一个运行时（按 `kind` 去重）。Kernel 在任务 16 / 17 完成后调用此方法
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

        self.run_plugin_up_migrations(source).await?;

        let defs = manifest.permission_definitions();
        if !defs.is_empty() {
            self.permission_engine
                .register_permissions(&manifest.plugin.name, defs)
                .await?;
        }

        let manifest_value = serde_json::to_value(manifest).map_err(|e| Error::Internal {
            message: format!("serialize manifest: {e}"),
            source: None,
        })?;
        let row = NewPluginRow {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            kind: manifest.plugin.kind,
            manifest: manifest_value,
        };
        let record = self.repository.insert(row).await?;
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

    async fn activate_record(
        &self,
        record: &PluginRecord,
        persist_status: bool,
        emit_event: bool,
        actor_id: Option<&str>,
    ) -> Result<()> {
        let manifest = Self::manifest_from_record(record)?;
        self.check_dependencies_enabled(&manifest).await?;

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
        // 传大 count 让 MigrationEngine 回滚该 source 下所有已 applied 的迁移。
        // v0.1 单插件迁移数不会超过此上限；v0.2 引入按 source 计数接口后替换。
        const ROLLBACK_SENTINEL: usize = 10_000;

        for rel in &manifest.migrations {
            let dir = self.plugins_root.join(plugin_name).join(rel);
            if !dir.exists() {
                warn!(plugin = %plugin_name, path = %dir.display(), "plugin migration dir missing on uninstall, skipping rollback");
                continue;
            }
            self.migration_engine
                .rollback(plugin_name, &dir, ROLLBACK_SENTINEL)
                .await?;
        }
        Ok(())
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
