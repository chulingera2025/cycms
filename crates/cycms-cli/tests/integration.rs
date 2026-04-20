use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use clap::Parser;
use cycms_cli::{Cli, run};
use cycms_kernel::Kernel;
use cycms_permission::SUPER_ADMIN_ROLE;
use cycms_plugin_manager::{PluginManifest, PluginStatus};
use sqlx::Row;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

fn wasm_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../cycms-plugin-wasm/tests/fixtures/hello_plugin.wasm")
}

fn sqlite_url(path: &Path) -> String {
    format!("sqlite://{}", path.display())
}

fn write_config(root: &Path, port: Option<u16>) -> PathBuf {
    let config_path = root.join("cycms.toml");
    let db_path = root.join("cycms.db");
    let uploads_dir = root.join("uploads");
    let plugins_dir = root.join("plugins-runtime");
    fs::create_dir_all(&uploads_dir).unwrap();
    fs::create_dir_all(&plugins_dir).unwrap();

    let server_section = port.map_or_else(String::new, |port| {
        format!("\n[server]\nhost = \"127.0.0.1\"\nport = {port}\nconnect_timeout_secs = 5\n")
    });

    fs::write(
        &config_path,
        format!(
            r#"[database]
driver = "sqlite"
url = "{}"
max_connections = 5
connect_timeout_secs = 5
idle_timeout_secs = 60

[media]
upload_dir = "{}"

[plugins]
directory = "{}"
wasm_enabled = true

[observability]
audit_enabled = false
{}"#,
            sqlite_url(&db_path),
            uploads_dir.display(),
            plugins_dir.display(),
            server_section,
        ),
    )
    .unwrap();

    config_path
}

fn write_wasm_plugin_source(root: &Path, name: &str) -> PathBuf {
    let plugin_dir = root.join(name);
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("plugin.toml"),
        format!(
            r#"migrations = []

[plugin]
name = "{name}"
version = "0.1.0"
kind = "wasm"
entry = "hello_plugin.wasm"

[compatibility]
cycms = ">=0.1.0"
"#
        ),
    )
    .unwrap();
    fs::copy(wasm_fixture(), plugin_dir.join("hello_plugin.wasm")).unwrap();
    plugin_dir
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn wait_for_server(port: u16) {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("server did not start on port {port}");
}

async fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8(buf).unwrap()
}

#[tokio::test]
async fn new_command_generates_project_skeleton_with_example_plugin() {
    let temp = tempdir().unwrap();
    let project_root = temp.path().join("demo-site");

    run(Cli::parse_from([
        "cycms",
        "new",
        project_root.to_str().unwrap(),
    ]))
    .await
    .unwrap();

    assert!(project_root.join("Cargo.toml").exists());
    assert!(project_root.join("cycms.toml").exists());
    assert!(project_root.join("migrations/sqlite").is_dir());

    let manifest =
        fs::read_to_string(project_root.join("plugins/example-plugin/plugin.toml")).unwrap();
    PluginManifest::from_toml_str(&manifest).unwrap();
    assert!(
        project_root
            .join("plugins/example-plugin/src/lib.rs")
            .exists()
    );
}

#[tokio::test]
async fn plugin_new_command_generates_valid_native_plugin_scaffold() {
    let temp = tempdir().unwrap();
    let plugin_root = temp.path().join("hello-native");

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "new",
        plugin_root.to_str().unwrap(),
    ]))
    .await
    .unwrap();

    let manifest = fs::read_to_string(plugin_root.join("plugin.toml")).unwrap();
    PluginManifest::from_toml_str(&manifest).unwrap();
    let lib_rs = fs::read_to_string(plugin_root.join("src/lib.rs")).unwrap();
    assert!(lib_rs.contains("cycms_plugin_api::export_plugin!"));
    assert!(plugin_root.join("migrations/postgres").is_dir());
}

#[tokio::test]
async fn migrate_run_creates_system_tables() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), None);

    run(Cli::parse_from([
        "cycms",
        "migrate",
        "--config",
        config_path.to_str().unwrap(),
        "run",
    ]))
    .await
    .unwrap();

    let pool = sqlx::SqlitePool::connect(&sqlite_url(&temp.path().join("cycms.db")))
        .await
        .unwrap();
    let row = sqlx::query(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type='table' AND name='users'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let count: i64 = row.try_get("count").unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn seed_command_creates_admin_roles_and_default_content_types() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), None);

    run(Cli::parse_from([
        "cycms",
        "seed",
        "--config",
        config_path.to_str().unwrap(),
        "--email",
        "admin@example.test",
        "--password",
        "StrongPass1!",
    ]))
    .await
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel
        .bootstrap(Some(&workspace_system_migrations()))
        .await
        .unwrap();

    assert_eq!(ctx.auth_engine.users().count().await.unwrap(), 1);
    let admin = ctx
        .auth_engine
        .users()
        .find_by_username("admin")
        .await
        .unwrap()
        .unwrap();
    let roles = ctx
        .permission_engine
        .roles()
        .list_by_user_id(&admin.id)
        .await
        .unwrap();
    assert!(roles.iter().any(|role| role.name == SUPER_ADMIN_ROLE));
    assert!(ctx.content_model.get_type("page").await.unwrap().is_some());
    assert!(ctx.content_model.get_type("post").await.unwrap().is_some());
}

#[tokio::test]
async fn plugin_commands_install_enable_disable_and_remove_wasm_plugin() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let source_dir = write_wasm_plugin_source(temp.path(), "hello");

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "install",
        "--config",
        config_path.to_str().unwrap(),
        source_dir.to_str().unwrap(),
    ]))
    .await
    .unwrap();

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "list",
        "--config",
        config_path.to_str().unwrap(),
    ]))
    .await
    .unwrap();

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "enable",
        "--config",
        config_path.to_str().unwrap(),
        "hello",
    ]))
    .await
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel
        .bootstrap(Some(&workspace_system_migrations()))
        .await
        .unwrap();
    let installed = ctx.plugin_manager.list().await.unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].status, PluginStatus::Enabled);
    let enabled_value = ctx.settings_manager.get("hello", "enabled").await.unwrap();
    assert_eq!(
        enabled_value.as_ref().and_then(|value| value.as_str()),
        Some("true")
    );

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "disable",
        "--config",
        config_path.to_str().unwrap(),
        "hello",
        "--force",
    ]))
    .await
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel
        .bootstrap(Some(&workspace_system_migrations()))
        .await
        .unwrap();
    let installed = ctx.plugin_manager.list().await.unwrap();
    assert_eq!(installed[0].status, PluginStatus::Disabled);
    let disabled_value = ctx.settings_manager.get("hello", "enabled").await.unwrap();
    assert_eq!(
        disabled_value.as_ref().and_then(|value| value.as_str()),
        Some("false")
    );

    run(Cli::parse_from([
        "cycms",
        "plugin",
        "remove",
        "--config",
        config_path.to_str().unwrap(),
        "hello",
    ]))
    .await
    .unwrap();

    let installed_root = temp.path().join("plugins-runtime/hello");
    assert!(!installed_root.exists());
}

#[tokio::test]
async fn serve_command_starts_http_server() {
    let temp = tempdir().unwrap();
    let port = free_port();
    let config_path = write_config(temp.path(), Some(port));

    let serve_future = run(Cli::parse_from([
        "cycms",
        "serve",
        "--config",
        config_path.to_str().unwrap(),
    ]));
    tokio::pin!(serve_future);

    let response = tokio::select! {
        result = &mut serve_future => panic!("serve exited early: {result:?}"),
        response = async {
            wait_for_server(port).await;
            http_get(port, "/api/docs").await
        } => response,
    };

    assert!(response.contains("200 OK"));
    assert!(response.contains("\"openapi\":\"3.1.0\""));
}
