use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- enums (mapped to Postgres enum types) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "user_role", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Librarian,
    /// Reading-room / bag-deposit counter only.
    DeskStaff,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Admin, Role::Librarian, Role::DeskStaff];

    fn rank(self) -> u8 {
        match self {
            Role::Admin => 3,
            Role::Librarian => 2,
            Role::DeskStaff => 1,
        }
    }

    /// Roles are hierarchical: admin ⊇ librarian ⊇ desk staff.
    pub fn satisfies(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }

    /// Value used in forms / JSON (`snake_case`).
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Librarian => "librarian",
            Role::DeskStaff => "desk_staff",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Librarian => "Librarian",
            Role::DeskStaff => "Desk staff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "member_status", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Suspended,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "copy_status", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum CopyStatus {
    Available,
    OnLoan,
    OnHoldShelf,
    Lost,
    Damaged,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "fine_status", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum FineStatus {
    Unpaid,
    Paid,
    Waived,
}

// ---------- entities ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct MemberType {
    pub id: Uuid,
    pub name: String,
    pub max_loans: i32,
    pub loan_days: i32,
    pub max_renewals: i32,
    pub daily_fine: Decimal,
    pub max_fine: Decimal,
    /// Members owing more than this cannot borrow.
    pub fine_block_threshold: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Member {
    pub id: Uuid,
    pub member_type_id: Uuid,
    pub card_number: String,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub status: MemberStatus,
    pub expires_on: NaiveDate,
    pub created_at: DateTime<Utc>,
}

/// Row shown in catalog lists / search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct BookSummary {
    pub id: Uuid,
    pub title: String,
    pub isbn: Option<String>,
    /// Comma-separated author names.
    pub authors: String,
    pub published_year: Option<i16>,
    pub category: Option<String>,
    pub total_copies: i64,
    pub available_copies: i64,
    pub has_cover: bool,
    /// An e-book (PDF) is uploaded, so the book can be read online.
    pub has_ebook: bool,
    /// Changes whenever the book or its files change; used to bust cover caches.
    pub updated_at: DateTime<Utc>,
}

impl BookSummary {
    pub fn cover_url(&self) -> Option<String> {
        self.has_cover.then(|| cover_url(self.id, self.updated_at))
    }
}

/// Public URL of a book cover (`v` busts browser caches after a new upload).
pub fn cover_url(id: Uuid, updated_at: DateTime<Utc>) -> String {
    format!("/api/v1/books/{id}/cover?v={}", updated_at.timestamp())
}

/// Public URL of a book's PDF (served inline, supports range requests).
pub fn ebook_url(id: Uuid) -> String {
    format!("/api/v1/books/{id}/ebook")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Book {
    pub id: Uuid,
    pub isbn: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i16>,
    pub language: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    /// Comma-separated author names, in order.
    pub authors: String,
    pub has_cover: bool,
    /// Size of the uploaded PDF; `None` = not readable online.
    pub ebook_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Book {
    pub fn cover_url(&self) -> Option<String> {
        self.has_cover.then(|| cover_url(self.id, self.updated_at))
    }
    pub fn has_ebook(&self) -> bool {
        self.ebook_bytes.is_some()
    }
}

/// Book page: metadata plus its physical copies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetail {
    #[serde(flatten)]
    pub book: Book,
    pub copies: Vec<Copy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Copy {
    pub id: Uuid,
    pub book_id: Uuid,
    pub barcode: String,
    pub shelf_location: Option<String>,
    pub status: CopyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Loan {
    pub id: Uuid,
    pub copy_id: Uuid,
    pub member_id: Uuid,
    pub issued_by: Uuid,
    pub checked_out_at: DateTime<Utc>,
    pub due_date: NaiveDate,
    pub returned_at: Option<DateTime<Utc>>,
    pub renewals: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Fine {
    pub id: Uuid,
    pub member_id: Uuid,
    pub loan_id: Option<Uuid>,
    pub amount: Decimal,
    pub reason: String,
    pub status: FineStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct LibrarySettings {
    pub library_name: String,
    pub opening_hours: String,
    pub bag_photo_retention_days: i32,
    pub face_match_threshold: f32,
    pub bag_tag_prefix: String,
    pub bag_tag_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Public facts for the website (no personal data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct SiteInfo {
    pub library_name: String,
    pub opening_hours: String,
    pub titles: i64,
    pub copies: i64,
    pub available_copies: i64,
    pub members: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct DashboardStats {
    pub titles: i64,
    pub copies: i64,
    pub active_members: i64,
    pub open_loans: i64,
    pub overdue_loans: i64,
    pub unpaid_fines: Decimal,
    pub active_staff: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AuditEntry {
    pub id: i64,
    pub user_name: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    /// JSON text.
    pub details: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "deposit_status", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum DepositStatus {
    Deposited,
    Returned,
    Unclaimed,
}

/// A bag at (or previously at) the deposit desk. Photos are fetched separately
/// from `/api/v1/bags/{id}/photo/{face|bag}` (staff only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct BagDeposit {
    pub id: Uuid,
    pub student_name: String,
    /// Student/teacher ID card given at drop-off (any card, not only library members).
    pub card_number: Option<String>,
    pub tag_number: String,
    pub has_face_photo: bool,
    pub has_bag_photo: bool,
    pub bag_color: String,
    pub bag_color_hex: String,
    pub photo_consent: bool,
    pub status: DepositStatus,
    pub deposited_at: DateTime<Utc>,
    /// "HH:MM" in the library's timezone, filled in by the server.
    #[cfg_attr(feature = "sqlx", sqlx(default))]
    pub deposited_local: String,
    pub deposited_by_name: Option<String>,
    pub returned_at: Option<DateTime<Utc>>,
    pub match_method: Option<String>,
    pub match_score: Option<f32>,
}

/// A pickup candidate from a face scan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BagMatch {
    pub deposit: BagDeposit,
    /// Cosine similarity, 0–1.
    pub score: f32,
    /// Score is at or above the configured face-match threshold.
    pub confident: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceScanResult {
    pub faces_found: usize,
    pub matches: Vec<BagMatch>,
}

/// Tags in use vs. available, for the desk header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeskSummary {
    pub in_use: i64,
    pub capacity: i32,
    /// Face models are installed on the server.
    pub face_search: bool,
}

/// Result of returning a copy: the closed loan and an overdue fine, if any.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnReceipt {
    pub loan: Loan,
    pub fine: Option<Fine>,
}
