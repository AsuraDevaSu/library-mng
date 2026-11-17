-- Library management: MVP schema
CREATE EXTENSION IF NOT EXISTS pg_trgm;   -- fuzzy / substring search on titles & authors

CREATE TYPE user_role          AS ENUM ('admin', 'librarian');
CREATE TYPE member_status      AS ENUM ('active', 'suspended', 'expired');
CREATE TYPE copy_status        AS ENUM ('available', 'on_loan', 'on_hold_shelf', 'lost', 'damaged', 'withdrawn');
CREATE TYPE reservation_status AS ENUM ('waiting', 'ready', 'fulfilled', 'cancelled', 'expired');
CREATE TYPE fine_status        AS ENUM ('unpaid', 'paid', 'waived');

-- ---------- staff ----------
CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT        NOT NULL,  -- stored lower-cased by the app
    password_hash TEXT        NOT NULL,
    full_name     TEXT        NOT NULL,
    role          user_role   NOT NULL,
    is_active     BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_lower ON users (lower(email));

-- ---------- members ----------
CREATE TABLE member_types (
    id                   UUID PRIMARY KEY,
    name                 TEXT          NOT NULL UNIQUE,
    max_loans            INT           NOT NULL CHECK (max_loans >= 0),
    loan_days            INT           NOT NULL CHECK (loan_days > 0),
    max_renewals         INT           NOT NULL CHECK (max_renewals >= 0),
    daily_fine           NUMERIC(10,2) NOT NULL CHECK (daily_fine >= 0),
    max_fine             NUMERIC(10,2) NOT NULL CHECK (max_fine >= 0),
    fine_block_threshold NUMERIC(10,2) NOT NULL CHECK (fine_block_threshold >= 0)
);

CREATE TABLE members (
    id             UUID PRIMARY KEY,
    member_type_id UUID          NOT NULL REFERENCES member_types(id),
    card_number    TEXT          NOT NULL UNIQUE,
    full_name      TEXT          NOT NULL,
    email          TEXT,
    phone          TEXT,
    status         member_status NOT NULL DEFAULT 'active',
    expires_on     DATE          NOT NULL,
    created_at     TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ   NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX members_email_lower ON members (lower(email)) WHERE email IS NOT NULL;
CREATE INDEX members_name_trgm ON members USING gin (full_name gin_trgm_ops);

-- ---------- catalog ----------
CREATE TABLE categories (
    id   UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE authors (
    id   UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);
CREATE INDEX authors_name_trgm ON authors USING gin (name gin_trgm_ops);

CREATE TABLE books (
    id             UUID PRIMARY KEY,
    isbn           VARCHAR(13) UNIQUE,
    title          TEXT        NOT NULL,
    subtitle       TEXT,
    publisher      TEXT,
    published_year SMALLINT,
    language       TEXT,
    description    TEXT,
    category_id    UUID REFERENCES categories(id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX books_title_trgm ON books USING gin (title gin_trgm_ops);

CREATE TABLE book_authors (
    book_id   UUID     NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    author_id UUID     NOT NULL REFERENCES authors(id),
    position  SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (book_id, author_id)
);
CREATE INDEX book_authors_author ON book_authors(author_id);

CREATE TABLE copies (
    id             UUID PRIMARY KEY,
    book_id        UUID        NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    barcode        TEXT        NOT NULL UNIQUE,
    shelf_location TEXT,
    status         copy_status NOT NULL DEFAULT 'available',
    acquired_on    DATE        NOT NULL DEFAULT CURRENT_DATE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX copies_book_status ON copies(book_id, status);

-- ---------- circulation ----------
CREATE TABLE loans (
    id             UUID PRIMARY KEY,
    copy_id        UUID        NOT NULL REFERENCES copies(id),
    member_id      UUID        NOT NULL REFERENCES members(id),
    issued_by      UUID        NOT NULL REFERENCES users(id),
    checked_out_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    due_date       DATE        NOT NULL,
    returned_at    TIMESTAMPTZ,
    renewals       INT         NOT NULL DEFAULT 0
);
-- A copy can have at most one open loan: last line of defence against double checkout.
CREATE UNIQUE INDEX loans_one_open_per_copy ON loans(copy_id) WHERE returned_at IS NULL;
CREATE INDEX loans_member_open ON loans(member_id) WHERE returned_at IS NULL;
CREATE INDEX loans_due_open    ON loans(due_date)  WHERE returned_at IS NULL;

CREATE TABLE reservations (
    id         UUID PRIMARY KEY,
    book_id    UUID               NOT NULL REFERENCES books(id),
    member_id  UUID               NOT NULL REFERENCES members(id),
    copy_id    UUID REFERENCES copies(id),          -- set when a copy is put on the hold shelf
    status     reservation_status NOT NULL DEFAULT 'waiting',
    created_at TIMESTAMPTZ        NOT NULL DEFAULT now(),
    ready_at   TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX reservations_one_active ON reservations(book_id, member_id)
    WHERE status IN ('waiting', 'ready');
CREATE INDEX reservations_queue ON reservations(book_id, created_at) WHERE status = 'waiting';

CREATE TABLE fines (
    id         UUID PRIMARY KEY,
    member_id  UUID          NOT NULL REFERENCES members(id),
    loan_id    UUID REFERENCES loans(id),
    amount     NUMERIC(10,2) NOT NULL CHECK (amount >= 0),
    reason     TEXT          NOT NULL,
    status     fine_status   NOT NULL DEFAULT 'unpaid',
    created_at TIMESTAMPTZ   NOT NULL DEFAULT now(),
    settled_at TIMESTAMPTZ
);
CREATE INDEX fines_member_unpaid ON fines(member_id) WHERE status = 'unpaid';

CREATE TABLE payments (
    id          UUID PRIMARY KEY,
    fine_id     UUID          NOT NULL REFERENCES fines(id),
    amount      NUMERIC(10,2) NOT NULL CHECK (amount > 0),
    received_by UUID          NOT NULL REFERENCES users(id),
    paid_at     TIMESTAMPTZ   NOT NULL DEFAULT now()
);

-- ---------- audit ----------
CREATE TABLE audit_logs (
    id         BIGSERIAL PRIMARY KEY,
    user_id    UUID REFERENCES users(id),
    action     TEXT        NOT NULL,          -- e.g. 'loan.checkout'
    entity     TEXT        NOT NULL,          -- e.g. 'loan'
    entity_id  UUID,
    details    JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX audit_logs_entity ON audit_logs(entity, entity_id);
