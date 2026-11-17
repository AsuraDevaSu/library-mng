use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Business-rule violations. Mapped to HTTP 409/422 by the server.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum DomainError {
    #[error("member account is {0}")]
    MemberNotActive(String),
    #[error("member card expired")]
    MembershipExpired,
    #[error("loan limit reached ({0} books)")]
    LoanLimitReached(i32),
    #[error("unpaid fines of {0} exceed the allowed limit")]
    UnpaidFinesTooHigh(Decimal),
    #[error("renewal limit reached ({0})")]
    RenewalLimitReached(i32),
    #[error("loan is overdue and cannot be renewed")]
    OverdueRenewal,
}
