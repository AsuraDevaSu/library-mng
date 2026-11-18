use domain::{
    dto::{AddCopiesRequest, CreateBookRequest, UpdateBookRequest},
    models::{Book, BookSummary, Copy, SiteInfo},
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{audit, like_pattern};
use crate::{AppError, AppResult, error::map_unique};

/// Book row with its category and authors resolved. Append `WHERE …`.
const BOOK_SELECT: &str = r#"
    SELECT b.id, b.isbn, b.title, b.subtitle, b.publisher, b.published_year, b.language, b.description,
           cat.name AS category,
           COALESCE((SELECT string_agg(a.name, ', ' ORDER BY ba.position)
                       FROM book_authors ba JOIN authors a ON a.id = ba.author_id
                      WHERE ba.book_id = b.id), '') AS authors,
           b.cover_mime IS NOT NULL AS has_cover, b.ebook_bytes, b.created_at, b.updated_at
      FROM books b LEFT JOIN categories cat ON cat.id = b.category_id
"#;

/// Catalog search by title, author (substring, trigram-indexed) or exact ISBN,
/// optionally narrowed to one category and/or books readable online.
pub async fn search(
    db: &PgPool,
    q: &str,
    category: &str,
    online_only: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<BookSummary>> {
    let q = q.trim();
    Ok(sqlx::query_as(
        r#"
        SELECT b.id, b.title, b.isbn, b.published_year, cat.name AS category,
               COALESCE((SELECT string_agg(a.name, ', ' ORDER BY ba.position)
                           FROM book_authors ba JOIN authors a ON a.id = ba.author_id
                          WHERE ba.book_id = b.id), '') AS authors,
               (SELECT count(*) FROM copies c WHERE c.book_id = b.id AND c.status <> 'withdrawn') AS total_copies,
               (SELECT count(*) FROM copies c WHERE c.book_id = b.id AND c.status = 'available') AS available_copies,
               b.cover_mime IS NOT NULL AS has_cover,
               b.ebook_bytes IS NOT NULL AS has_ebook,
               b.updated_at
          FROM books b LEFT JOIN categories cat ON cat.id = b.category_id
         WHERE ($1 = ''
                OR b.isbn = upper(replace($1, '-', ''))
                OR b.title ILIKE $2
                OR EXISTS (SELECT 1 FROM book_authors ba JOIN authors a ON a.id = ba.author_id
                            WHERE ba.book_id = b.id AND a.name ILIKE $2))
           AND ($3 = '' OR cat.name = $3)
           AND (NOT $4 OR b.ebook_bytes IS NOT NULL)
         ORDER BY b.title
         LIMIT $5 OFFSET $6
        "#,
    )
    .bind(q)
    .bind(like_pattern(q))
    .bind(category.trim())
    .bind(online_only)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?)
}

pub async fn get(db: &PgPool, id: Uuid) -> AppResult<Book> {
    fetch(&mut *db.acquire().await?, id).await
}

async fn fetch(conn: &mut PgConnection, id: Uuid) -> AppResult<Book> {
    sqlx::query_as(&format!("{BOOK_SELECT} WHERE b.id = $1"))
        .bind(id)
        .fetch_optional(conn)
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

/// Category names that have at least one book, for catalog filters.
pub async fn categories(db: &PgPool) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT c.name FROM categories c WHERE EXISTS (SELECT 1 FROM books b WHERE b.category_id = c.id) ORDER BY c.name",
    )
    .fetch_all(db)
    .await?)
}

/// Finds or creates a category by name (case-insensitive match on existing names).
async fn category_id(conn: &mut PgConnection, name: Option<&str>) -> AppResult<Option<Uuid>> {
    let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    if let Some(id) = sqlx::query_scalar("SELECT id FROM categories WHERE lower(name) = lower($1)")
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?
    {
        return Ok(Some(id));
    }
    let id = sqlx::query_scalar(
        "INSERT INTO categories (id, name) VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(name)
    .fetch_one(&mut *conn)
    .await?;
    Ok(Some(id))
}

/// Replaces a book's author list (keeps order, ignores duplicates and blanks).
async fn set_authors(conn: &mut PgConnection, book_id: Uuid, authors: &[String]) -> AppResult<()> {
    sqlx::query("DELETE FROM book_authors WHERE book_id = $1").bind(book_id).execute(&mut *conn).await?;
    let mut seen = std::collections::HashSet::new();
    let authors = authors
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
        .fetch_one(&mut *conn)
        .await?;
        sqlx::query("INSERT INTO book_authors (book_id, author_id, position) VALUES ($1, $2, $3)")
            .bind(book_id)
            .bind(author_id)
            .bind(position as i16)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn insert_copies(
    conn: &mut PgConnection,
    book_id: Uuid,
    barcodes: &[String],
    shelf: &Option<String>,
) -> AppResult<()> {
    for barcode in barcodes {
        let barcode = barcode.trim().to_uppercase();
        sqlx::query("INSERT INTO copies (id, book_id, barcode, shelf_location) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(book_id)
            .bind(&barcode)
            .bind(shelf)
            .execute(&mut *conn)
            .await
            .map_err(|e| map_unique(e, &format!("copy with barcode {barcode}")))?;
    }
    Ok(())
}

/// Creates a book with its authors, category and physical copies in one transaction.
pub async fn create(db: &PgPool, req: &CreateBookRequest, staff_id: Uuid) -> AppResult<Book> {
    let mut tx = db.begin().await?;
    let id = Uuid::new_v4();
    let category = category_id(&mut tx, req.category.as_deref()).await?;

    sqlx::query(
        "INSERT INTO books (id, isbn, title, subtitle, publisher, published_year, language, description, category_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(&req.isbn)
    .bind(req.title.trim())
    .bind(&req.subtitle)
    .bind(&req.publisher)
    .bind(req.published_year)
    .bind(&req.language)
    .bind(&req.description)
    .bind(category)
    .execute(&mut *tx)
    .await
    .map_err(|e| map_unique(e, "book with this ISBN"))?;

    set_authors(&mut tx, id, &req.authors).await?;
    insert_copies(&mut tx, id, &req.copy_barcodes, &req.shelf_location).await?;

    audit::log(
        &mut *tx,
        staff_id,
        "book.create",
        "book",
        id,
        serde_json::json!({ "title": req.title.trim(), "copies": req.copy_barcodes.len() }),
    )
    .await?;
    let book = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(book)
}

pub async fn update(db: &PgPool, id: Uuid, req: &UpdateBookRequest, staff_id: Uuid) -> AppResult<Book> {
    let mut tx = db.begin().await?;
    let category = category_id(&mut tx, req.category.as_deref()).await?;
    let done = sqlx::query(
        "UPDATE books SET isbn = $2, title = $3, subtitle = $4, publisher = $5, published_year = $6,
                          language = $7, description = $8, category_id = $9, updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(&req.isbn)
    .bind(req.title.trim())
    .bind(&req.subtitle)
    .bind(&req.publisher)
    .bind(req.published_year)
    .bind(&req.language)
    .bind(&req.description)
    .bind(category)
    .execute(&mut *tx)
    .await
    .map_err(|e| map_unique(e, "book with this ISBN"))?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound("book"));
    }
    set_authors(&mut tx, id, &req.authors).await?;
    audit::log(&mut *tx, staff_id, "book.update", "book", id, serde_json::json!({ "title": req.title.trim() }))
        .await?;
    let book = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(book)
}

pub async fn add_copies(db: &PgPool, book_id: Uuid, req: &AddCopiesRequest, staff_id: Uuid) -> AppResult<Vec<Copy>> {
    let mut tx = db.begin().await?;
    fetch(&mut tx, book_id).await?;
    insert_copies(&mut tx, book_id, &req.barcodes, &req.shelf_location).await?;
    audit::log(&mut *tx, staff_id, "book.add_copies", "book", book_id, serde_json::json!({ "barcodes": req.barcodes }))
        .await?;
    tx.commit().await?;
    copies(db, book_id).await
}

/// Deletes a book that has never been borrowed or reserved. Books with history
/// are kept for the records: withdraw their copies instead.
pub async fn delete(db: &PgPool, id: Uuid, staff_id: Uuid) -> AppResult<()> {
    let mut tx = db.begin().await?;
    let book = fetch(&mut tx, id).await?;
    let used: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM loans l JOIN copies c ON c.id = l.copy_id WHERE c.book_id = $1)
             OR EXISTS (SELECT 1 FROM reservations WHERE book_id = $1)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if used {
        return Err(AppError::Conflict(
            "This book has loan or reservation history, so it can't be deleted. Remove its e-book or edit it instead."
                .into(),
        ));
    }
    sqlx::query("DELETE FROM books WHERE id = $1").bind(id).execute(&mut *tx).await?;
    audit::log(&mut *tx, staff_id, "book.delete", "book", id, serde_json::json!({ "title": book.title })).await?;
    tx.commit().await?;
    Ok(())
}

/// Records that a cover was uploaded (`Some(mime)`) or removed (`None`).
pub async fn set_cover(db: &PgPool, id: Uuid, mime: Option<&str>, staff_id: Uuid) -> AppResult<()> {
    let mut tx = db.begin().await?;
    let done = sqlx::query("UPDATE books SET cover_mime = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(mime)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound("book"));
    }
    let action = if mime.is_some() { "book.cover_upload" } else { "book.cover_remove" };
    audit::log(&mut *tx, staff_id, action, "book", id, serde_json::json!({})).await?;
    tx.commit().await?;
    Ok(())
}

/// Records that an e-book was uploaded (`Some(bytes)`) or removed (`None`).
pub async fn set_ebook(db: &PgPool, id: Uuid, bytes: Option<i64>, staff_id: Uuid) -> AppResult<()> {
    let mut tx = db.begin().await?;
    let done = sqlx::query("UPDATE books SET ebook_bytes = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(bytes)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound("book"));
    }
    let action = if bytes.is_some() { "book.ebook_upload" } else { "book.ebook_remove" };
    audit::log(&mut *tx, staff_id, action, "book", id, serde_json::json!({ "bytes": bytes })).await?;
    tx.commit().await?;
    Ok(())
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
