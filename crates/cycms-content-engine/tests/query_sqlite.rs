use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_engine::{
    ColumnField, ContentEntryRepository, ContentQuery, ContentStatus, FieldRef, FilterOperator,
    FilterSpec, NewContentEntryRow, SortDir, SortSpec, new_content_entry_id,
};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use serde_json::{Value, json};
use sqlx::SqlitePool;

const TYPE_ARTICLE: &str = "00000000-0000-0000-0000-0000000000aa";
const TYPE_PAGE: &str = "00000000-0000-0000-0000-0000000000bb";
const USER_AUTHOR: &str = "00000000-0000-0000-0000-000000000001";
const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 100;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_pool() -> Arc<DatabasePool> {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("sqlite pool"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("migrations");
    pool
}

async fn seed_user(pool: &SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, 'hashed')",
    )
    .bind(id)
    .bind(id)
    .bind(format!("{id}@example.com"))
    .execute(pool)
    .await
    .expect("seed user");
}

async fn seed_type(pool: &SqlitePool, id: &str, api_id: &str, kind: &str) {
    sqlx::query("INSERT INTO content_types (id, name, api_id, kind) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(api_id)
        .bind(api_id)
        .bind(kind)
        .execute(pool)
        .await
        .expect("seed type");
}

async fn seed_entry(
    repo: &ContentEntryRepository,
    type_id: &str,
    slug: Option<&str>,
    status: ContentStatus,
    fields: Value,
) -> String {
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: type_id.to_owned(),
        slug: slug.map(str::to_owned),
        status,
        fields,
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .expect("insert entry");
    id
}

async fn prepare() -> (Arc<DatabasePool>, ContentEntryRepository) {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite");
    };
    seed_user(inner, USER_AUTHOR).await;
    seed_type(inner, TYPE_ARTICLE, "article", "collection").await;
    seed_type(inner, TYPE_PAGE, "page", "single").await;
    let repo = ContentEntryRepository::new(Arc::clone(&pool));
    (pool, repo)
}

#[tokio::test]
async fn list_isolates_by_content_type() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("a-1"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_PAGE,
        Some("p-1"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;

    let res = repo
        .list(
            TYPE_ARTICLE,
            &ContentQuery::default(),
            DEFAULT_PAGE_SIZE,
            MAX_PAGE_SIZE,
        )
        .await
        .unwrap();
    assert_eq!(res.total, 1);
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].slug.as_deref(), Some("a-1"));
}

#[tokio::test]
async fn filter_eq_status_and_in_status() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("d1"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("d2"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("a1"),
        ContentStatus::Archived,
        json!({}),
    )
    .await;

    let q_status = ContentQuery {
        status: Some(ContentStatus::Draft),
        ..ContentQuery::default()
    };
    let res = repo
        .list(TYPE_ARTICLE, &q_status, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res.total, 2);

    let q_in = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Status),
            op: FilterOperator::In,
            value: json!(["draft", "archived"]),
        }],
        ..ContentQuery::default()
    };
    let res_in = repo
        .list(TYPE_ARTICLE, &q_in, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res_in.total, 3);
}

#[tokio::test]
async fn filter_null_and_not_null_on_slug() {
    let (_pool, repo) = prepare().await;
    seed_entry(&repo, TYPE_ARTICLE, None, ContentStatus::Draft, json!({})).await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("with-slug"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;

    let q_null = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op: FilterOperator::Null,
            value: Value::Null,
        }],
        ..ContentQuery::default()
    };
    let null_res = repo
        .list(TYPE_ARTICLE, &q_null, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(null_res.total, 1);
    assert!(null_res.entries[0].slug.is_none());

    let q_not_null = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op: FilterOperator::NotNull,
            value: Value::Null,
        }],
        ..ContentQuery::default()
    };
    let nn_res = repo
        .list(TYPE_ARTICLE, &q_not_null, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(nn_res.total, 1);
    assert_eq!(nn_res.entries[0].slug.as_deref(), Some("with-slug"));
}

#[tokio::test]
async fn filter_contains_starts_ends_on_slug() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("hello-world"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("world-of-rust"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("rust-lang"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;

    let make = |op| ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op,
            value: json!("rust"),
        }],
        ..ContentQuery::default()
    };

    let contains = repo
        .list(
            TYPE_ARTICLE,
            &make(FilterOperator::Contains),
            DEFAULT_PAGE_SIZE,
            MAX_PAGE_SIZE,
        )
        .await
        .unwrap();
    assert_eq!(contains.total, 2);

    let starts = repo
        .list(
            TYPE_ARTICLE,
            &make(FilterOperator::StartsWith),
            DEFAULT_PAGE_SIZE,
            MAX_PAGE_SIZE,
        )
        .await
        .unwrap();
    assert_eq!(starts.total, 1);
    assert_eq!(starts.entries[0].slug.as_deref(), Some("rust-lang"));

    let ends = repo
        .list(
            TYPE_ARTICLE,
            &make(FilterOperator::EndsWith),
            DEFAULT_PAGE_SIZE,
            MAX_PAGE_SIZE,
        )
        .await
        .unwrap();
    assert_eq!(ends.total, 1);
    assert_eq!(ends.entries[0].slug.as_deref(), Some("world-of-rust"));
}

#[tokio::test]
async fn filter_eq_and_contains_on_json_field() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("a"),
        ContentStatus::Draft,
        json!({ "title": "Hello World" }),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("b"),
        ContentStatus::Draft,
        json!({ "title": "Goodbye" }),
    )
    .await;

    let q_eq = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Json("title".to_owned()),
            op: FilterOperator::Eq,
            value: json!("Hello World"),
        }],
        ..ContentQuery::default()
    };
    let res_eq = repo
        .list(TYPE_ARTICLE, &q_eq, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res_eq.total, 1);
    assert_eq!(res_eq.entries[0].fields["title"], "Hello World");

    let q_contains = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Json("title".to_owned()),
            op: FilterOperator::Contains,
            value: json!("World"),
        }],
        ..ContentQuery::default()
    };
    let res_contains = repo
        .list(TYPE_ARTICLE, &q_contains, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res_contains.total, 1);
}

#[tokio::test]
async fn filter_gt_lte_on_json_number_field() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "views": 5 }),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "views": 10 }),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "views": 100 }),
    )
    .await;

    let q_gt = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Json("views".to_owned()),
            op: FilterOperator::Gt,
            value: json!(5),
        }],
        ..ContentQuery::default()
    };
    let res_gt = repo
        .list(TYPE_ARTICLE, &q_gt, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res_gt.total, 2);

    let q_lte = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Json("views".to_owned()),
            op: FilterOperator::Lte,
            value: json!(10),
        }],
        ..ContentQuery::default()
    };
    let res_lte = repo
        .list(TYPE_ARTICLE, &q_lte, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res_lte.total, 2);
}

#[tokio::test]
async fn filter_not_in_excludes_listed_values() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("alpha"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("beta"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("gamma"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;

    let q = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op: FilterOperator::NotIn,
            value: json!(["alpha", "beta"]),
        }],
        ..ContentQuery::default()
    };
    let res = repo
        .list(TYPE_ARTICLE, &q, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(res.total, 1);
    assert_eq!(res.entries[0].slug.as_deref(), Some("gamma"));
}

#[tokio::test]
async fn sort_by_slug_asc_and_desc() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("c"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("a"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        Some("b"),
        ContentStatus::Draft,
        json!({}),
    )
    .await;

    let asc = ContentQuery {
        sort: vec![SortSpec {
            field: FieldRef::Column(ColumnField::Slug),
            direction: SortDir::Asc,
        }],
        ..ContentQuery::default()
    };
    let asc_res = repo
        .list(TYPE_ARTICLE, &asc, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    let asc_slugs: Vec<&str> = asc_res
        .entries
        .iter()
        .map(|e| e.slug.as_deref().unwrap())
        .collect();
    assert_eq!(asc_slugs, vec!["a", "b", "c"]);

    let desc = ContentQuery {
        sort: vec![SortSpec {
            field: FieldRef::Column(ColumnField::Slug),
            direction: SortDir::Desc,
        }],
        ..ContentQuery::default()
    };
    let desc_res = repo
        .list(TYPE_ARTICLE, &desc, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    let desc_slugs: Vec<&str> = desc_res
        .entries
        .iter()
        .map(|e| e.slug.as_deref().unwrap())
        .collect();
    assert_eq!(desc_slugs, vec!["c", "b", "a"]);
}

#[tokio::test]
async fn sort_by_json_field_asc() {
    let (_pool, repo) = prepare().await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "title": "Charlie" }),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "title": "Alpha" }),
    )
    .await;
    seed_entry(
        &repo,
        TYPE_ARTICLE,
        None,
        ContentStatus::Draft,
        json!({ "title": "Bravo" }),
    )
    .await;

    let q = ContentQuery {
        sort: vec![SortSpec {
            field: FieldRef::Json("title".to_owned()),
            direction: SortDir::Asc,
        }],
        ..ContentQuery::default()
    };
    let res = repo
        .list(TYPE_ARTICLE, &q, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    let titles: Vec<&str> = res
        .entries
        .iter()
        .map(|e| e.fields["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, vec!["Alpha", "Bravo", "Charlie"]);
}

#[tokio::test]
async fn pagination_bounds_and_meta() {
    let (_pool, repo) = prepare().await;
    for i in 0..5 {
        seed_entry(
            &repo,
            TYPE_ARTICLE,
            Some(&format!("s-{i}")),
            ContentStatus::Draft,
            json!({ "i": i }),
        )
        .await;
    }

    let q_p1 = ContentQuery {
        page: Some(1),
        page_size: Some(2),
        sort: vec![SortSpec {
            field: FieldRef::Column(ColumnField::Slug),
            direction: SortDir::Asc,
        }],
        ..ContentQuery::default()
    };
    let p1 = repo
        .list(TYPE_ARTICLE, &q_p1, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(p1.total, 5);
    assert_eq!(p1.page, 1);
    assert_eq!(p1.page_size, 2);
    assert_eq!(p1.entries.len(), 2);
    assert_eq!(p1.entries[0].slug.as_deref(), Some("s-0"));

    let q_p3 = ContentQuery {
        page: Some(3),
        page_size: Some(2),
        sort: vec![SortSpec {
            field: FieldRef::Column(ColumnField::Slug),
            direction: SortDir::Asc,
        }],
        ..ContentQuery::default()
    };
    let p3 = repo
        .list(TYPE_ARTICLE, &q_p3, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap();
    assert_eq!(p3.total, 5);
    assert_eq!(p3.entries.len(), 1);
    assert_eq!(p3.entries[0].slug.as_deref(), Some("s-4"));
}

#[tokio::test]
async fn invalid_json_path_is_rejected() {
    let (_pool, repo) = prepare().await;
    let q = ContentQuery {
        filters: vec![FilterSpec {
            field: FieldRef::Json("0bad".to_owned()),
            op: FilterOperator::Eq,
            value: json!("x"),
        }],
        ..ContentQuery::default()
    };
    let err = repo
        .list(TYPE_ARTICLE, &q, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("bad_request") || msg.contains("invalid"),
        "got: {msg}"
    );
}
