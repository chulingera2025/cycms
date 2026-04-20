use rand::Rng;
use rand::distributions::Alphanumeric;
use std::sync::Arc;

use cycms_auth::{AuthEngine, CreateUserInput};
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry, seed_default_types};
use cycms_core::{Error, Result};
use cycms_permission::{PermissionEngine, SUPER_ADMIN_ROLE, seed_defaults};

use crate::cli::SeedArgs;
use crate::support::{load_migration_engine, workspace_system_migrations};

pub(crate) async fn run(args: &SeedArgs) -> Result<()> {
    let (config, db, engine) = load_migration_engine(&args.config).await?;
    let _ = engine
        .run_system_migrations(&workspace_system_migrations())
        .await?;

    let auth_engine = AuthEngine::new(Arc::clone(&db), config.auth.clone())?;
    let permission_engine = PermissionEngine::new(Arc::clone(&db));
    let content_model =
        ContentModelRegistry::new(Arc::clone(&db), Arc::new(FieldTypeRegistry::new()));

    seed_defaults(&permission_engine).await?;

    let existing_users = auth_engine.users().count().await?;
    if existing_users == 0 {
        let password = args
            .password
            .clone()
            .unwrap_or_else(generate_admin_password);
        let user = auth_engine
            .setup_admin(CreateUserInput {
                username: args.username.clone(),
                email: args.email.clone(),
                password: password.clone(),
            })
            .await?;

        let super_admin = permission_engine
            .roles()
            .find_by_name(SUPER_ADMIN_ROLE)
            .await?
            .ok_or_else(|| Error::Internal {
                message: format!("default role not found after seed: {SUPER_ADMIN_ROLE}"),
                source: None,
            })?;
        permission_engine
            .roles()
            .bind_user(&user.id, &super_admin.id)
            .await?;

        println!(
            "Seeded initial administrator {} <{}> with super_admin role.",
            user.username, user.email
        );
        if args.password.is_none() {
            println!("Generated admin password: {password}");
        }
    } else {
        println!(
            "Skipped admin seed because the system already contains {existing_users} user(s)."
        );
    }

    let seeded_types = seed_default_types(&content_model).await?;
    println!(
        "Seeded default roles and ensured {} content type(s).",
        seeded_types.len()
    );
    Ok(())
}

fn generate_admin_password() -> String {
    let random: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(18)
        .map(char::from)
        .collect();
    format!("Seeded{random}9!")
}
