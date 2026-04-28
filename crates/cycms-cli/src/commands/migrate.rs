use cycms_core::{Error, Result};
use cycms_migrate::SYSTEM_SOURCE;

use crate::cli::{MigrateArgs, MigrateCommand};
use crate::support::{load_migration_engine, workspace_system_migrations};

pub(crate) async fn run(args: &MigrateArgs) -> Result<()> {
    match args.command {
        MigrateCommand::Run => run_migrations(&args.config).await,
        MigrateCommand::Rollback { count } => rollback_migrations(&args.config, count).await,
        MigrateCommand::Status => run_status(&args.config).await,
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

async fn run_status(config_path: &std::path::Path) -> Result<()> {
    use cycms_migrate::scan;

    let (_, _, engine) = load_migration_engine(config_path).await?;
    let migrations_root = workspace_system_migrations();

    // Ensure meta table exists so we can query it.
    engine.ensure_meta_table().await?;

    let available = scan(&migrations_root)?;
    let applied_versions = engine.applied_versions(SYSTEM_SOURCE).await?;

    let total = available.len();
    let applied_count = applied_versions.len();
    let pending_count = total - applied_count;

    println!("System migration status:");
    println!("  Source:     {SYSTEM_SOURCE}");
    println!("  Directory:  {}", migrations_root.display());
    println!("  Available:  {total}");
    println!("  Applied:    {applied_count}");
    println!("  Pending:    {pending_count}");

    if pending_count > 0 {
        println!();
        println!("Pending migrations:");
        let applied_set: std::collections::HashSet<i64> = applied_versions.into_iter().collect();
        for migration in &available {
            if !applied_set.contains(&migration.version) {
                let has_down = if migration.down_sql.is_some() {
                    " (has down)"
                } else {
                    ""
                };
                println!("  - {}_{}{has_down}", migration.version, migration.name,);
            }
        }
    }

    Ok(())
}
