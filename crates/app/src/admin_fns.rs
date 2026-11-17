//! Admin server functions. Each one checks `Role::Admin` on the server.

use domain::{
    dto::{CreateUserRequest, MemberTypeInput, SettingsInput},
    models::{AuditEntry, DashboardStats, LibrarySettings, MemberType, Role, User},
};
use leptos::prelude::*;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::auth::ssr::{require, to_sfe, validate};

#[server]
pub async fn admin_dashboard() -> Result<(DashboardStats, Vec<AuditEntry>), ServerFnError> {
    let (state, _) = require(Role::Admin).await?;
    let stats = infra::service::admin::dashboard(&state.db).await.map_err(to_sfe)?;
    let recent = infra::service::admin::audit_log(&state.db, "", 8, 0).await.map_err(to_sfe)?;
    Ok((stats, recent))
}

// ---------- staff ----------

#[server]
pub async fn list_staff() -> Result<Vec<User>, ServerFnError> {
    let (state, _) = require(Role::Admin).await?;
    infra::service::admin::list_staff(&state.db).await.map_err(to_sfe)
}

#[server]
pub async fn create_staff(input: CreateUserRequest) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    validate(&input)?;
    infra::service::admin::create_staff(&state.db, &input, me.id).await.map_err(to_sfe)?;
    Ok(())
}

#[server]
pub async fn update_staff_role(id: Uuid, role: Role) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    let req = domain::dto::UpdateUserRequest { role: Some(role), ..Default::default() };
    infra::service::admin::update_staff(&state.db, id, &req, me.id).await.map_err(to_sfe)?;
    Ok(())
}

#[server]
pub async fn set_staff_active(id: Uuid, active: bool) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    let req = domain::dto::UpdateUserRequest { is_active: Some(active), ..Default::default() };
    infra::service::admin::update_staff(&state.db, id, &req, me.id).await.map_err(to_sfe)?;
    Ok(())
}

#[server]
pub async fn reset_staff_password(id: Uuid, password: String) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    infra::service::admin::reset_password(&state.db, id, &password, me.id).await.map_err(to_sfe)
}

// ---------- borrowing policies ----------

#[server]
pub async fn list_member_types() -> Result<Vec<MemberType>, ServerFnError> {
    let (state, _) = require(Role::Admin).await?;
    infra::service::admin::list_member_types(&state.db).await.map_err(to_sfe)
}

#[server]
pub async fn create_member_type(input: MemberTypeInput) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    validate(&input)?;
    infra::service::admin::create_member_type(&state.db, &input, me.id).await.map_err(to_sfe)?;
    Ok(())
}

#[server]
pub async fn update_member_type(id: Uuid, input: MemberTypeInput) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    validate(&input)?;
    infra::service::admin::update_member_type(&state.db, id, &input, me.id).await.map_err(to_sfe)?;
    Ok(())
}

// ---------- settings ----------

#[server]
pub async fn get_settings() -> Result<LibrarySettings, ServerFnError> {
    let (state, _) = require(Role::Admin).await?;
    infra::service::admin::get_settings(&state.db).await.map_err(to_sfe)
}

#[server]
pub async fn save_settings(input: SettingsInput) -> Result<(), ServerFnError> {
    let (state, me) = require(Role::Admin).await?;
    validate(&input)?;
    infra::service::admin::update_settings(&state.db, &input, me.id).await.map_err(to_sfe)?;
    Ok(())
}

// ---------- audit ----------

pub const AUDIT_PAGE_SIZE: i64 = 50;

#[server]
pub async fn audit_log(q: String, page: i64) -> Result<Vec<AuditEntry>, ServerFnError> {
    let (state, _) = require(Role::Admin).await?;
    infra::service::admin::audit_log(&state.db, &q, AUDIT_PAGE_SIZE, page.max(0) * AUDIT_PAGE_SIZE)
        .await
        .map_err(to_sfe)
}
