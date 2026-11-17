//! Bag deposit desk (ដាក់ផ្ញើកាតាប).
//!
//! Drop-off: name + face photo + bag photo → face embedding, bag colour, next free tag.
//! Pickup: a face scan is compared only against bags currently at the desk; staff
//! confirm with the stored photos. Tag/name search is always available as a fallback.
//!
//! Privacy: the embedding is erased at pickup; photos are deleted
//! `bag_photo_retention_days` after pickup by [`cleanup`]. Pickup scans are never stored.

use std::path::PathBuf;

use base64::Engine as _;
use domain::{
    dto::{BAG_COLORS, DepositInput},
    models::{BagDeposit, BagMatch, DeskSummary, FaceScanResult},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppError, AppResult, AppState, repo::audit, repo::like_pattern, service::admin};

const SELECT: &str = r#"
    SELECT d.id, d.student_name, d.tag_number,
           (d.face_photo IS NOT NULL AND d.photos_deleted_at IS NULL) AS has_face_photo,
           (d.photos_deleted_at IS NULL)                            AS has_bag_photo,
           d.bag_color, d.bag_color_hex, d.photo_consent, d.status, d.deposited_at,
           u.full_name AS deposited_by_name, d.returned_at, d.match_method, d.match_score
      FROM bag_deposits d LEFT JOIN users u ON u.id = d.deposited_by
"#;

/// Max stored photo size (longest side, px). Re-encoding also strips EXIF/GPS.
const PHOTO_MAX_SIDE: u32 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoKind {
    Face,
    Bag,
}

impl std::str::FromStr for PhotoKind {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "face" => Ok(PhotoKind::Face),
            "bag" => Ok(PhotoKind::Bag),
            _ => Err(AppError::NotFound("photo")),
        }
    }
}

fn decode_data_url(data: &str) -> AppResult<Vec<u8>> {
    let b64 = data.split_once(";base64,").map(|(_, b)| b).unwrap_or(data);
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| AppError::Conflict("photo is not a valid image".into()))
}

/// Runs CPU-heavy image work off the async executor.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> AppResult<T> {
    tokio::task::spawn_blocking(f).await.map_err(|e| AppError::Internal(e.to_string()))
}

fn bad_photo(what: &str) -> impl Fn(vision::VisionError) -> AppError + '_ {
    move |_| AppError::Conflict(format!("the {what} photo could not be read, please retake it"))
}

pub async fn summary(state: &AppState) -> AppResult<DeskSummary> {
    let settings = admin::get_settings(&state.db).await?;
    let in_use: i64 = sqlx::query_scalar("SELECT count(*) FROM bag_deposits WHERE status = 'deposited'")
        .fetch_one(&state.db)
        .await?;
    Ok(DeskSummary { in_use, capacity: settings.bag_tag_count, face_search: state.faces.is_some() })
}

/// Fills `deposited_local` ("HH:MM", server timezone = library timezone via TZ).
fn localize(mut d: BagDeposit) -> BagDeposit {
    d.deposited_local = d.deposited_at.with_timezone(&chrono::Local).format("%H:%M").to_string();
    d
}

pub async fn deposit(state: &AppState, input: DepositInput, staff: Uuid) -> AppResult<BagDeposit> {
    let db = &state.db;
    let id = Uuid::new_v4();

    // ---- bag photo + colour
    let bag_raw = decode_data_url(&input.bag_photo)?;
    let bag_color = input.bag_color.clone().filter(|c| !c.is_empty());
    let (bag_jpeg, detected) = blocking(move || {
        let jpeg = vision::normalize_jpeg(&bag_raw, PHOTO_MAX_SIDE)?;
        let color = vision::color::dominant_color(&jpeg)?;
        Ok::<_, vision::VisionError>((jpeg, color))
    })
    .await?
    .map_err(bad_photo("bag"))?;
    let (color_name, color_hex) = match bag_color {
        Some(name) => {
            let (n, hex) = BAG_COLORS
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&name))
                .ok_or_else(|| AppError::Conflict(format!("unknown colour {name}")))?;
            (n.to_string(), hex.to_string())
        }
        None => detected,
    };

    // ---- face photo + embedding (only with consent)
    let (face_jpeg, embedding) = match (input.photo_consent, &input.face_photo) {
        (false, _) => (None, None),
        (true, None) => return Err(AppError::Conflict("take the student's face photo first".into())),
        (true, Some(data)) => {
            let raw = decode_data_url(data)?;
            let engine = state.faces.clone();
            let (jpeg, analysis) = blocking(move || {
                let jpeg = vision::normalize_jpeg(&raw, PHOTO_MAX_SIDE)?;
                let analysis = engine.map(|e| e.analyze(&jpeg)).transpose()?;
                Ok::<_, vision::VisionError>((jpeg, analysis))
            })
            .await?
            .map_err(bad_photo("face"))?;
            let embedding = match analysis {
                // Models not installed: keep the photo for visual checks, no face search.
                None => None,
                Some(a) if a.embedding.is_none() => {
                    return Err(AppError::Conflict(
                        "no face found in the face photo. Ask the student to face the camera and retake it".into(),
                    ));
                }
                Some(a) => a.embedding,
            };
            (Some(jpeg), embedding)
        }
    };

    // ---- optional member card
    let member_id: Option<Uuid> = match input.card_number.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        None => None,
        Some(card) => Some(
            sqlx::query_scalar("SELECT id FROM members WHERE card_number = upper($1)")
                .bind(card)
                .fetch_optional(db)
                .await?
                .ok_or(AppError::NotFound("member with this card"))?,
        ),
    };

    // ---- write photos, then the row (files are removed again if the insert fails)
    let dir = state.config.bag_photo_dir.clone();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(e.to_string()))?;
    let bag_file = format!("{id}-bag.jpg");
    let face_file = face_jpeg.as_ref().map(|_| format!("{id}-face.jpg"));
    tokio::fs::write(dir.join(&bag_file), &bag_jpeg).await.map_err(|e| AppError::Internal(e.to_string()))?;
    if let (Some(f), Some(bytes)) = (&face_file, &face_jpeg) {
        tokio::fs::write(dir.join(f), bytes).await.map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let result = insert_with_free_tag(db, id, &input, member_id, face_file.as_deref(), &bag_file, &color_name, &color_hex, embedding, staff).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(dir.join(&bag_file)).await;
        if let Some(f) = &face_file {
            let _ = tokio::fs::remove_file(dir.join(f)).await;
        }
    }
    let tag = result?;
    audit::log(db, staff, "bag.deposit", "bag_deposit", id, serde_json::json!({ "tag": tag, "consent": input.photo_consent }))
        .await?;
    get(db, id).await
}

/// Picks the lowest free tag and inserts; retries if another desk took the same tag.
#[allow(clippy::too_many_arguments)]
async fn insert_with_free_tag(
    db: &PgPool,
    id: Uuid,
    input: &DepositInput,
    member_id: Option<Uuid>,
    face_file: Option<&str>,
    bag_file: &str,
    color: &str,
    color_hex: &str,
    embedding: Option<Vec<f32>>,
    staff: Uuid,
) -> AppResult<String> {
    for _ in 0..5 {
        let tag: Option<String> = sqlx::query_scalar(
            r#"
            SELECT s.bag_tag_prefix || '-' || lpad(n::text, 3, '0')
              FROM library_settings s, generate_series(1, s.bag_tag_count) n
             WHERE s.id = 1
               AND s.bag_tag_prefix || '-' || lpad(n::text, 3, '0') NOT IN
                   (SELECT tag_number FROM bag_deposits WHERE status = 'deposited')
             ORDER BY n LIMIT 1
            "#,
        )
        .fetch_optional(db)
        .await?;
        let Some(tag) = tag else {
            return Err(AppError::Conflict("all bag tags are in use".into()));
        };
        let res = sqlx::query(
            "INSERT INTO bag_deposits (id, student_name, member_id, tag_number, face_photo, bag_photo,
                                       bag_color, bag_color_hex, face_embedding, photo_consent, deposited_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(input.student_name.trim())
        .bind(member_id)
        .bind(&tag)
        .bind(face_file)
        .bind(bag_file)
        .bind(color)
        .bind(color_hex)
        .bind(&embedding)
        .bind(input.photo_consent)
        .bind(staff)
        .execute(db)
        .await;
        match res {
            Ok(_) => return Ok(tag),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::Conflict("the desk is busy, please try again".into()))
}

pub async fn get(db: &PgPool, id: Uuid) -> AppResult<BagDeposit> {
    sqlx::query_as(&format!("{SELECT} WHERE d.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .map(localize)
        .ok_or(AppError::NotFound("bag deposit"))
}

/// Bags currently at the desk, optionally filtered by name or exact tag.
pub async fn list_active(db: &PgPool, q: &str) -> AppResult<Vec<BagDeposit>> {
    let q = q.trim();
    Ok(sqlx::query_as(&format!(
        "{SELECT} WHERE d.status = 'deposited'
            AND ($1 = '' OR d.tag_number = upper($1) OR d.student_name ILIKE $2)
          ORDER BY d.deposited_at DESC LIMIT 200"
    ))
    .bind(q)
    .bind(like_pattern(q))
    .fetch_all(db)
    .await?
    .into_iter()
    .map(localize)
    .collect())
}

/// Compares a pickup face scan with every bag currently at the desk. Top 3 matches.
pub async fn match_face(state: &AppState, photo: &str) -> AppResult<FaceScanResult> {
    let engine = state
        .faces
        .clone()
        .ok_or_else(|| AppError::Conflict("face matching is not installed. Search by tag or name instead".into()))?;
    let raw = decode_data_url(photo)?;
    let analysis = blocking(move || engine.analyze(&raw)).await?.map_err(bad_photo("face"))?;
    let Some(probe) = analysis.embedding else {
        return Ok(FaceScanResult { faces_found: 0, matches: vec![] });
    };

    let rows: Vec<(Uuid, Vec<f32>)> = sqlx::query_as(
        "SELECT id, face_embedding FROM bag_deposits WHERE status = 'deposited' AND face_embedding IS NOT NULL",
    )
    .fetch_all(&state.db)
    .await?;
    let mut scored: Vec<(Uuid, f32)> = rows.into_iter().map(|(id, e)| (id, vision::cosine(&probe, &e))).collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(3);

    let threshold = admin::get_settings(&state.db).await?.face_match_threshold;
    let mut matches = Vec::with_capacity(scored.len());
    for (id, score) in scored {
        let deposit = get(&state.db, id).await?;
        matches.push(BagMatch { deposit, score: score.max(0.0), confident: score >= threshold });
    }
    Ok(FaceScanResult { faces_found: analysis.faces_found, matches })
}

/// Marks the bag as returned and erases the face embedding.
pub async fn hand_over(db: &PgPool, id: Uuid, method: &str, score: Option<f32>, staff: Uuid) -> AppResult<BagDeposit> {
    if !matches!(method, "face" | "tag" | "name") {
        return Err(AppError::Conflict("unknown pickup method".into()));
    }
    let updated = sqlx::query(
        "UPDATE bag_deposits
            SET status = 'returned', returned_at = now(), returned_by = $2,
                match_method = $3, match_score = $4, face_embedding = NULL
          WHERE id = $1 AND status = 'deposited'",
    )
    .bind(id)
    .bind(staff)
    .bind(method)
    .bind(score)
    .execute(db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::Conflict("this bag was already handed over".into()));
    }
    audit::log(db, staff, "bag.hand_over", "bag_deposit", id, serde_json::json!({ "method": method, "score": score }))
        .await?;
    get(db, id).await
}

/// Path of a stored photo, if it still exists.
pub async fn photo_path(state: &AppState, id: Uuid, kind: PhotoKind) -> AppResult<PathBuf> {
    let col = match kind {
        PhotoKind::Face => "face_photo",
        PhotoKind::Bag => "bag_photo",
    };
    let file: Option<String> = sqlx::query_scalar(&format!(
        "SELECT {col} FROM bag_deposits WHERE id = $1 AND photos_deleted_at IS NULL"
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    let file = file.ok_or(AppError::NotFound("photo"))?;
    Ok(state.config.bag_photo_dir.join(file))
}

/// Deletes photos of bags returned more than `bag_photo_retention_days` ago.
/// Returns how many deposits were cleaned.
pub async fn cleanup(state: &AppState) -> AppResult<usize> {
    let rows: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
        "SELECT d.id, d.face_photo, d.bag_photo
           FROM bag_deposits d, library_settings s
          WHERE s.id = 1 AND d.status <> 'deposited' AND d.photos_deleted_at IS NULL
            AND d.returned_at < now() - make_interval(days => s.bag_photo_retention_days)",
    )
    .fetch_all(&state.db)
    .await?;
    for (id, face, bag) in &rows {
        for f in face.iter().chain(std::iter::once(bag)) {
            let _ = tokio::fs::remove_file(state.config.bag_photo_dir.join(f)).await;
        }
        sqlx::query("UPDATE bag_deposits SET photos_deleted_at = now(), face_embedding = NULL WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    Ok(rows.len())
}
