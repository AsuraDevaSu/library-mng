use domain::{dto::CreateMemberRequest, models::Member};
use sqlx::PgPool;
use uuid::Uuid;

use super::{audit, like_pattern};
use crate::{AppResult, error::map_unique};

pub(crate) const MEMBER_COLS: &str =
    "id, member_type_id, card_number, full_name, email, phone, status, expires_on, created_at";

pub async fn create(db: &PgPool, req: &CreateMemberRequest, staff_id: Uuid) -> AppResult<Member> {
    let mut tx = db.begin().await?;
    let member: Member = sqlx::query_as(&format!(
        "INSERT INTO members (id, member_type_id, card_number, full_name, email, phone, expires_on)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {MEMBER_COLS}"
    ))
    .bind(Uuid::new_v4())
    .bind(req.member_type_id)
    .bind(req.card_number.trim().to_uppercase())
    .bind(req.full_name.trim())
    .bind(req.email.as_deref().map(|e| e.trim().to_lowercase()))
    .bind(&req.phone)
    .bind(req.expires_on)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_unique(e, "member with this card number or email"))?;

    audit::log(&mut *tx, staff_id, "member.create", "member", member.id, serde_json::json!({})).await?;
    tx.commit().await?;
    Ok(member)
}

/// Search by name (substring) or exact card number.
pub async fn search(db: &PgPool, q: &str, limit: i64, offset: i64) -> AppResult<Vec<Member>> {
    let q = q.trim();
    Ok(sqlx::query_as(&format!(
        "SELECT {MEMBER_COLS} FROM members
          WHERE $1 = '' OR card_number = upper($1) OR full_name ILIKE $2
          ORDER BY full_name LIMIT $3 OFFSET $4"
    ))
    .bind(q)
    .bind(like_pattern(q))
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?)
}
