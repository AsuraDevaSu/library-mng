//! Book covers and e-books on disk: `BOOK_FILES_DIR/<book id>/{cover,ebook.pdf}`.
//! Uploads are streamed to a temp file, size-limited, checked by their magic
//! bytes (not the client's file name or MIME type), then renamed into place.

use std::{path::PathBuf, str::FromStr};

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{AppError, AppResult, AppState, repo::books};

pub const COVER_MAX_BYTES: u64 = 5 * 1024 * 1024;
pub const EBOOK_MAX_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookFile {
    Cover,
    Ebook,
}

impl FromStr for BookFile {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cover" => Ok(Self::Cover),
            "ebook" => Ok(Self::Ebook),
            _ => Err(AppError::NotFound("file")),
        }
    }
}

impl BookFile {
    pub fn max_bytes(self) -> u64 {
        match self {
            Self::Cover => COVER_MAX_BYTES,
            Self::Ebook => EBOOK_MAX_BYTES,
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Ebook => "ebook.pdf",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cover => "Cover image",
            Self::Ebook => "E-book",
        }
    }
}

fn io(e: std::io::Error) -> AppError {
    AppError::Internal(e.to_string())
}

fn book_dir(state: &AppState, id: Uuid) -> PathBuf {
    state.config.book_files_dir.join(id.to_string())
}

pub fn path(state: &AppState, id: Uuid, kind: BookFile) -> PathBuf {
    book_dir(state, id).join(kind.file_name())
}

/// MIME type of a supported cover image, from its first bytes.
pub fn sniff_image(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

pub fn is_pdf(head: &[u8]) -> bool {
    head.starts_with(b"%PDF-")
}

/// An upload in progress. Dropping it without `finish` deletes the temp file.
pub struct Upload {
    book_id: Uuid,
    kind: BookFile,
    tmp: PathBuf,
    file: tokio::fs::File,
    written: u64,
    head: Vec<u8>,
    done: bool,
}

impl Upload {
    pub async fn begin(state: &AppState, book_id: Uuid, kind: BookFile) -> AppResult<Self> {
        books::get(&state.db, book_id).await?; // 404 before writing anything
        let dir = book_dir(state, book_id);
        tokio::fs::create_dir_all(&dir).await.map_err(io)?;
        let tmp = dir.join(format!(".upload-{}", Uuid::new_v4()));
        let file = tokio::fs::File::create(&tmp).await.map_err(io)?;
        Ok(Self { book_id, kind, tmp, file, written: 0, head: Vec::with_capacity(16), done: false })
    }

    pub async fn write(&mut self, chunk: &[u8]) -> AppResult<()> {
        self.written += chunk.len() as u64;
        if self.written > self.kind.max_bytes() {
            return Err(AppError::Conflict(format!(
                "{} is too large (max {} MB).",
                self.kind.label(),
                self.kind.max_bytes() / (1024 * 1024)
            )));
        }
        if self.head.len() < 16 {
            let take = (16 - self.head.len()).min(chunk.len());
            self.head.extend_from_slice(&chunk[..take]);
        }
        self.file.write_all(chunk).await.map_err(io)
    }

    /// Checks the file type, moves the file into place and records it on the book.
    pub async fn finish(mut self, state: &AppState, staff_id: Uuid) -> AppResult<()> {
        if self.written == 0 {
            return Err(AppError::Conflict("Choose a file to upload.".into()));
        }
        let cover_mime = match self.kind {
            BookFile::Cover => Some(sniff_image(&self.head).ok_or_else(|| {
                AppError::Conflict("Cover must be a JPEG, PNG or WebP image.".into())
            })?),
            BookFile::Ebook if !is_pdf(&self.head) => {
                return Err(AppError::Conflict("E-book must be a PDF file.".into()));
            }
            BookFile::Ebook => None,
        };
        self.file.flush().await.map_err(io)?;
        self.file.sync_all().await.map_err(io)?;
        tokio::fs::rename(&self.tmp, path(state, self.book_id, self.kind)).await.map_err(io)?;
        self.done = true;
        match self.kind {
            BookFile::Cover => books::set_cover(&state.db, self.book_id, cover_mime, staff_id).await,
            BookFile::Ebook => books::set_ebook(&state.db, self.book_id, Some(self.written as i64), staff_id).await,
        }
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        if !self.done {
            let _ = std::fs::remove_file(&self.tmp);
        }
    }
}

/// Removes a cover or e-book from the book and from disk.
pub async fn remove(state: &AppState, book_id: Uuid, kind: BookFile, staff_id: Uuid) -> AppResult<()> {
    match kind {
        BookFile::Cover => books::set_cover(&state.db, book_id, None, staff_id).await?,
        BookFile::Ebook => books::set_ebook(&state.db, book_id, None, staff_id).await?,
    }
    match tokio::fs::remove_file(path(state, book_id, kind)).await {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io(e)),
        _ => Ok(()),
    }
}

/// Deletes the book (see [`books::delete`]) and its files.
pub async fn delete_book(state: &AppState, book_id: Uuid, staff_id: Uuid) -> AppResult<()> {
    books::delete(&state.db, book_id, staff_id).await?;
    let _ = tokio::fs::remove_dir_all(book_dir(state, book_id)).await;
    Ok(())
}

/// Path and MIME type of a stored file, if the book has it.
pub async fn stored(state: &AppState, book_id: Uuid, kind: BookFile) -> AppResult<(PathBuf, String)> {
    let book = books::get(&state.db, book_id).await?;
    let mime: Option<String> = match kind {
        BookFile::Cover if book.has_cover => {
            sqlx::query_scalar("SELECT cover_mime FROM books WHERE id = $1").bind(book_id).fetch_one(&state.db).await?
        }
        BookFile::Ebook if book.has_ebook() => Some("application/pdf".into()),
        _ => None,
    };
    let mime = mime.ok_or(AppError::NotFound("file"))?;
    Ok((path(state, book_id, kind), mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_types() {
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(sniff_image(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff_image(b"%PDF-1.7"), None);
        assert!(is_pdf(b"%PDF-1.7\n"));
        assert!(!is_pdf(b"<html>"));
    }
}
