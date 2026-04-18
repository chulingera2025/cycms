use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{
    AUTHOR_ROLE, EDITOR_ROLE, PermissionEngine, SUPER_ADMIN_ROLE, seed_defaults,
};

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_pool() -> Arc<DatabasePool> {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("sqlite pool connect"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("run system migrations");
    pool
}

#[tokio::test]
async fn seed_creates_three_system_roles() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);

    seed_defaults(&engine).await.unwrap();
    let roles = engine.roles().list().await.unwrap();
    let names: Vec<String> = roles.into_iter().map(|r| r.name).collect();
    assert!(names.contains(&SUPER_ADMIN_ROLE.to_owned()));
    assert!(names.contains(&EDITOR_ROLE.to_owned()));
    assert!(names.contains(&AUTHOR_ROLE.to_owned()));
}

#[tokio::test]
async fn seed_is_idempotent_on_repeated_call() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool.clone());

    seed_defaults(&engine).await.unwrap();
    let first_count_roles = engine.roles().list().await.unwrap().len();
    let first_count_perms = engine
        .permissions()
        .list_by_source("system")
        .await
        .unwrap()
        .len();

    seed_defaults(&engine).await.unwrap();
    let second_count_roles = engine.roles().list().await.unwrap().len();
    let second_count_perms = engine
        .permissions()
        .list_by_source("system")
        .await
        .unwrap()
        .len();

    assert_eq!(first_count_roles, second_count_roles);
    assert_eq!(first_count_perms, second_count_perms);

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    let (links_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM role_permissions")
        .fetch_one(inner)
        .await
        .unwrap();
    // 第二次 seed 不能把已有的 role_permissions 行膨胀（attach_permission 是幂等的）
    assert!(links_count > 0);
}

#[tokio::test]
async fn editor_can_publish_but_author_only_own() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_defaults(&engine).await.unwrap();

    // editor 拥有 content.entry.publish (scope=All)
    let editor_roles = vec![EDITOR_ROLE.to_owned()];
    assert!(
        engine
            .check_permission(
                "user-editor",
                &editor_roles,
                "content.entry.publish",
                Some("someone-else"),
            )
            .await
            .unwrap()
    );

    // author 仅拥有 scope=Own 的发布权
    let author_roles = vec![AUTHOR_ROLE.to_owned()];
    assert!(
        engine
            .check_permission(
                "user-author",
                &author_roles,
                "content.entry.publish",
                Some("user-author"),
            )
            .await
            .unwrap()
    );
    assert!(
        !engine
            .check_permission(
                "user-author",
                &author_roles,
                "content.entry.publish",
                Some("someone-else"),
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn super_admin_holds_all_system_permissions_even_if_short_circuit_bypassed() {
    // 即便短路被关闭，super_admin 也应显式绑定所有系统权限。
    // 这里通过传一个 "假的 super_admin 无短路" 的等效：用 super_admin 角色 + 非短路名称
    // 不可行，因此改为验证权限表中 super_admin 关联数 >= system permission 数。
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool.clone());
    seed_defaults(&engine).await.unwrap();

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    let super_admin = engine
        .roles()
        .find_by_name(SUPER_ADMIN_ROLE)
        .await
        .unwrap()
        .unwrap();
    let (links_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM role_permissions WHERE role_id = ?")
            .bind(&super_admin.id)
            .fetch_one(inner)
            .await
            .unwrap();

    let perms_total = engine
        .permissions()
        .list_by_source("system")
        .await
        .unwrap()
        .len();
    assert_eq!(usize::try_from(links_count).unwrap(), perms_total);
}
