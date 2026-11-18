//! Request/response bodies shared by the REST API, server functions and clients.

use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::models::{Role, User};

/// ISBN-10 or ISBN-13, digits only (an ISBN-10 may end in X).
pub static ISBN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\d{9}[\dX]|\d{13})$").unwrap());
/// Barcodes / card numbers: 4–32 uppercase letters, digits or dashes.
pub static CODE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z0-9-]{4,32}$").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(length(min = 1, max = 200))]
    pub full_name: String,
    pub role: Role,
}

/// Partial update of a staff account; `None` leaves the field unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 1, max = 200))]
    pub full_name: Option<String>,
    pub role: Option<Role>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MemberTypeInput {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(range(min = 0, max = 100))]
    pub max_loans: i32,
    #[validate(range(min = 1, max = 365))]
    pub loan_days: i32,
    #[validate(range(min = 0, max = 20))]
    pub max_renewals: i32,
    #[validate(custom(function = "non_negative"))]
    pub daily_fine: Decimal,
    #[validate(custom(function = "non_negative"))]
    pub max_fine: Decimal,
    #[validate(custom(function = "non_negative"))]
    pub fine_block_threshold: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SettingsInput {
    #[validate(length(min = 1, max = 200))]
    pub library_name: String,
    #[validate(length(max = 200))]
    pub opening_hours: String,
    #[validate(range(min = 1, max = 365))]
    pub bag_photo_retention_days: i32,
    #[validate(range(min = 0.0, max = 1.0))]
    pub face_match_threshold: f32,
    #[validate(regex(path = *TAG_PREFIX_RE, message = "1–3 capital letters"))]
    pub bag_tag_prefix: String,
    #[validate(range(min = 1, max = 999))]
    pub bag_tag_count: i32,
}

pub static TAG_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]{1,3}$").unwrap());

/// Colours offered for bags at the desk (name, swatch hex). The server's
/// automatic colour detection returns one of these names.
pub const BAG_COLORS: [(&str, &str); 13] = [
    ("Black", "#1f1f1f"),
    ("White", "#f4f4f4"),
    ("Grey", "#8c8c8c"),
    ("Red", "#d23b3b"),
    ("Orange", "#ee8a2a"),
    ("Brown", "#7a4e2d"),
    ("Yellow", "#f0d03c"),
    ("Green", "#3fa34d"),
    ("Teal", "#1f9e9a"),
    ("Blue", "#2f6fd6"),
    ("Navy", "#1f2f5c"),
    ("Purple", "#7d4cc2"),
    ("Pink", "#e46aa8"),
];

/// Drop-off form. Photos are `data:image/jpeg;base64,…` URLs from the desk camera.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DepositInput {
    #[validate(length(min = 1, max = 200))]
    pub student_name: String,
    /// Optional student/teacher ID card. Anyone may use the desk; the deposit is
    /// linked to a library member only if the card matches one.
    #[validate(length(max = 64))]
    pub card_number: Option<String>,
    /// Student agreed to the face photo. Without consent the bag is found by tag/name only.
    pub photo_consent: bool,
    #[validate(length(max = 6_000_000))]
    pub face_photo: Option<String>,
    #[validate(length(min = 100, max = 6_000_000))]
    pub bag_photo: String,
    /// Staff correction of the detected colour (one of `BAG_COLORS`).
    pub bag_color: Option<String>,
}

fn non_negative(v: &Decimal) -> Result<(), ValidationError> {
    if v.is_sign_negative() && !v.is_zero() {
        return Err(ValidationError::new("non_negative").with_message("must be 0 or more".into()));
    }
    if v.scale() > 2 {
        return Err(ValidationError::new("scale").with_message("at most 2 decimal places".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateBookRequest {
    #[validate(regex(path = *ISBN_RE, message = "ISBN must be 10 or 13 digits"))]
    pub isbn: Option<String>,
    #[validate(length(min = 1, max = 500))]
    pub title: String,
    #[validate(length(max = 500))]
    pub subtitle: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    #[validate(range(min = 1000, max = 2100))]
    pub published_year: Option<i16>,
    pub language: Option<String>,
    pub description: Option<String>,
    /// Category name (created if new), e.g. "Science".
    #[serde(default)]
    #[validate(length(max = 100))]
    pub category: Option<String>,
    /// Barcodes of physical copies to register with the book.
    #[validate(length(max = 200))]
    pub copy_barcodes: Vec<String>,
    pub shelf_location: Option<String>,
}

/// Edit a book's metadata (copies and files are managed separately).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateBookRequest {
    #[validate(regex(path = *ISBN_RE, message = "ISBN must be 10 or 13 digits"))]
    pub isbn: Option<String>,
    #[validate(length(min = 1, max = 500))]
    pub title: String,
    #[validate(length(max = 500))]
    pub subtitle: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    #[validate(range(min = 1000, max = 2100))]
    pub published_year: Option<i16>,
    pub language: Option<String>,
    pub description: Option<String>,
    #[validate(length(max = 100))]
    pub category: Option<String>,
}

/// Register more physical copies of a book.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddCopiesRequest {
    #[validate(length(min = 1, max = 200))]
    pub barcodes: Vec<String>,
    pub shelf_location: Option<String>,
}

/// Flat, all-text book form used by the admin web UI (empty string = not set).
/// `authors` is comma-separated; `copy_barcodes` is separated by commas,
/// spaces or new lines.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BookForm {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub authors: String,
    #[serde(default)]
    pub isbn: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub published_year: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub copy_barcodes: String,
    #[serde(default)]
    pub shelf_location: String,
}

fn opt(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Splits a list typed by staff (commas, spaces, new lines).
pub fn split_codes(s: &str) -> Vec<String> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .map(|c| c.trim().to_uppercase())
        .filter(|c| !c.is_empty())
        .collect()
}

impl BookForm {
    fn year(&self) -> Result<Option<i16>, String> {
        let y = self.published_year.trim();
        if y.is_empty() {
            return Ok(None);
        }
        y.parse().map(Some).map_err(|_| "Year must be a number like 2019.".to_string())
    }

    fn isbn(&self) -> Option<String> {
        opt(&self.isbn.replace(['-', ' '], "")).map(|i| i.to_uppercase())
    }

    fn authors(&self) -> Vec<String> {
        self.authors.split(',').filter_map(opt).collect()
    }

    pub fn to_create(&self) -> Result<CreateBookRequest, String> {
        Ok(CreateBookRequest {
            isbn: self.isbn(),
            title: self.title.trim().to_string(),
            subtitle: opt(&self.subtitle),
            authors: self.authors(),
            publisher: opt(&self.publisher),
            published_year: self.year()?,
            language: opt(&self.language),
            description: opt(&self.description),
            category: opt(&self.category),
            copy_barcodes: split_codes(&self.copy_barcodes),
            shelf_location: opt(&self.shelf_location),
        })
    }

    pub fn to_update(&self) -> Result<UpdateBookRequest, String> {
        let c = self.to_create()?;
        Ok(UpdateBookRequest {
            isbn: c.isbn,
            title: c.title,
            subtitle: c.subtitle,
            authors: c.authors,
            publisher: c.publisher,
            published_year: c.published_year,
            language: c.language,
            description: c.description,
            category: c.category,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateMemberRequest {
    pub member_type_id: Uuid,
    #[validate(regex(path = *CODE_RE, message = "card number must be 4-32 chars: A-Z, 0-9, -"))]
    pub card_number: String,
    #[validate(length(min = 1, max = 200))]
    pub full_name: String,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(max = 30))]
    pub phone: Option<String>,
    pub expires_on: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CheckoutRequest {
    #[validate(regex(path = *CODE_RE))]
    pub card_number: String,
    #[validate(regex(path = *CODE_RE))]
    pub copy_barcode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ReturnRequest {
    #[validate(regex(path = *CODE_RE))]
    pub copy_barcode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    /// Exact category name; empty = all.
    #[serde(default)]
    pub category: String,
    /// Only books that can be read online.
    #[serde(default)]
    pub online: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isbn_regex() {
        assert!(ISBN_RE.is_match("9780132350884"));
        assert!(ISBN_RE.is_match("013235088X"));
        assert!(!ISBN_RE.is_match("978-0132350884"));
    }

    #[test]
    fn member_type_money_validation() {
        let mut t = MemberTypeInput {
            name: "student".into(),
            max_loans: 3,
            loan_days: 14,
            max_renewals: 2,
            daily_fine: "0.25".parse().unwrap(),
            max_fine: "5".parse().unwrap(),
            fine_block_threshold: "10".parse().unwrap(),
        };
        assert!(t.validate().is_ok());
        t.daily_fine = "-1".parse().unwrap();
        assert!(t.validate().is_err());
        t.daily_fine = "0.001".parse().unwrap();
        assert!(t.validate().is_err());
    }

    #[test]
    fn role_hierarchy() {
        assert!(Role::Admin.satisfies(Role::Librarian));
        assert!(Role::Librarian.satisfies(Role::DeskStaff));
        assert!(!Role::DeskStaff.satisfies(Role::Librarian));
        assert!(!Role::Librarian.satisfies(Role::Admin));
    }

    #[test]
    fn login_validation() {
        let bad = LoginRequest { email: "nope".into(), password: "short".into() };
        assert!(bad.validate().is_err());
        let ok = LoginRequest { email: "a@b.co".into(), password: "longenough".into() };
        assert!(ok.validate().is_ok());
    }
}
