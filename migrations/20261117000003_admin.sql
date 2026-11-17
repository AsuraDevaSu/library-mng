-- Admin module: counter-staff role and library-wide settings.

-- Desk staff run the reading-room / bag-deposit counter only.
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'desk_staff';

-- Single-row settings table (id is always 1).
CREATE TABLE library_settings (
    id                       SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    library_name             TEXT        NOT NULL DEFAULT 'Library',
    opening_hours            TEXT        NOT NULL DEFAULT 'Mon–Fri 07:00–17:00',
    -- Bag deposit desk: how long face/bag photos are kept after pickup.
    bag_photo_retention_days INT         NOT NULL DEFAULT 7 CHECK (bag_photo_retention_days BETWEEN 1 AND 365),
    -- Minimum cosine similarity for a face match (SFace recommends ~0.363).
    face_match_threshold     REAL        NOT NULL DEFAULT 0.363 CHECK (face_match_threshold BETWEEN 0 AND 1),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by               UUID REFERENCES users(id)
);
INSERT INTO library_settings (id) VALUES (1);

CREATE INDEX audit_logs_created ON audit_logs(created_at DESC);
