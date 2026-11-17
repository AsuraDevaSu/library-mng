//! Circulation rules. Pure functions so they are trivially unit-testable
//! and usable from the browser (e.g. to preview a due date).

use chrono::{Days, NaiveDate};
use rust_decimal::Decimal;

use crate::{
    DomainError,
    models::{MemberStatus, MemberType},
};

/// Snapshot of a member's standing, loaded inside the checkout transaction.
#[derive(Debug, Clone)]
pub struct BorrowerStanding {
    pub status: MemberStatus,
    pub expires_on: NaiveDate,
    pub active_loans: i64,
    pub unpaid_fines: Decimal,
}

/// Can this member borrow one more item today?
pub fn check_can_borrow(
    standing: &BorrowerStanding,
    policy: &MemberType,
    today: NaiveDate,
) -> Result<(), DomainError> {
    match standing.status {
        MemberStatus::Active => {}
        MemberStatus::Suspended => return Err(DomainError::MemberNotActive("suspended".into())),
        MemberStatus::Expired => return Err(DomainError::MembershipExpired),
    }
    if standing.expires_on < today {
        return Err(DomainError::MembershipExpired);
    }
    if standing.active_loans >= i64::from(policy.max_loans) {
        return Err(DomainError::LoanLimitReached(policy.max_loans));
    }
    if standing.unpaid_fines > policy.fine_block_threshold {
        return Err(DomainError::UnpaidFinesTooHigh(standing.unpaid_fines));
    }
    Ok(())
}

pub fn due_date(from: NaiveDate, policy: &MemberType) -> NaiveDate {
    from.checked_add_days(Days::new(policy.loan_days.max(0) as u64))
        .expect("due date overflow")
}

/// Renewal extends from today (not the old due date), and is refused when overdue.
pub fn renew(
    current_due: NaiveDate,
    renewals_so_far: i32,
    policy: &MemberType,
    today: NaiveDate,
) -> Result<NaiveDate, DomainError> {
    if renewals_so_far >= policy.max_renewals {
        return Err(DomainError::RenewalLimitReached(policy.max_renewals));
    }
    if current_due < today {
        return Err(DomainError::OverdueRenewal);
    }
    Ok(due_date(today, policy))
}

/// Overdue fine = days late × daily rate, capped at `max_fine`.
pub fn overdue_fine(due: NaiveDate, returned_on: NaiveDate, policy: &MemberType) -> Decimal {
    let days_late = (returned_on - due).num_days();
    if days_late <= 0 {
        return Decimal::ZERO;
    }
    (policy.daily_fine * Decimal::from(days_late)).min(policy.max_fine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;
    use uuid::Uuid;

    fn policy() -> MemberType {
        MemberType {
            id: Uuid::nil(),
            name: "student".into(),
            max_loans: 3,
            loan_days: 14,
            max_renewals: 2,
            daily_fine: dec!(0.25),
            max_fine: dec!(5.00),
            fine_block_threshold: dec!(10.00),
        }
    }

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn standing() -> BorrowerStanding {
        BorrowerStanding {
            status: MemberStatus::Active,
            expires_on: d("2027-12-31"),
            active_loans: 0,
            unpaid_fines: Decimal::ZERO,
        }
    }

    #[test]
    fn active_member_can_borrow() {
        assert!(check_can_borrow(&standing(), &policy(), d("2026-11-17")).is_ok());
    }

    #[test]
    fn loan_limit() {
        let s = BorrowerStanding { active_loans: 3, ..standing() };
        assert_eq!(
            check_can_borrow(&s, &policy(), d("2026-11-17")),
            Err(DomainError::LoanLimitReached(3))
        );
    }

    #[test]
    fn expired_card() {
        let s = BorrowerStanding { expires_on: d("2026-01-01"), ..standing() };
        assert_eq!(
            check_can_borrow(&s, &policy(), d("2026-11-17")),
            Err(DomainError::MembershipExpired)
        );
    }

    #[test]
    fn fines_block() {
        let s = BorrowerStanding { unpaid_fines: dec!(10.01), ..standing() };
        assert!(matches!(
            check_can_borrow(&s, &policy(), d("2026-11-17")),
            Err(DomainError::UnpaidFinesTooHigh(_))
        ));
    }

    #[test]
    fn due_date_adds_loan_days() {
        assert_eq!(due_date(d("2026-11-17"), &policy()), d("2026-12-01"));
    }

    #[test]
    fn fine_is_zero_when_on_time() {
        assert_eq!(overdue_fine(d("2026-12-01"), d("2026-12-01"), &policy()), Decimal::ZERO);
    }

    #[test]
    fn fine_per_day_and_capped() {
        assert_eq!(overdue_fine(d("2026-12-01"), d("2026-12-05"), &policy()), dec!(1.00));
        assert_eq!(overdue_fine(d("2026-12-01"), d("2027-03-01"), &policy()), dec!(5.00));
    }

    #[test]
    fn renewal_rules() {
        let p = policy();
        assert_eq!(renew(d("2026-12-01"), 0, &p, d("2026-11-30")), Ok(d("2026-12-14")));
        assert_eq!(
            renew(d("2026-12-01"), 2, &p, d("2026-11-30")),
            Err(DomainError::RenewalLimitReached(2))
        );
        assert_eq!(
            renew(d("2026-12-01"), 0, &p, d("2026-12-02")),
            Err(DomainError::OverdueRenewal)
        );
    }
}
