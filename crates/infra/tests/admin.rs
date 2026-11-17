//! Integration tests against a real Postgres. `#[sqlx::test]` creates a fresh
//! database per test (needs DATABASE_URL pointing at a server where the user can CREATE DATABASE).

use std::sync::Arc;

use domain::{
    dto::{CreateUserRequest, UpdateUserRequest},
    models::{Role, User},
};
use infra::{AppError, AppState, Config, auth, repo::users, service::{admin, session}};
use sqlx::PgPool;

async fn staff(db: &PgPool, email: &str, role: Role) -> User {
    users::create(
        db,
        &CreateUserRequest {
            email: email.into(),
            password: "password123".into(),
            full_name: email.into(),
            role,
        },
    )
    .await
    .unwrap()
}

fn set_role(role: Role) -> UpdateUserRequest {
    UpdateUserRequest { role: Some(role), ..Default::default() }
}

fn disable() -> UpdateUserRequest {
    UpdateUserRequest { is_active: Some(false), ..Default::default() }
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_cannot_demote_or_disable_self(db: PgPool) {
    let a = staff(&db, "a@lib.test", Role::Admin).await;
    let _b = staff(&db, "b@lib.test", Role::Admin).await;

    let err = admin::update_staff(&db, a.id, &set_role(Role::Librarian), a.id).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
    let err = admin::update_staff(&db, a.id, &disable(), a.id).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn last_active_admin_is_protected(db: PgPool) {
    let a = staff(&db, "a@lib.test", Role::Admin).await;
    let b = staff(&db, "b@lib.test", Role::Admin).await;

    // With two admins, one may demote the other…
    let demoted = admin::update_staff(&db, a.id, &set_role(Role::Librarian), b.id).await.unwrap();
    assert_eq!(demoted.role, Role::Librarian);

    // …but now b is the only admin, so nobody may remove b.
    let err = admin::update_staff(&db, b.id, &disable(), a.id).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(ref m) if m.contains("at least one active admin")), "{err:?}");

    // Non-admin changes are unaffected.
    let d = staff(&db, "d@lib.test", Role::DeskStaff).await;
    let d = admin::update_staff(&db, d.id, &disable(), b.id).await.unwrap();
    assert!(!d.is_active);

    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_logs ORDER BY id").fetch_all(&db).await.unwrap();
    assert_eq!(actions, ["staff.update", "staff.update"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn disabled_or_demoted_user_takes_effect_immediately(db: PgPool) {
    let state = AppState {
        db: db.clone(),
        config: Arc::new(Config {
            database_url: String::new(),
            db_max_connections: 1,
            jwt_secret: "integration-test-secret-integration-test".into(),
            jwt_ttl_minutes: 5,
            cookie_secure: false,
            cors_origins: vec![],
            bootstrap_admin: None,
            bag_photo_dir: std::env::temp_dir(),
            models_dir: "models".into(),
        }),
        faces: None,
    };
    let boss = staff(&db, "boss@lib.test", Role::Admin).await;
    let lib = staff(&db, "lib@lib.test", Role::Librarian).await;
    let token = auth::issue_token(lib.id, lib.role, &state.config.jwt_secret, 5).unwrap();

    // Role comes from the DB, not from the (older) token.
    admin::update_staff(&db, lib.id, &set_role(Role::DeskStaff), boss.id).await.unwrap();
    assert_eq!(session::user_from_token(&state, &token).await.unwrap().role, Role::DeskStaff);

    admin::update_staff(&db, lib.id, &disable(), boss.id).await.unwrap();
    assert!(matches!(session::user_from_token(&state, &token).await, Err(AppError::Unauthorized)));
    assert!(matches!(session::login(&state, "lib@lib.test", "password123").await, Err(AppError::Unauthorized)));
}

#[sqlx::test(migrations = "../../migrations")]
async fn password_reset_changes_login(db: PgPool) {
    let boss = staff(&db, "boss@lib.test", Role::Admin).await;
    let lib = staff(&db, "lib@lib.test", Role::Librarian).await;
    admin::reset_password(&db, lib.id, "new-password-1", boss.id).await.unwrap();

    let found = users::find_with_hash_by_email(&db, "LIB@lib.test").await.unwrap().unwrap();
    assert!(auth::verify_password("new-password-1".into(), found.password_hash.clone()).await.unwrap());
    assert!(!auth::verify_password("password123".into(), found.password_hash).await.unwrap());
}
