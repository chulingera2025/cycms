use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver, MediaConfig};
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_media::{MediaManager, MediaQuery, UploadInput};
use cycms_migrate::MigrationEngine;
use tempfile::TempDir;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

/// 内存 SQLite + 系统迁移 + 临时存储目录。
struct Setup {
    pool: Arc<DatabasePool>,
    manager: MediaManager,
    _storage_dir: TempDir,
}

async fn make_setup() -> Setup {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("sqlite connect"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("migrations");

    seed_user(&pool).await;

    let storage_dir = TempDir::new().expect("tempdir");
    let event_bus = Arc::new(EventBus::new());

    let config = MediaConfig {
        upload_dir: storage_dir.path().to_str().unwrap().to_owned(),
        max_file_size: 1024 * 1024,
        allowed_mime_types: vec!["image/png".to_owned(), "image/jpeg".to_owned()],
        on_referenced_delete: "block".to_owned(),
    };

    let manager = MediaManager::new(&pool, Arc::clone(&event_bus), &config);

    Setup {
        pool,
        manager,
        _storage_dir: storage_dir,
    }
}

async fn seed_user(pool: &Arc<DatabasePool>) {
    let DatabasePool::Sqlite(p) = pool.as_ref() else {
        panic!("expected sqlite");
    };
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) \
         VALUES ('user-01', 'tester', 'test@example.com', 'hash')",
    )
    .execute(p)
    .await
    .expect("seed user");
}

/// 最小的 PNG 文件（1×1 像素白色）。
fn tiny_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

fn jpeg_header() -> Vec<u8> {
    // JPEG magic bytes + 最小有效 JFIF 头
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00,
    ]
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn upload_stores_file_and_returns_asset() {
    let s = make_setup().await;
    let data = tiny_png();
    let input = UploadInput {
        original_filename: "avatar.png".to_owned(),
        data: data.clone(),
        mime_type: None,
        uploaded_by: "user-01".to_owned(),
        metadata: None,
    };

    let asset = s.manager.upload(input).await.expect("upload");

    assert_eq!(asset.mime_type, "image/png");
    assert_eq!(asset.original_filename, "avatar.png");
    assert_eq!(asset.filename, "avatar.png");
    assert_eq!(asset.size, data.len() as i64);
    assert_eq!(asset.uploaded_by, "user-01");
    assert!(!asset.storage_path.is_empty());

    // 文件确实落盘了
    let file_path = s._storage_dir.path().join(&asset.storage_path);
    assert!(file_path.exists(), "stored file must exist on disk");
}

#[tokio::test]
async fn upload_detects_mime_type_from_magic_bytes() {
    let s = make_setup().await;
    let asset = s
        .manager
        .upload(UploadInput {
            original_filename: "photo.jpg".to_owned(),
            data: jpeg_header(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .expect("upload");

    assert_eq!(asset.mime_type, "image/jpeg");
}

#[tokio::test]
async fn upload_rejects_disallowed_mime_type() {
    let s = make_setup().await;
    let err = s
        .manager
        .upload(UploadInput {
            original_filename: "doc.pdf".to_owned(),
            data: b"%PDF-1.4 ...".to_vec(),
            mime_type: Some("application/pdf".to_owned()),
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .expect_err("should reject pdf");

    assert!(
        matches!(err, cycms_media::MediaError::DisallowedMimeType(_)),
        "expected DisallowedMimeType, got: {err:?}"
    );
}

#[tokio::test]
async fn upload_rejects_oversized_file() {
    let storage_dir = TempDir::new().unwrap();
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .unwrap(),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .unwrap();
    seed_user(&pool).await;

    let config = MediaConfig {
        upload_dir: storage_dir.path().to_str().unwrap().to_owned(),
        max_file_size: 10, // 只允许 10 字节
        allowed_mime_types: vec![],
        on_referenced_delete: "block".to_owned(),
    };
    let manager = MediaManager::new(&pool, Arc::new(EventBus::new()), &config);

    let err = manager
        .upload(UploadInput {
            original_filename: "big.png".to_owned(),
            data: vec![0u8; 20],
            mime_type: Some("image/png".to_owned()),
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .expect_err("should reject oversized");

    assert!(
        matches!(err, cycms_media::MediaError::FileTooLarge { .. }),
        "expected FileTooLarge, got: {err:?}"
    );
}

#[tokio::test]
async fn get_by_id_returns_uploaded_asset() {
    let s = make_setup().await;
    let asset = s
        .manager
        .upload(UploadInput {
            original_filename: "img.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();

    let found = s.manager.get(&asset.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, asset.id);
}

#[tokio::test]
async fn get_nonexistent_returns_none() {
    let s = make_setup().await;
    let result = s.manager.get("no-such-id").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn list_with_mime_filter() {
    let s = make_setup().await;

    // 上传一个 PNG 和一个 JPEG
    s.manager
        .upload(UploadInput {
            original_filename: "a.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();
    s.manager
        .upload(UploadInput {
            original_filename: "b.jpg".to_owned(),
            data: jpeg_header(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();

    let result = s
        .manager
        .list(&MediaQuery {
            mime_type: Some("image/png".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.data[0].mime_type, "image/png");
}

#[tokio::test]
async fn list_with_filename_filter() {
    let s = make_setup().await;

    s.manager
        .upload(UploadInput {
            original_filename: "banner.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();
    s.manager
        .upload(UploadInput {
            original_filename: "logo.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();

    let result = s
        .manager
        .list(&MediaQuery {
            filename_contains: Some("banner".to_owned()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(result.total, 1);
    assert!(result.data[0].original_filename.contains("banner"));
}

#[tokio::test]
async fn list_pagination() {
    let s = make_setup().await;

    // 上传 3 个资产
    for i in 0..3u8 {
        s.manager
            .upload(UploadInput {
                original_filename: format!("img{i}.png"),
                data: tiny_png(),
                mime_type: None,
                uploaded_by: "user-01".to_owned(),
                metadata: None,
            })
            .await
            .unwrap();
    }

    let page1 = s
        .manager
        .list(&MediaQuery {
            page: Some(1),
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page1.total, 3);
    assert_eq!(page1.data.len(), 2);
    assert_eq!(page1.page_count, 2);

    let page2 = s
        .manager
        .list(&MediaQuery {
            page: Some(2),
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page2.data.len(), 1);
}

#[tokio::test]
async fn delete_unreferenced_removes_file_and_db_record() {
    let s = make_setup().await;
    let asset = s
        .manager
        .upload(UploadInput {
            original_filename: "del.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();

    let file_path = s._storage_dir.path().join(&asset.storage_path);
    assert!(file_path.exists());

    s.manager.delete(&asset.id).await.expect("delete");

    assert!(!file_path.exists(), "file must be deleted from disk");
    assert!(s.manager.get(&asset.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_with_block_policy_on_referenced_returns_error() {
    let s = make_setup().await;
    let asset = s
        .manager
        .upload(UploadInput {
            original_filename: "ref.png".to_owned(),
            data: tiny_png(),
            mime_type: None,
            uploaded_by: "user-01".to_owned(),
            metadata: None,
        })
        .await
        .unwrap();

    // 插入一个 content_type，再插入一个 content_entry，其 fields 包含该资产 ID
    let DatabasePool::Sqlite(pool) = s.pool.as_ref() else {
        panic!("expected sqlite");
    };
    sqlx::query(
        "INSERT INTO content_types (id, api_id, name, kind) \
         VALUES ('ct-01', 'article', 'Article', 'collection')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO content_entries \
         (id, content_type_id, status, fields, created_by, updated_by) \
         VALUES ('entry-01', 'ct-01', 'draft', ?, 'user-01', 'user-01')",
    )
    .bind(format!(r#"{{"cover_image":"{}"}}"#, asset.id))
    .execute(pool)
    .await
    .unwrap();

    let err = s
        .manager
        .delete(&asset.id)
        .await
        .expect_err("should block deletion");

    assert!(
        matches!(err, cycms_media::MediaError::ReferencedAsset(_, n) if n == 1),
        "expected ReferencedAsset(_, 1), got: {err:?}"
    );
}
