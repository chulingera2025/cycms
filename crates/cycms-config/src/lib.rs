use std::fs;
use std::path::Path;

use cycms_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE: &str = "cycms.toml";

/// JWT 密钥占位符：出现在默认配置中用于提示未覆盖的生产部署。
pub const JWT_SECRET_PLACEHOLDER: &str = "CHANGE_ME_IN_PRODUCTION";

/// HS256 签名推荐的最小密钥字节数。
pub const MIN_JWT_SECRET_BYTES: usize = 32;

/// 应用根配置，对应 `cycms.toml` 的顶层结构。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
#[derive(Default)]
pub struct AppConfig {
    /// HTTP 服务监听、限流与 CORS 配置。
    pub server: ServerConfig,
    /// 数据库连接池与驱动配置。
    pub database: DatabaseConfig,
    /// 认证、JWT 与 Argon2 参数。
    pub auth: AuthConfig,
    /// 内容引擎的默认删除策略与分页参数。
    pub content: ContentConfig,
    /// 媒体上传与删除策略配置。
    pub media: MediaConfig,
    /// 进程内事件总线配置。
    pub events: EventsConfig,
    /// tracing 与审计日志配置。
    pub observability: ObservabilityConfig,
    /// 插件目录与 Wasm 开关配置。
    pub plugins: PluginsConfig,
}

/// HTTP 服务端口、限流与 CORS 相关配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ServerConfig {
    /// 监听地址。
    pub host: String,
    /// 监听端口。
    pub port: u16,
    /// 通用与敏感接口限流配额。
    pub rate_limit: RateLimitConfig,
    /// 跨域配置。
    pub cors: CorsConfig,
}

/// HTTP 限流参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RateLimitConfig {
    /// 普通接口每分钟最大请求数。
    pub requests_per_minute: u32,
    /// 登录、注册、刷新等敏感接口每分钟最大请求数。
    pub sensitive_requests_per_minute: u32,
}

/// 跨域资源共享配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CorsConfig {
    /// 允许的来源列表。
    pub allowed_origins: Vec<String>,
    /// 是否允许携带凭证。
    pub allow_credentials: bool,
    /// 预检结果缓存秒数。
    pub max_age_secs: u64,
}

/// 数据库驱动与连接池配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DatabaseConfig {
    /// 底层数据库驱动。
    pub driver: DatabaseDriver,
    /// 连接字符串。
    pub url: String,
    /// 最大连接数。
    pub max_connections: u32,
    /// 连接获取超时秒数。
    pub connect_timeout_secs: u64,
    /// 连接空闲超时秒数。
    pub idle_timeout_secs: u64,
}

/// 支持的数据库驱动类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[default]
    Postgres,
    MySql,
    Sqlite,
}

/// 认证与密码学参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    /// JWT 对称签名密钥。
    pub jwt_secret: String,
    /// access token 生命周期，单位秒。
    pub access_token_ttl_secs: u64,
    /// refresh token 生命周期，单位秒。
    pub refresh_token_ttl_secs: u64,
    /// Argon2id 参数。
    pub argon2: Argon2Config,
}

/// Argon2id 参数配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Argon2Config {
    /// 内存成本参数。
    pub m_cost: u32,
    /// 时间成本参数。
    pub t_cost: u32,
    /// 并行度参数。
    pub p_cost: u32,
}

/// 内容模块的默认行为配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContentConfig {
    /// 未显式指定时的删除模式。
    pub default_delete_mode: DeleteMode,
    /// 默认分页大小。
    pub default_page_size: u64,
    /// 分页大小上限。
    pub max_page_size: u64,
}

/// 内容删除策略。`Soft` 将实例状态标记为 `archived` 保留数据，`Hard` 直接物理删除。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    #[default]
    Soft,
    Hard,
}

/// 媒体模块配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MediaConfig {
    /// 上传文件保存目录。
    pub upload_dir: String,
    /// 允许的最大文件大小，单位字节。
    pub max_file_size: u64,
    /// 白名单 MIME 列表；空列表表示不限制。
    pub allowed_mime_types: Vec<String>,
    /// 删除有引用的媒体资产时的行为：`"block"` 返回错误，`"warn"` 仅记录警告并继续删除。
    pub on_referenced_delete: String,
}

/// 进程内事件总线配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct EventsConfig {
    /// 每个 `EventKind` 对应 broadcast channel 的容量。
    pub channel_capacity: usize,
    /// 单个 handler 处理一次事件的超时秒数。
    pub handler_timeout_secs: u64,
}

/// tracing 与审计日志配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ObservabilityConfig {
    /// 日志输出格式。
    pub format: LogFormat,
    /// tracing 过滤级别。
    pub level: String,
    /// 是否写入审计日志。
    pub audit_enabled: bool,
}

/// tracing 日志格式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    #[default]
    Pretty,
}

/// 插件系统配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PluginsConfig {
    /// 插件根目录。
    pub directory: String,
    /// 是否启用 Wasm 插件运行时。
    pub wasm_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_owned(),
            port: 8080,
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            sensitive_requests_per_minute: 10,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_owned()],
            allow_credentials: true,
            max_age_secs: 600,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            driver: DatabaseDriver::default(),
            url: "postgres://cycms:cycms@localhost:5432/cycms".to_owned(),
            max_connections: 10,
            connect_timeout_secs: 10,
            idle_timeout_secs: 300,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: JWT_SECRET_PLACEHOLDER.to_owned(),
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 1_209_600,
            argon2: Argon2Config::default(),
        }
    }
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            m_cost: 19_456,
            t_cost: 2,
            p_cost: 1,
        }
    }
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            default_delete_mode: DeleteMode::Soft,
            default_page_size: 20,
            max_page_size: 100,
        }
    }
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            upload_dir: "uploads".to_owned(),
            max_file_size: 10 * 1024 * 1024,
            allowed_mime_types: vec![
                "image/jpeg".to_owned(),
                "image/png".to_owned(),
                "image/gif".to_owned(),
                "image/webp".to_owned(),
                "application/pdf".to_owned(),
                "video/mp4".to_owned(),
            ],
            on_referenced_delete: "block".to_owned(),
        }
    }
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 256,
            handler_timeout_secs: 5,
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            level: "info".to_owned(),
            audit_enabled: true,
        }
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            directory: "plugins".to_owned(),
            wasm_enabled: true,
        }
    }
}

impl AppConfig {
    /// 从 cycms.toml 和环境变量加载配置。
    ///
    /// # Errors
    /// 读取配置文件失败、TOML 解析失败或环境变量覆盖值类型不匹配时返回错误。
    pub fn load(path: Option<&Path>) -> Result<Self> {
        Self::load_from_path_and_env(path, std::env::vars())
    }

    fn load_from_path_and_env<I, K, V>(path: Option<&Path>, env_vars: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let resolved_path = path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE));
        let mut config = match (path.is_some(), resolved_path.exists()) {
            (_, true) => load_config_file(resolved_path)?,
            (true, false) => {
                return Err(Error::NotFound {
                    message: format!("configuration file not found: {}", resolved_path.display()),
                });
            }
            (false, false) => Self::default(),
        };

        apply_env_overrides_from_iter(&mut config, env_vars)?;
        Ok(config)
    }
}

fn load_config_file(path: &Path) -> Result<AppConfig> {
    let contents = fs::read_to_string(path).map_err(|source| Error::BadRequest {
        message: format!("failed to read configuration file: {}", path.display()),
        source: Some(Box::new(source)),
    })?;

    toml::from_str(&contents).map_err(|source| Error::BadRequest {
        message: format!("failed to parse configuration file: {}", path.display()),
        source: Some(Box::new(source)),
    })
}

fn apply_env_overrides_from_iter<I, K, V>(config: &mut AppConfig, env_vars: I) -> Result<()>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut config_value =
        toml::Value::try_from(config.clone()).map_err(|source| Error::Internal {
            message: "failed to serialize configuration for environment overrides".to_owned(),
            source: Some(Box::new(source)),
        })?;

    let mut entries = env_vars
        .into_iter()
        .filter_map(|(key, value)| {
            let key = key.as_ref();
            key.strip_prefix("CYCMS__")
                .map(|suffix| (suffix.to_owned(), value.as_ref().to_owned()))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    for (suffix, raw_value) in entries {
        let path_segments = suffix
            .split("__")
            .filter(|segment| !segment.is_empty())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        if path_segments.is_empty() {
            continue;
        }

        let Some(target) = find_value_mut(&mut config_value, &path_segments) else {
            continue;
        };
        let replacement = parse_override_value(target, &raw_value)?;
        *target = replacement;
    }

    *config = config_value
        .try_into()
        .map_err(|source| Error::BadRequest {
            message: "failed to deserialize configuration after applying environment overrides"
                .to_owned(),
            source: Some(Box::new(source)),
        })?;

    Ok(())
}

fn find_value_mut<'a>(
    value: &'a mut toml::Value,
    path_segments: &[String],
) -> Option<&'a mut toml::Value> {
    if path_segments.is_empty() {
        return Some(value);
    }

    match value {
        toml::Value::Table(table) => {
            let next = table.get_mut(path_segments.first()?)?;
            find_value_mut(next, &path_segments[1..])
        }
        _ => None,
    }
}

fn parse_override_value(target: &toml::Value, raw_value: &str) -> Result<toml::Value> {
    match target {
        toml::Value::String(_) => Ok(toml::Value::String(raw_value.to_owned())),
        toml::Value::Integer(_) => parse_typed_toml_value(target, raw_value, "integer"),
        toml::Value::Float(_) => parse_typed_toml_value(target, raw_value, "float"),
        toml::Value::Boolean(_) => parse_typed_toml_value(target, raw_value, "boolean"),
        toml::Value::Datetime(_) => parse_typed_toml_value(target, raw_value, "datetime"),
        toml::Value::Array(_) => parse_typed_toml_value(target, raw_value, "array"),
        toml::Value::Table(_) => parse_typed_toml_value(target, raw_value, "table"),
    }
}

fn parse_typed_toml_value(
    target: &toml::Value,
    raw_value: &str,
    expected_type: &str,
) -> Result<toml::Value> {
    let parsed = parse_toml_value(raw_value)?;
    let is_matching_type = matches!(
        (target, &parsed),
        (toml::Value::Integer(_), toml::Value::Integer(_))
            | (toml::Value::Float(_), toml::Value::Float(_))
            | (toml::Value::Boolean(_), toml::Value::Boolean(_))
            | (toml::Value::Datetime(_), toml::Value::Datetime(_))
            | (toml::Value::Array(_), toml::Value::Array(_))
            | (toml::Value::Table(_), toml::Value::Table(_))
    );

    if is_matching_type {
        Ok(parsed)
    } else {
        Err(Error::BadRequest {
            message: format!(
                "invalid environment override type: expected {expected_type}, got {}",
                value_kind(&parsed)
            ),
            source: None,
        })
    }
}

fn parse_toml_value(raw_value: &str) -> Result<toml::Value> {
    let wrapped = format!("value = {raw_value}");
    let table = wrapped
        .parse::<toml::Table>()
        .map_err(|source| Error::BadRequest {
            message: format!("failed to parse environment override value: {raw_value}"),
            source: Some(Box::new(source)),
        })?;

    table.get("value").cloned().ok_or_else(|| Error::Internal {
        message: "parsed environment override is missing wrapped value".to_owned(),
        source: None,
    })
}

fn value_kind(value: &toml::Value) -> &'static str {
    match value {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "integer",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "boolean",
        toml::Value::Datetime(_) => "datetime",
        toml::Value::Array(_) => "array",
        toml::Value::Table(_) => "table",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use super::{
        AppConfig, DEFAULT_CONFIG_FILE, DatabaseDriver, DeleteMode, Error, LogFormat,
        apply_env_overrides_from_iter,
    };

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn loads_config_from_toml_file() {
        let temp_dir = TempDir::new();
        let config_path = temp_dir.path().join("custom.toml");
        fs::write(
            &config_path,
            r#"
[server]
port = 9090

[database]
driver = "sqlite"
url = "sqlite://cycms.db"

[auth]
jwt_secret = "test-secret"

[content]
default_delete_mode = "hard"
default_page_size = 50
max_page_size = 200

[media]
upload_dir = "media"

[events]
channel_capacity = 1024
handler_timeout_secs = 3

[observability]
format = "json"
level = "debug"
audit_enabled = false

[plugins]
wasm_enabled = false
"#,
        )
        .unwrap();

        let config = AppConfig::load_from_path_and_env(
            Some(&config_path),
            std::iter::empty::<(&str, &str)>(),
        )
        .unwrap();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
        assert_eq!(config.database.url, "sqlite://cycms.db");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.auth.jwt_secret, "test-secret");
        assert_eq!(config.content.default_delete_mode, DeleteMode::Hard);
        assert_eq!(config.content.default_page_size, 50);
        assert_eq!(config.content.max_page_size, 200);
        assert_eq!(config.media.upload_dir, "media");
        assert_eq!(config.events.channel_capacity, 1024);
        assert_eq!(config.events.handler_timeout_secs, 3);
        assert_eq!(config.observability.format, LogFormat::Json);
        assert_eq!(config.observability.level, "debug");
        assert!(!config.observability.audit_enabled);
        assert!(!config.plugins.wasm_enabled);
    }

    #[test]
    fn env_overrides_replace_scalar_values() {
        let mut config = AppConfig::default();

        apply_env_overrides_from_iter(
            &mut config,
            [
                ("CYCMS__SERVER__PORT", "9091"),
                ("CYCMS__DATABASE__DRIVER", "mysql"),
                (
                    "CYCMS__DATABASE__URL",
                    "mysql://root:root@localhost:3306/cycms",
                ),
                ("CYCMS__AUTH__JWT_SECRET", "override-secret"),
            ],
        )
        .unwrap();

        assert_eq!(config.server.port, 9091);
        assert_eq!(config.database.driver, DatabaseDriver::MySql);
        assert_eq!(
            config.database.url,
            "mysql://root:root@localhost:3306/cycms"
        );
        assert_eq!(config.auth.jwt_secret, "override-secret");
    }

    #[test]
    fn env_overrides_replace_arrays_and_booleans() {
        let mut config = AppConfig::default();

        apply_env_overrides_from_iter(
            &mut config,
            [
                (
                    "CYCMS__SERVER__CORS__ALLOWED_ORIGINS",
                    r#"["https://admin.example.com","https://cms.example.com"]"#,
                ),
                ("CYCMS__EVENTS__CHANNEL_CAPACITY", "512"),
                ("CYCMS__EVENTS__HANDLER_TIMEOUT_SECS", "2"),
                ("CYCMS__OBSERVABILITY__AUDIT_ENABLED", "false"),
                ("CYCMS__PLUGINS__WASM_ENABLED", "false"),
            ],
        )
        .unwrap();

        assert_eq!(
            config.server.cors.allowed_origins,
            vec![
                "https://admin.example.com".to_owned(),
                "https://cms.example.com".to_owned(),
            ]
        );
        assert_eq!(config.events.channel_capacity, 512);
        assert_eq!(config.events.handler_timeout_secs, 2);
        assert!(!config.observability.audit_enabled);
        assert!(!config.plugins.wasm_enabled);
    }

    #[test]
    fn events_defaults_match_runtime_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.events.channel_capacity, 256);
        assert_eq!(config.events.handler_timeout_secs, 5);
    }

    #[test]
    fn observability_defaults_to_pretty_info_with_audit_enabled() {
        let config = AppConfig::default();
        assert_eq!(config.observability.format, LogFormat::Pretty);
        assert_eq!(config.observability.level, "info");
        assert!(config.observability.audit_enabled);
    }

    #[test]
    fn content_config_defaults_to_soft_delete_and_safe_page_size() {
        let config = AppConfig::default();
        assert_eq!(config.content.default_delete_mode, DeleteMode::Soft);
        assert_eq!(config.content.default_page_size, 20);
        assert_eq!(config.content.max_page_size, 100);
    }

    #[test]
    fn env_overrides_content_delete_mode_and_page_caps() {
        let mut config = AppConfig::default();

        apply_env_overrides_from_iter(
            &mut config,
            [
                ("CYCMS__CONTENT__DEFAULT_DELETE_MODE", "hard"),
                ("CYCMS__CONTENT__DEFAULT_PAGE_SIZE", "10"),
                ("CYCMS__CONTENT__MAX_PAGE_SIZE", "200"),
            ],
        )
        .unwrap();

        assert_eq!(config.content.default_delete_mode, DeleteMode::Hard);
        assert_eq!(config.content.default_page_size, 10);
        assert_eq!(config.content.max_page_size, 200);
    }

    #[test]
    fn load_with_missing_explicit_path_returns_not_found() {
        let temp_dir = TempDir::new();
        let missing_path = temp_dir.path().join("missing.toml");

        let error = AppConfig::load_from_path_and_env(
            Some(&missing_path),
            std::iter::empty::<(&str, &str)>(),
        )
        .unwrap_err();

        assert!(matches!(error, Error::NotFound { .. }));
    }

    #[test]
    fn load_without_default_file_falls_back_to_defaults() {
        let _guard = TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let temp_dir = TempDir::new();
        let current_dir_guard = CurrentDirGuard::enter(temp_dir.path());

        let config =
            AppConfig::load_from_path_and_env(None, std::iter::empty::<(&str, &str)>()).unwrap();

        assert_eq!(config, AppConfig::default());
        assert!(!temp_dir.path().join(DEFAULT_CONFIG_FILE).exists());

        drop(current_dir_guard);
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let unique_id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("cycms-config-tests-{}-{unique_id}", process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    struct CurrentDirGuard {
        original_dir: PathBuf,
    }

    impl CurrentDirGuard {
        fn enter(next_dir: &Path) -> Self {
            let original_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(next_dir).unwrap();
            Self { original_dir }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }
}
