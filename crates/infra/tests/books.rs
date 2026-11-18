//! Book catalog integration tests (real Postgres via #[sqlx::test]).

use std::sync::Arc;

use domain::{
    dto::{AddCopiesRequest, BookForm, CreateUserRequest},
    models::Role,
};
use infra::{
    AppError, AppState, Config,
    repo::{books, users},
    service::book_files::{self, BookFile, Upload},
};
use sqlx::PgPool;
use uuid::Uuid;

fn state(db: &PgPool, dir: &std::path::Path) -> AppState {
    AppState {
        db: db.clone(),
        config: Arc::new(Config {
            database_url: String::new(),
            db_max_connections: 1,
            jwt_secret: "integration-test-secret-integration-test".into(),
            jwt_ttl_minutes: 5,
            cookie_secure: false,
            cors_origins: vec![],
            bootstrap_admin: None,
            bag_photo_dir: dir.to_path_buf(),
            book_files_dir: dir.join("books"),
            models_dir: "models".into(),
        }),
        faces: None,
    }
}

fn test_dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("library-books-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

async fn librarian(db: &PgPool) -> Uuid {
    users::create(
        db,
        &CreateUserRequest {
            email: "lib@example.com".into(),
            password: "password123".into(),
            full_name: "Librarian".into(),
            role: Role::Librarian,
        },
    )
    .await
    .unwrap()
    .id
}

fn form(title: &str, category: &str, barcodes: &str) -> BookForm {
    BookForm {
        title: title.into(),
        authors: "Sok Dara, Chan Vanna".into(),
        category: category.into(),
        published_year: "2020".into(),
        isbn: "978-0-13-235088-4".into(),
        copy_barcodes: barcodes.into(),
        shelf_location: "A3".into(),
        ..Default::default()
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_update_and_filter(db: PgPool) {
    let staff = librarian(&db).await;
    let book = books::create(&db, &form("Physics 101", "Science", "bk-1, BK-2").to_create().unwrap(), staff)
        .await
        .unwrap();
    assert_eq!(book.authors, "Sok Dara, Chan Vanna");
    assert_eq!(book.category.as_deref(), Some("Science"));
    assert_eq!(book.isbn.as_deref(), Some("9780132350884"));
    assert_eq!(books::copies(&db, book.id).await.unwrap().len(), 2);

    // Category names are matched case-insensitively, not duplicated.
    let mut f = form("Physics 101", "science", "");
    f.authors = "Chan Vanna".into();
    f.isbn.clear();
    let updated = books::update(&db, book.id, &f.to_update().unwrap(), staff).await.unwrap();
    assert_eq!(updated.authors, "Chan Vanna");
    assert_eq!(updated.category.as_deref(), Some("Science"));
    assert_eq!(books::categories(&db).await.unwrap(), vec!["Science".to_string()]);

    books::add_copies(&db, book.id, &AddCopiesRequest { barcodes: vec!["BK-3".into()], shelf_location: None }, staff)
        .await
        .unwrap();
    let dup = books::add_copies(&db, book.id, &AddCopiesRequest { barcodes: vec!["BK-3".into()], shelf_location: None }, staff).await;
    assert!(matches!(dup, Err(AppError::Conflict(_))));

    let mut other = form("Khmer poems", "Literature", "");
    other.isbn.clear();
    books::create(&db, &other.to_create().unwrap(), staff).await.unwrap();

    let science = books::search(&db, "", "Science", false, 50, 0).await.unwrap();
    assert_eq!(science.len(), 1);
    assert_eq!(science[0].total_copies, 3);
    assert!(books::search(&db, "", "", true, 50, 0).await.unwrap().is_empty(), "no e-books yet");
    assert_eq!(books::search(&db, "poems", "", false, 50, 0).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn ebook_upload_makes_book_readable_online(db: PgPool) {
    let dir = test_dir();
    let state = state(&db, &dir);
    let staff = librarian(&db).await;
    let mut f = form("Online only", "", "");
    f.isbn.clear();
    let book = books::create(&db, &f.to_create().unwrap(), staff).await.unwrap();

    // Not a PDF: rejected, nothing left on disk.
    let mut up = Upload::begin(&state, book.id, BookFile::Ebook).await.unwrap();
    up.write(b"<html>not a pdf</html>").await.unwrap();
    assert!(matches!(up.finish(&state, staff).await, Err(AppError::Conflict(_))));
    assert!(!books::get(&db, book.id).await.unwrap().has_ebook());

    let mut up = Upload::begin(&state, book.id, BookFile::Ebook).await.unwrap();
    up.write(b"%PDF-1.7\n").await.unwrap();
    up.write(&[b'x'; 1000]).await.unwrap();
    up.finish(&state, staff).await.unwrap();

    let b = books::get(&db, book.id).await.unwrap();
    assert_eq!(b.ebook_bytes, Some(1009));
    assert!(book_files::path(&state, book.id, BookFile::Ebook).exists());
    let online = books::search(&db, "", "", true, 50, 0).await.unwrap();
    assert_eq!(online.len(), 1);
    assert!(online[0].has_ebook);
    let leftovers = std::fs::read_dir(dir.join("books").join(book.id.to_string())).unwrap().count();
    assert_eq!(leftovers, 1, "temp files are cleaned up");

    book_files::remove(&state, book.id, BookFile::Ebook, staff).await.unwrap();
    assert!(!books::get(&db, book.id).await.unwrap().has_ebook());
    assert!(!book_files::path(&state, book.id, BookFile::Ebook).exists());

    // Cover too large.
    let mut up = Upload::begin(&state, book.id, BookFile::Cover).await.unwrap();
    let big = vec![0u8; (book_files::COVER_MAX_BYTES + 1) as usize];
    assert!(matches!(up.write(&big).await, Err(AppError::Conflict(_))));

    book_files::delete_book(&state, book.id, staff).await.unwrap();
    assert!(matches!(books::get(&db, book.id).await, Err(AppError::NotFound(_))));
    let _ = std::fs::remove_dir_all(dir);
}

#[sqlx::test(migrations = "../../migrations")]
async fn borrowed_book_cannot_be_deleted(db: PgPool) {
    let dir = test_dir();
    let state = state(&db, &dir);
    let staff = librarian(&db).await;
    let mut f = form("Borrowed", "", "BK-9");
    f.isbn.clear();
    let book = books::create(&db, &f.to_create().unwrap(), staff).await.unwrap();
    let copy = books::copies(&db, book.id).await.unwrap().remove(0);
    let member: Uuid = sqlx::query_scalar(
        "INSERT INTO members (id, member_type_id, card_number, full_name, expires_on)
         VALUES ($1, '00000000-0000-0000-0000-000000000001', 'S-0001', 'Student', CURRENT_DATE + 30) RETURNING id",
    )
    .bind(Uuid::new_v4())
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO loans (id, copy_id, member_id, issued_by, due_date) VALUES ($1, $2, $3, $4, CURRENT_DATE)")
        .bind(Uuid::new_v4())
        .bind(copy.id)
        .bind(member)
        .bind(staff)
        .execute(&db)
        .await
        .unwrap();
    let res = book_files::delete_book(&state, book.id, staff).await;
    assert!(matches!(res, Err(AppError::Conflict(_))));
    assert!(books::get(&db, book.id).await.is_ok());
    let _ = std::fs::remove_dir_all(dir);
}
