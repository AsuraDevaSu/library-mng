//! Admin module: staff accounts, borrowing policies, library settings,
//! dashboard numbers and the audit log. Callers must have checked `Role::Admin`.

use domain::{
    dto::{CreateUserRequest, MemberTypeInput, SettingsInput, UpdateUserRequest},
    models::{AuditEntry, DashboardStats, LibrarySettings, MemberType, Role, User},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    AppError, AppResult, auth,
    error::map_unique,
    repo::{audit, like_pattern, users},
};

const USER_COLS: &str = "id, email, full_name, role, is_active, created_at";

// ---------------- staff ----------------

pub async fn list_staff(db: &PgPool) -> AppResult<Vec<User>> {
    Ok(sqlx::query_as(&format!("SELECT {USER_COLS} FROM users ORDER BY is_active DESC, full_name")).fetch_all(db).await?)
}

pub async fn create_staff(db: &PgPool, req: &CreateUserRequest, actor: Uuid) -> AppResult<User> {
    let user = users::create(db, req).await?;
    audit::log(db, actor, "staff.create", "user", user.id, serde_json::json!({ "role": user.role, "email": user.email }))
        .await?;
    Ok(user)
}

/// Changes name / role / active flag. Refuses to leave the library without an
/// active admin, and refuses to let admins demote or disable themselves.
pub async fn update_staff(db: &PgPool, id: Uuid, req: &UpdateUserRequest, actor: Uuid) -> AppResult<User> {
    let mut tx = db.begin().await?;

    // Lock every active admin (stable order → no deadlocks) so two admins
    // can't concurrently demote each other and leave zero admins.
    let active_admins: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE role = 'admin' AND is_active ORDER BY id FOR UPDATE")
            .fetch_all(&mut *tx)
            .await?;

    let current: User = sqlx::query_as(&format!("SELECT {USER_COLS} FROM users WHERE id = $1 FOR UPDATE"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound("staff account"))?;

    let new_role = req.role.unwrap_or(current.role);
    let new_active = req.is_active.unwrap_or(current.is_active);
    let loses_admin = current.role == Role::Admin && current.is_active && (new_role != Role::Admin || !new_active);

    if loses_admin && id == actor {
        return Err(AppError::Conflict("you cannot demote or disable your own account".into()));
    }
    if loses_admin && active_admins.iter().all(|a| *a == id) {
        return Err(AppError::Conflict("there must be at least one active admin".into()));
    }

    let user: User = sqlx::query_as(&format!(
        "UPDATE users SET full_name = COALESCE($2, full_name), role = $3, is_active = $4, updated_at = now()
          WHERE id = $1 RETURNING {USER_COLS}"
    ))
    .bind(id)
    .bind(req.full_name.as_deref().map(str::trim))
    .bind(new_role)
    .bind(new_active)
    .fetch_one(&mut *tx)
    .await?;

    audit::log(
        &mut *tx,
        actor,
        "staff.update",
        "user",
        id,
        serde_json::json!({
            "role": { "from": current.role, "to": new_role },
            "active": { "from": current.is_active, "to": new_active },
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(user)
}

pub async fn reset_password(db: &PgPool, id: Uuid, new_password: &str, actor: Uuid) -> AppResult<()> {
    if !(8..=128).contains(&new_password.chars().count()) {
        return Err(AppError::Conflict("password must be 8–128 characters".into()));
    }
    let hash = auth::hash_password(new_password.to_owned()).await?;
    let updated = sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(hash)
        .execute(db)
        .await?
        .rows_affected();
    if updated == 0 {
        return Err(AppError::NotFound("staff account"));
    }
    audit::log(db, actor, "staff.reset_password", "user", id, serde_json::json!({})).await?;
    Ok(())
}

// ---------------- borrowing policies ----------------

pub async fn list_member_types(db: &PgPool) -> AppResult<Vec<MemberType>> {
    Ok(sqlx::query_as("SELECT * FROM member_types ORDER BY name").fetch_all(db).await?)
}

pub async fn create_member_type(db: &PgPool, input: &MemberTypeInput, actor: Uuid) -> AppResult<MemberType> {
    let t: MemberType = sqlx::query_as(
        "INSERT INTO member_types (id, name, max_loans, loan_days, max_renewals, daily_fine, max_fine, fine_block_threshold)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(input.name.trim().to_lowercase())
    .bind(input.max_loans)
    .bind(input.loan_days)
    .bind(input.max_renewals)
    .bind(input.daily_fine)
    .bind(input.max_fine)
    .bind(input.fine_block_threshold)
    .fetch_one(db)
    .await
    .map_err(|e| map_unique(e, "member type with this name"))?;
    audit::log(db, actor, "policy.create", "member_type", t.id, serde_json::to_value(&t).unwrap_or_default()).await?;
    Ok(t)
}

/// Policy changes apply to future loans; existing due dates are not recalculated.
pub async fn update_member_type(db: &PgPool, id: Uuid, input: &MemberTypeInput, actor: Uuid) -> AppResult<MemberType> {
    let mut tx = db.begin().await?;
    let before: MemberType = sqlx::query_as("SELECT * FROM member_types WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound("member type"))?;
    let after: MemberType = sqlx::query_as(
        "UPDATE member_types SET name = $2, max_loans = $3, loan_days = $4, max_renewals = $5,
                daily_fine = $6, max_fine = $7, fine_block_threshold = $8
          WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(input.name.trim().to_lowercase())
    .bind(input.max_loans)
    .bind(input.loan_days)
    .bind(input.max_renewals)
    .bind(input.daily_fine)
    .bind(input.max_fine)
    .bind(input.fine_block_threshold)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_unique(e, "member type with this name"))?;
    audit::log(&mut *tx, actor, "policy.update", "member_type", id, serde_json::json!({ "before": before, "after": after }))
        .await?;
    tx.commit().await?;
    Ok(after)
}

// ---------------- settings ----------------

const SETTINGS_COLS: &str = "library_name, opening_hours, bag_photo_retention_days, face_match_threshold, updated_at";

pub async fn get_settings(db: &PgPool) -> AppResult<LibrarySettings> {
    Ok(sqlx::query_as(&format!("SELECT {SETTINGS_COLS} FROM library_settings WHERE id = 1")).fetch_one(db).await?)
}

pub async fn update_settings(db: &PgPool, input: &SettingsInput, actor: Uuid) -> AppResult<LibrarySettings> {
    let s: LibrarySettings = sqlx::query_as(&format!(
        "UPDATE library_settings SET library_name = $1, opening_hours = $2, bag_photo_retention_days = $3,
                face_match_threshold = $4, updated_at = now(), updated_by = $5
          WHERE id = 1 RETURNING {SETTINGS_COLS}"
    ))
    .bind(input.library_name.trim())
    .bind(input.opening_hours.trim())
    .bind(input.bag_photo_retention_days)
    .bind(input.face_match_threshold)
    .bind(actor)
    .fetch_one(db)
    .await?;
    audit::log(db, actor, "settings.update", "settings", Uuid::nil(), serde_json::to_value(input).unwrap_or_default())
        .await?;
    Ok(s)
}

// ---------------- dashboard & audit ----------------

pub async fn dashboard(db: &PgPool) -> AppResult<DashboardStats> {
    Ok(sqlx::query_as(
        r#"
        SELECT
          (SELECT count(*) FROM books)                                                  AS titles,
          (SELECT count(*) FROM copies WHERE status <> 'withdrawn')                     AS copies,
          (SELECT count(*) FROM members WHERE status = 'active' AND expires_on >= CURRENT_DATE) AS active_members,
          (SELECT count(*) FROM loans WHERE returned_at IS NULL)                        AS open_loans,
          (SELECT count(*) FROM loans WHERE returned_at IS NULL AND due_date < CURRENT_DATE) AS overdue_loans,
          (SELECT COALESCE(sum(amount), 0) FROM fines WHERE status = 'unpaid')          AS unpaid_fines,
          (SELECT count(*) FROM users WHERE is_active)                                  AS active_staff
        "#,
    )
    .fetch_one(db)
    .await?)
}

/// Newest first. `q` matches action (e.g. `loan.`), entity, or staff name.
pub async fn audit_log(db: &PgPool, q: &str, limit: i64, offset: i64) -> AppResult<Vec<AuditEntry>> {
    let q = q.trim();
    Ok(sqlx::query_as(
        "SELECT a.id, u.full_name AS user_name, a.action, a.entity, a.entity_id, a.details::text AS details, a.created_at
           FROM audit_logs a LEFT JOIN users u ON u.id = a.user_id
          WHERE $1 = '' OR a.action ILIKE $2 OR a.entity ILIKE $2 OR u.full_name ILIKE $2
          ORDER BY a.created_at DESC, a.id DESC
          LIMIT $3 OFFSET $4",
    )
    .bind(q)
    .bind(like_pattern(q))
    .bind(limit.clamp(1, 200))
    .bind(offset.max(0))
    .fetch_all(db)
    .await?)
}
