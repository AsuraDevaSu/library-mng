//! Bag deposit desk server functions. All require `Role::DeskStaff` or higher.
//! Photo uploads use the JSON codec (base64 data URLs from the desk camera).

use domain::{
    dto::DepositInput,
    models::{BagDeposit, DeskSummary, FaceScanResult},
};
use leptos::{prelude::*, server_fn::codec::Json};
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::auth::ssr::{require, to_sfe, validate};
#[cfg(feature = "ssr")]
use domain::models::Role;

#[server]
pub async fn desk_bags(q: String) -> Result<(DeskSummary, Vec<BagDeposit>), ServerFnError> {
    let (state, _) = require(Role::DeskStaff).await?;
    let summary = infra::service::bags::summary(&state).await.map_err(to_sfe)?;
    let bags = infra::service::bags::list_active(&state.db, &q).await.map_err(to_sfe)?;
    Ok((summary, bags))
}

#[server(input = Json)]
pub async fn deposit_bag(input: DepositInput) -> Result<BagDeposit, ServerFnError> {
    let (state, me) = require(Role::DeskStaff).await?;
    validate(&input)?;
    infra::service::bags::deposit(&state, input, me.id).await.map_err(to_sfe)
}

#[server(input = Json)]
pub async fn scan_face(photo: String) -> Result<FaceScanResult, ServerFnError> {
    let (state, _) = require(Role::DeskStaff).await?;
    infra::service::bags::match_face(&state, &photo).await.map_err(to_sfe)
}

#[server]
pub async fn hand_over_bag(id: Uuid, method: String, score: Option<f32>) -> Result<BagDeposit, ServerFnError> {
    let (state, me) = require(Role::DeskStaff).await?;
    infra::service::bags::hand_over(&state.db, id, &method, score, me.id).await.map_err(to_sfe)
}
