use sqlx::PgExecutor;
use uuid::Uuid;

/// Records a staff action. Call with the same transaction as the change itself.
pub async fn log<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    action: &str,
    entity: &str,
    entity_id: Uuid,
    details: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, entity, entity_id, details) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(action)
    .bind(entity)
    .bind(entity_id)
    .bind(details)
    .execute(db)
    .await?;
    Ok(())
}
