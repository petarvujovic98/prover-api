use std::ops::Add;

use axum::{debug_handler, extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::Utc;
use ethereum_types::{H160, H256};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::{OpenApi, ToSchema};

use crate::{ApiResult, AppState, Tier};

#[derive(Debug, Deserialize, ToSchema)]
struct TierFee {
    #[schema(example = 0)]
    tier: Tier,
    #[schema(example = 100)]
    fee: u64,
}

impl ToString for TierFee {
    fn to_string(&self) -> String {
        format!("{{tier: {}, fee: {}}}", self.tier.to_string(), self.fee)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CreateAssignmentRequestBody {
    #[schema(example = "0x0...0", value_type = String)]
    fee_token: H160,
    #[schema(example = json!([{"tier": 0, "fee": 100_000}]))]
    tier_fees: Vec<TierFee>,
    #[schema(example = 100_000)]
    expiry: u64,
    #[schema(example = "0x0...0", value_type = String)]
    tx_list_hash: H256,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ProposeBlockResponse {
    #[schema(example = json!([0, 1, 2, 3]))]
    signed_payload: Vec<u8>,
    #[schema(example = "0x0...0", value_type = String)]
    prover: H160,
    #[schema(example = 170)]
    max_block_id: u64,
    #[schema(example = 10)]
    max_proposed_in: u64,
}

impl Serialize for ProposeBlockResponse {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("ProposeBlockResponse", 4)?;
        state.serialize_field("signedPayload", &self.signed_payload)?;
        state.serialize_field("prover", &self.prover)?;
        state.serialize_field("maxBlockId", &self.max_block_id)?;
        state.serialize_field("maxProposedIn", &self.max_proposed_in)?;
        state.end()
    }
}

#[utoipa::path(
    post,
    path = "/assignment",
    tag = "assignment",
    responses(
        (status = 200, description = "Create a proof assignment", body = ProposeBlockResponse),
        (status = 422, description = "Unprocessable entity", body = String, examples( 
            ("InvalidTxListHash" = (value = json!("invalid txList hash"))), 
            ("OnlyETH" = (value = json!("only receive ETH"))), 
            ("ProofFeeLow" = (value = json!("proof fee too low"))), 
            ("ExpiryTooLong" = (value = json!("expiry too long"))),
        )),
    ),
    request_body = CreateAssignmentRequestBody,
)]
#[debug_handler(state = AppState)]
async fn create_assignment(
    State(state): State<AppState>,
    Json(req): Json<CreateAssignmentRequestBody>,
) -> ApiResult<ProposeBlockResponse> {
    info!(
        target: "create_assignment",
        description = "Proof assignment request body",
        feeToken = req.fee_token.to_string(),
        expiry = req.expiry,
        tierFees = req
            .tier_fees
            .iter()
            .map(|tf| tf.to_string())
            .collect::<Vec<String>>()
            .join(", "),
        txListHash = req.tx_list_hash.to_string(),
    );

    if req.tx_list_hash.is_zero() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid txList hash".to_string(),
        ));
    }

    if !req.fee_token.is_zero() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "only receive ETH".to_string(),
        ));
    }

    if !state.is_guardian {
        // TODO: check prover balance
    }

    for tier in req.tier_fees {
        if tier.tier == Tier::Guardian {
            continue;
        }

        let min_fee = match tier.tier {
            Tier::Optimistic => state.min_optimistic_tier_fee,
            Tier::Sgx => state.min_sgx_tier_fee,
            Tier::PseZkevm => state.min_pse_zkevm_tier_fee,
            Tier::SgxAndPseZkevm => state.min_sgx_and_pse_zkevm_tier_fee,
            _ => 0,
        };

        if tier.fee < min_fee {
            warn!(
                target: "create_assignment",
                description = "Proof fee too low",
                tier = tier.tier.to_string(),
                fee = tier.fee,
                minTierFee = min_fee,
                proposerIP = "TODO",
                // TODO: get proposer IP
            );

            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "proof fee too low".to_string(),
            ));
        }
    }

    let curr_max_expiry = Utc::now().add(chrono::Duration::milliseconds(state.max_expiry as i64));

    if req.expiry > curr_max_expiry.timestamp_millis() as u64 {
        warn!(
            target: "create_assignment",
            description = "Expiry too long",
            expiry= req.expiry,
            srvMaxExpiry= state.max_expiry,
            proposerIP = "TODO",
            // TODO: get proposer IP
        );

        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "expiry too long".to_string(),
        ));
    }

    // TODO: check if prover has any capacity

    // TODO: get L1 block head

    // TODO: encode assignment payload

    // TODO: sign encoded payload

    Ok(Json(ProposeBlockResponse {
        signed_payload: vec![],
        prover: H160::zero(),
        max_block_id: 0,
        max_proposed_in: 0,
    }))
}

#[derive(OpenApi)]
#[openapi(
    paths(create_assignment),
    components(schemas(CreateAssignmentRequestBody, ProposeBlockResponse, TierFee, Tier,),)
)]
pub struct AssignmentDoc;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", post(create_assignment))
}
