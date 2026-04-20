use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cycms_config::AppConfig;
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_kernel::{AppContext, Kernel};
use cycms_migrate::MigrationEngine;

pub(crate) fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

pub(crate) fn resolve_plugins_root(config_path: &Path, directory: &str) -> PathBuf {
    let raw = PathBuf::from(directory);
    if raw.is_absolute() {
        return raw;
    }

    config_path.parent().unwrap_or(Path::new(".")).join(raw)
}

pub(crate) fn ensure_absent(path: &Path, kind: &str) -> Result<()> {
    if path.exists() {
        return Err(Error::Conflict {
            message: format!("{kind} already exists: {}", path.display()),
        });
    }
    Ok(())
}

pub(crate) fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| Error::Internal {
        message: format!("create directory {}", path.display()),
        source: Some(Box::new(source)),
    })
}

pub(crate) fn write_text_file(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).map_err(|source| Error::Internal {
        message: format!("write file {}", path.display()),
        source: Some(Box::new(source)),
    })
}

pub(crate) fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    create_dir_all(target)?;
    for entry in fs::read_dir(source).map_err(|error| Error::Internal {
        message: format!("read directory {}", source_path_display(source)),
        source: Some(Box::new(error)),
    })? {
        let entry = entry.map_err(|error| Error::Internal {
            message: format!("iterate directory {}", source_path_display(source)),
            source: Some(Box::new(error)),
        })?;
        let source_path = entry.path();
        let file_type = entry.file_type().map_err(|source| Error::Internal {
            message: format!("inspect file type in {}", source_path.display()),
            source: Some(Box::new(source)),
        })?;
        let target_path = target.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path).map_err(|source| Error::Internal {
                message: format!(
                    "copy file {} -> {}",
                    source_path.display(),
                    target_path.display()
                ),
                source: Some(Box::new(source)),
            })?;
        } else if file_type.is_symlink() {
            return Err(Error::BadRequest {
                message: format!(
                    "symbolic links are not supported in generated or installed plugin trees: {}",
                    source_path.display()
                ),
                source: None,
            });
        }
    }
    Ok(())
}

pub(crate) fn canonicalized_eq(lhs: &Path, rhs: &Path) -> Result<bool> {
    let lhs = lhs.canonicalize().map_err(|source| Error::Internal {
        message: format!("canonicalize path {}", lhs.display()),
        source: Some(Box::new(source)),
    })?;
    let rhs = rhs.canonicalize().map_err(|source| Error::Internal {
        message: format!("canonicalize path {}", rhs.display()),
        source: Some(Box::new(source)),
    })?;
    Ok(lhs == rhs)
}

pub(crate) fn load_config(config_path: &Path) -> Result<AppConfig> {
    AppConfig::load(Some(config_path))
}

pub(crate) async fn load_config_and_db(
    config_path: &Path,
) -> Result<(AppConfig, Arc<DatabasePool>)> {
    let config = load_config(config_path)?;
    let db = Arc::new(DatabasePool::connect(&config.database).await?);
    Ok((config, db))
}

pub(crate) async fn load_migration_engine(
    config_path: &Path,
) -> Result<(AppConfig, Arc<DatabasePool>, MigrationEngine)> {
    let (config, db) = load_config_and_db(config_path).await?;
    Ok((config, Arc::clone(&db), MigrationEngine::new(db)))
}

pub(crate) async fn bootstrap_app(config_path: &Path) -> Result<AppContext> {
    let kernel = Kernel::build(Some(config_path)).await?;
    kernel.bootstrap(Some(&workspace_system_migrations())).await
}

fn source_path_display(path: &Path) -> String {
    path.display().to_string()
}
