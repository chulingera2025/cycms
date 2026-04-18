use std::fs;
use std::path::Path;

use cycms_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE: &str = "cycms.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
#[derive(Default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub media: MediaConfig,
    pub plugins: PluginsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub sensitive_requests_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
    pub max_age_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[default]
    Postgres,
    MySql,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub argon2: Argon2Config,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Argon2Config {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MediaConfig {
    pub upload_dir: String,
    pub max_file_size: u64,
    pub allowed_mime_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PluginsConfig {
    pub directory: String,
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
            jwt_secret: "CHANGE_ME_IN_PRODUCTION".to_owned(),
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
        AppConfig, DEFAULT_CONFIG_FILE, DatabaseDriver, Error, apply_env_overrides_from_iter,
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

[media]
upload_dir = "media"

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
        assert_eq!(config.media.upload_dir, "media");
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
        assert!(!config.plugins.wasm_enabled);
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
