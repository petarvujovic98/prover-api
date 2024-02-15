use axum::{debug_handler, extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::{ApiResult, AppState};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct Status {
    #[schema(example = 100)]
    min_optimistic_tier_fee: u64,
    #[schema(example = 100)]
    min_sgx_tier_fee: u64,
    #[schema(example = 100)]
    min_pse_zkevm_tier_fee: u64,
    #[schema(example = 170)]
    max_expiry: u64,
    #[schema(example = "0x0...0")]
    prover: String,
}

#[utoipa::path(
    get,
    path = "/status",
    tag = "status",
    responses(
        (status = 200, description = "Service status", body = Status),
    )
)]
#[debug_handler(state = AppState)]
async fn get_status(
    State(AppState {
        min_optimistic_tier_fee,
        min_sgx_tier_fee,
        min_pse_zkevm_tier_fee,
        max_expiry,
        prover_address,
        ..
    }): State<AppState>,
) -> ApiResult<Status> {
    Ok(Json(Status {
        min_optimistic_tier_fee,
        min_sgx_tier_fee,
        min_pse_zkevm_tier_fee,
        max_expiry,
        prover: format!("{prover_address:#?}"),
    }))
}

#[derive(OpenApi)]
#[openapi(paths(get_status), components(schemas(Status)))]
pub struct StatusDoc;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(get_status))
}
