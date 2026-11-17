//! Bag desk integration tests (real Postgres via #[sqlx::test], no face models needed).

use std::sync::Arc;

use base64::Engine as _;
use domain::{
    dto::{CreateUserRequest, DepositInput},
    models::{DepositStatus, Role},
};
use infra::{AppError, AppState, Config, repo::users, service::bags};
use sqlx::PgPool;

fn state(db: &PgPool, dir: &std::path::Path) -> AppState {
    AppState {
        db: db.clone(),
        config: Arc::new(Config {
            database_url: String::new(),
            db_max_connections: 1,
            jwt_secret: "integration-test-secret-integration-test".into(),
            jwt_ttl_minutes: 5,
            cookie_secure: false,
            cors_origins: vec![],
            bootstrap_admin: None,
            bag_photo_dir: dir.to_path_buf(),
            models_dir: "models".into(),
        }),
        faces: None,
    }
}

fn photo(rgb: [u8; 3]) -> String {
    let img = image::RgbImage::from_pixel(200, 150, image::Rgb(rgb));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(out.into_inner()))
}

fn opt_out(name: &str) -> DepositInput {
    DepositInput {
        student_name: name.into(),
        card_number: None,
        photo_consent: false,
        face_photo: None,
        bag_photo: photo([30, 45, 95]),
        bag_color: None,
    }
}

async fn staff(db: &PgPool) -> uuid::Uuid {
    users::create(
        db,
        &CreateUserRequest {
            email: "desk@lib.test".into(),
            password: "password123".into(),
            full_name: "Desk".into(),
            role: Role::DeskStaff,
        },
    )
    .await
    .unwrap()
    .id
}

fn tmpdir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("bagtest-{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[sqlx::test(migrations = "../../migrations")]
async fn tags_are_assigned_and_reused(db: PgPool) {
    let dir = tmpdir("tags");
    let st = state(&db, &dir);
    let me = staff(&db).await;

    let a = bags::deposit(&st, opt_out("Sokha"), me).await.unwrap();
    let b = bags::deposit(&st, opt_out("Dara"), me).await.unwrap();
    assert_eq!((a.tag_number.as_str(), b.tag_number.as_str()), ("B-001", "B-002"));
    assert_eq!(a.bag_color, "Navy", "colour detected from the bag photo");
    assert!(!a.has_face_photo);
    assert!(dir.join(format!("{}-bag.jpg", a.id)).exists());

    // Hand over A → B-001 is free again for the next student.
    let back = bags::hand_over(&db, a.id, "tag", None, me).await.unwrap();
    assert_eq!(back.status, DepositStatus::Returned);
    let c = bags::deposit(&st, opt_out("Vanna"), me).await.unwrap();
    assert_eq!(c.tag_number, "B-001");

    // Handing over twice is refused.
    assert!(matches!(bags::hand_over(&db, a.id, "tag", None, me).await, Err(AppError::Conflict(_))));

    // Search by name and by tag.
    assert_eq!(bags::list_active(&db, "dara").await.unwrap().len(), 1);
    assert_eq!(bags::list_active(&db, "b-001").await.unwrap()[0].student_name, "Vanna");
}

#[sqlx::test(migrations = "../../migrations")]
async fn desk_full_and_consent_rules(db: PgPool) {
    let st = state(&db, &tmpdir("full"));
    let me = staff(&db).await;
    sqlx::query("UPDATE library_settings SET bag_tag_count = 1").execute(&db).await.unwrap();

    bags::deposit(&st, opt_out("One"), me).await.unwrap();
    let err = bags::deposit(&st, opt_out("Two"), me).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(ref m) if m.contains("all bag tags")), "{err:?}");

    // Consent given but no face photo → refused.
    sqlx::query("UPDATE library_settings SET bag_tag_count = 10").execute(&db).await.unwrap();
    let mut input = opt_out("Three");
    input.photo_consent = true;
    assert!(matches!(bags::deposit(&st, input, me).await, Err(AppError::Conflict(_))));

    // Staff colour correction wins over detection.
    let mut input = opt_out("Four");
    input.bag_color = Some("red".into());
    assert_eq!(bags::deposit(&st, input, me).await.unwrap().bag_color, "Red");

    // Without models, face search reports it is unavailable.
    assert!(matches!(bags::match_face(&st, &photo([1, 2, 3])).await, Err(AppError::Conflict(_))));
}

#[sqlx::test(migrations = "../../migrations")]
async fn cleanup_deletes_old_photos_only(db: PgPool) {
    let dir = tmpdir("cleanup");
    let st = state(&db, &dir);
    let me = staff(&db).await;
    let old = bags::deposit(&st, opt_out("Old"), me).await.unwrap();
    let recent = bags::deposit(&st, opt_out("Recent"), me).await.unwrap();
    bags::hand_over(&db, old.id, "name", None, me).await.unwrap();
    bags::hand_over(&db, recent.id, "name", None, me).await.unwrap();
    sqlx::query("UPDATE bag_deposits SET returned_at = now() - interval '30 days' WHERE id = $1")
        .bind(old.id)
        .execute(&db)
        .await
        .unwrap();

    assert_eq!(bags::cleanup(&st).await.unwrap(), 1);
    assert!(!dir.join(format!("{}-bag.jpg", old.id)).exists());
    assert!(dir.join(format!("{}-bag.jpg", recent.id)).exists());
    assert!(!bags::get(&db, old.id).await.unwrap().has_bag_photo);
    assert!(matches!(bags::photo_path(&st, old.id, bags::PhotoKind::Bag).await, Err(AppError::NotFound(_))));
}
