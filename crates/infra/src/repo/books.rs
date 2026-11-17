use domain::{
    dto::CreateBookRequest,
    models::{Book, BookSummary, Copy, SiteInfo},
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{audit, like_pattern};
use crate::{AppError, AppResult, error::map_unique};

const BOOK_COLS: &str =
    "id, isbn, title, subtitle, publisher, published_year, language, description, created_at";

/// Catalog search by title, author (substring, trigram-indexed) or exact ISBN.
pub async fn search(db: &PgPool, q: &str, limit: i64, offset: i64) -> AppResult<Vec<BookSummary>> {
    let q = q.trim();
    Ok(sqlx::query_as(
        r#"
        SELECT b.id, b.title, b.isbn, b.published_year,
               COALESCE((SELECT string_agg(a.name, ', ' ORDER BY ba.position)
                           FROM book_authors ba JOIN authors a ON a.id = ba.author_id
                          WHERE ba.book_id = b.id), '') AS authors,
               (SELECT count(*) FROM copies c WHERE c.book_id = b.id AND c.status <> 'withdrawn') AS total_copies,
               (SELECT count(*) FROM copies c WHERE c.book_id = b.id AND c.status = 'available') AS available_copies
          FROM books b
         WHERE $1 = ''
            OR b.isbn = $1
            OR b.title ILIKE $2
            OR EXISTS (SELECT 1 FROM book_authors ba JOIN authors a ON a.id = ba.author_id
                        WHERE ba.book_id = b.id AND a.name ILIKE $2)
         ORDER BY b.title
         LIMIT $3 OFFSET $4
        "#,
    )
    .bind(q)
    .bind(like_pattern(q))
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?)
}

pub async fn get(db: &PgPool, id: Uuid) -> AppResult<Book> {
    sqlx::query_as(&format!("SELECT {BOOK_COLS} FROM books WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound("book"))
}

pub async fn copies(db: &PgPool, book_id: Uuid) -> AppResult<Vec<Copy>> {
    Ok(sqlx::query_as(
        "SELECT id, book_id, barcode, shelf_location, status FROM copies WHERE book_id = $1 ORDER BY barcode",
    )
    .bind(book_id)
    .fetch_all(db)
    .await?)
}

/// Creates a book with its authors and physical copies in one transaction.
pub async fn create(db: &PgPool, req: &CreateBookRequest, staff_id: Uuid) -> AppResult<Book> {
    let mut tx = db.begin().await?;

    let book: Book = sqlx::query_as(&format!(
        "INSERT INTO books (id, isbn, title, subtitle, publisher, published_year, language, description)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {BOOK_COLS}"
    ))
    .bind(Uuid::new_v4())
    .bind(&req.isbn)
    .bind(req.title.trim())
    .bind(&req.subtitle)
    .bind(&req.publisher)
    .bind(req.published_year)
    .bind(&req.language)
    .bind(&req.description)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_unique(e, "book with this ISBN"))?;

    let mut seen = std::collections::HashSet::new();
    let authors = req
        .authors
        .iter()
        .map(|a| a.trim())
        .filter(|a| !a.is_empty() && seen.insert(a.to_lowercase()));
    for (position, name) in authors.enumerate() {
        let author_id: Uuid = sqlx::query_scalar(
            "INSERT INTO authors (id, name) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO book_authors (book_id, author_id, position) VALUES ($1, $2, $3)")
            .bind(book.id)
            .bind(author_id)
            .bind(position as i16)
            .execute(&mut *tx)
            .await?;
    }

    for barcode in &req.copy_barcodes {
        sqlx::query("INSERT INTO copies (id, book_id, barcode, shelf_location) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(book.id)
            .bind(barcode.trim().to_uppercase())
            .bind(&req.shelf_location)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_unique(e, &format!("copy with barcode {barcode}")))?;
    }

    audit::log(
        &mut *tx,
        staff_id,
        "book.create",
        "book",
        book.id,
        serde_json::json!({ "title": book.title, "copies": req.copy_barcodes.len() }),
    )
    .await?;
    tx.commit().await?;
    Ok(book)
}

/// Public numbers + library name/hours for the website.
pub async fn site_info(db: &PgPool) -> AppResult<SiteInfo> {
    Ok(sqlx::query_as(
        r#"
        SELECT s.library_name, s.opening_hours,
               (SELECT count(*) FROM books)                                      AS titles,
               (SELECT count(*) FROM copies WHERE status <> 'withdrawn')         AS copies,
               (SELECT count(*) FROM copies WHERE status = 'available')          AS available_copies,
               (SELECT count(*) FROM members WHERE status = 'active')            AS members
          FROM library_settings s WHERE s.id = 1
        "#,
    )
    .fetch_one(db)
    .await?)
}
