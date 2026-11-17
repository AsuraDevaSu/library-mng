-- Bag deposit desk: drop-off with face + bag photos, pickup by face match / tag / name.

CREATE TYPE deposit_status AS ENUM ('deposited', 'returned', 'unclaimed');

ALTER TABLE library_settings
    ADD COLUMN bag_tag_prefix TEXT NOT NULL DEFAULT 'B' CHECK (bag_tag_prefix ~ '^[A-Z]{1,3}$'),
    ADD COLUMN bag_tag_count  INT  NOT NULL DEFAULT 200 CHECK (bag_tag_count BETWEEN 1 AND 999);

CREATE TABLE bag_deposits (
    id                UUID PRIMARY KEY,
    student_name      TEXT           NOT NULL,
    member_id         UUID REFERENCES members(id),
    tag_number        TEXT           NOT NULL,             -- e.g. B-042
    -- File names inside BAG_PHOTO_DIR. face_photo is NULL when the student opted out.
    face_photo        TEXT,
    bag_photo         TEXT           NOT NULL,
    bag_color         TEXT           NOT NULL,             -- e.g. 'Navy'
    bag_color_hex     TEXT           NOT NULL,
    -- SFace embedding (128 floats). Cleared at pickup; never kept after the bag leaves.
    face_embedding    REAL[],
    photo_consent     BOOLEAN        NOT NULL,
    status            deposit_status NOT NULL DEFAULT 'deposited',
    deposited_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    deposited_by      UUID           NOT NULL REFERENCES users(id),
    returned_at       TIMESTAMPTZ,
    returned_by       UUID REFERENCES users(id),
    match_method      TEXT CHECK (match_method IN ('face', 'tag', 'name')),
    match_score       REAL,
    photos_deleted_at TIMESTAMPTZ
);

-- A physical tag can only be on one bag at a time.
CREATE UNIQUE INDEX bag_deposits_active_tag ON bag_deposits(tag_number) WHERE status = 'deposited';
CREATE INDEX bag_deposits_active ON bag_deposits(deposited_at) WHERE status = 'deposited';
CREATE INDEX bag_deposits_cleanup ON bag_deposits(returned_at) WHERE photos_deleted_at IS NULL;
