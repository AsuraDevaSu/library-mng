//! Checkout / return / renew. Each operation is one transaction; the business
//! decisions themselves live in `domain::rules`.

use chrono::{Local, NaiveDate};
use domain::{
    dto::{CheckoutRequest, ReturnRequest},
    models::{CopyStatus, Fine, Loan, Member, MemberType, ReturnReceipt},
    rules::{self, BorrowerStanding},
};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{AppError, AppResult, repo::audit, repo::members::MEMBER_COLS};

const LOAN_COLS: &str =
    "id, copy_id, member_id, issued_by, checked_out_at, due_date, returned_at, renewals";

/// "Today" in the library's timezone — set the `TZ` env var on the server.
fn today() -> NaiveDate {
    Local::now().date_naive()
}

async fn policy_for(conn: &mut PgConnection, member_type_id: Uuid) -> AppResult<MemberType> {
    Ok(sqlx::query_as("SELECT * FROM member_types WHERE id = $1")
        .bind(member_type_id)
        .fetch_one(conn)
        .await?)
}

pub async fn checkout(db: &PgPool, req: &CheckoutRequest, staff_id: Uuid) -> AppResult<Loan> {
    let mut tx = db.begin().await?;

    // Lock the member row: concurrent checkouts for the same member are serialized,
    // so the loan limit cannot be exceeded by racing requests.
    let member: Member = sqlx::query_as(&format!(
        "SELECT {MEMBER_COLS} FROM members WHERE card_number = upper($1) FOR UPDATE"
    ))
    .bind(req.card_number.trim())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("member"))?;

    let policy = policy_for(&mut tx, member.member_type_id).await?;
    let active_loans: i64 =
        sqlx::query_scalar("SELECT count(*) FROM loans WHERE member_id = $1 AND returned_at IS NULL")
            .bind(member.id)
            .fetch_one(&mut *tx)
            .await?;
    let unpaid_fines: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(sum(amount), 0) FROM fines WHERE member_id = $1 AND status = 'unpaid'",
    )
    .bind(member.id)
    .fetch_one(&mut *tx)
    .await?;

    let today = today();
    rules::check_can_borrow(
        &BorrowerStanding {
            status: member.status,
            expires_on: member.expires_on,
            active_loans,
            unpaid_fines,
        },
        &policy,
        today,
    )?;

    // Atomic claim of the copy: only succeeds if it is still available.
    let barcode = req.copy_barcode.trim().to_uppercase();
    let copy_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE copies SET status = 'on_loan' WHERE barcode = $1 AND status = 'available' RETURNING id",
    )
    .bind(&barcode)
    .fetch_optional(&mut *tx)
    .await?;
    let copy_id = match copy_id {
        Some(id) => id,
        None => {
            let status: Option<CopyStatus> =
                sqlx::query_scalar("SELECT status FROM copies WHERE barcode = $1")
                    .bind(&barcode)
                    .fetch_optional(&mut *tx)
                    .await?;
            return Err(match status {
                None => AppError::NotFound("copy"),
                Some(s) => AppError::Conflict(format!("copy is not available (status: {s:?})")),
            });
        }
    };

    let loan: Loan = sqlx::query_as(&format!(
        "INSERT INTO loans (id, copy_id, member_id, issued_by, due_date)
         VALUES ($1, $2, $3, $4, $5) RETURNING {LOAN_COLS}"
    ))
    .bind(Uuid::new_v4())
    .bind(copy_id)
    .bind(member.id)
    .bind(staff_id)
    .bind(rules::due_date(today, &policy))
    .fetch_one(&mut *tx)
    .await?;

    audit::log(
        &mut *tx,
        staff_id,
        "loan.checkout",
        "loan",
        loan.id,
        serde_json::json!({ "barcode": barcode, "member": member.card_number }),
    )
    .await?;
    tx.commit().await?;
    Ok(loan)
}

pub async fn return_copy(db: &PgPool, req: &ReturnRequest, staff_id: Uuid) -> AppResult<ReturnReceipt> {
    let mut tx = db.begin().await?;
    let barcode = req.copy_barcode.trim().to_uppercase();

    let open: Loan = sqlx::query_as(&format!(
        "SELECT {} FROM loans l JOIN copies c ON c.id = l.copy_id
          WHERE c.barcode = $1 AND l.returned_at IS NULL FOR UPDATE OF l",
        LOAN_COLS.split(", ").map(|c| format!("l.{c}")).collect::<Vec<_>>().join(", ")
    ))
    .bind(&barcode)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("open loan for this copy"))?;

    let loan: Loan = sqlx::query_as(&format!(
        "UPDATE loans SET returned_at = now() WHERE id = $1 RETURNING {LOAN_COLS}"
    ))
    .bind(open.id)
    .fetch_one(&mut *tx)
    .await?;

    let member_type_id: Uuid = sqlx::query_scalar("SELECT member_type_id FROM members WHERE id = $1")
        .bind(loan.member_id)
        .fetch_one(&mut *tx)
        .await?;
    let policy = policy_for(&mut tx, member_type_id).await?;

    let amount = rules::overdue_fine(loan.due_date, today(), &policy);
    let fine = if amount > Decimal::ZERO {
        let fine: Fine = sqlx::query_as(
            "INSERT INTO fines (id, member_id, loan_id, amount, reason) VALUES ($1, $2, $3, $4, $5)
             RETURNING id, member_id, loan_id, amount, reason, status, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(loan.member_id)
        .bind(loan.id)
        .bind(amount)
        .bind(format!("Overdue: due {}", loan.due_date))
        .fetch_one(&mut *tx)
        .await?;
        Some(fine)
    } else {
        None
    };

    // TODO(holds): if a reservation is waiting for this book, set 'on_hold_shelf' instead.
    sqlx::query("UPDATE copies SET status = 'available' WHERE id = $1")
        .bind(loan.copy_id)
        .execute(&mut *tx)
        .await?;

    audit::log(
        &mut *tx,
        staff_id,
        "loan.return",
        "loan",
        loan.id,
        serde_json::json!({ "barcode": barcode, "fine": amount }),
    )
    .await?;
    tx.commit().await?;
    Ok(ReturnReceipt { loan, fine })
}

pub async fn renew(db: &PgPool, loan_id: Uuid, staff_id: Uuid) -> AppResult<Loan> {
    let mut tx = db.begin().await?;
    let loan: Loan = sqlx::query_as(&format!(
        "SELECT {LOAN_COLS} FROM loans WHERE id = $1 AND returned_at IS NULL FOR UPDATE"
    ))
    .bind(loan_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("open loan"))?;

    let member_type_id: Uuid = sqlx::query_scalar("SELECT member_type_id FROM members WHERE id = $1")
        .bind(loan.member_id)
        .fetch_one(&mut *tx)
        .await?;
    let policy = policy_for(&mut tx, member_type_id).await?;
    // TODO(holds): refuse renewal when another member is waiting for this book.
    let new_due = rules::renew(loan.due_date, loan.renewals, &policy, today())?;

    let loan: Loan = sqlx::query_as(&format!(
        "UPDATE loans SET due_date = $2, renewals = renewals + 1 WHERE id = $1 RETURNING {LOAN_COLS}"
    ))
    .bind(loan.id)
    .bind(new_due)
    .fetch_one(&mut *tx)
    .await?;

    audit::log(&mut *tx, staff_id, "loan.renew", "loan", loan.id, serde_json::json!({ "due": new_due }))
        .await?;
    tx.commit().await?;
    Ok(loan)
}
