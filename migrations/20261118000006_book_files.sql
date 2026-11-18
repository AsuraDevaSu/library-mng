-- Book covers and e-books (read online). Files live on disk under BOOK_FILES_DIR/<book id>/;
-- these columns record whether they exist.
ALTER TABLE books
    ADD COLUMN cover_mime  TEXT,     -- image/jpeg | image/png | image/webp; NULL = no cover
    ADD COLUMN ebook_bytes BIGINT;   -- size of ebook.pdf; NULL = not readable online

CREATE INDEX books_with_ebook ON books (title) WHERE ebook_bytes IS NOT NULL;
CREATE INDEX books_category ON books (category_id);
