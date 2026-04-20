use cycms_core::{Error, Result};
use cycms_migrate::SYSTEM_SOURCE;

use crate::cli::{MigrateArgs, MigrateCommand};
use crate::support::{load_migration_engine, workspace_system_migrations};

pub(crate) async fn run(args: &MigrateArgs) -> Result<()> {
    match args.command {
        MigrateCommand::Run => run_migrations(&args.config).await,
        MigrateCommand::Rollback { count } => rollback_migrations(&args.config, count).await,
    }
}

async fn run_migrations(config_path: &std::path::Path) -> Result<()> {
    let (_, _, engine) = load_migration_engine(config_path).await?;
    let records = engine
        .run_system_migrations(&workspace_system_migrations())
        .await?;

    if records.is_empty() {
        println!("No pending system migrations.");
        return Ok(());
    }

    for record in records {
        println!(
            "applied {} {} [{}] in {}ms",
            record.version,
            record.name,
            record.status.as_str(),
            record.execution_time_ms
        );
    }

    Ok(())
}

async fn rollback_migrations(config_path: &std::path::Path, count: usize) -> Result<()> {
    if count == 0 {
        return Err(Error::BadRequest {
            message: "rollback count must be at least 1".to_owned(),
            source: None,
        });
    }

    let (_, _, engine) = load_migration_engine(config_path).await?;
    let records = engine
        .rollback(SYSTEM_SOURCE, &workspace_system_migrations(), count)
        .await?;

    if records.is_empty() {
        println!("No applied system migrations to roll back.");
        return Ok(());
    }

    for record in records {
        println!(
            "rolled back {} {} [{}]",
            record.version,
            record.name,
            record.status.as_str()
        );
    }

    Ok(())
}
