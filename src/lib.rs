use assignment::AssignmentDoc;
use axum::{debug_handler, http::StatusCode, routing::get, Json, Router};
use p256::ecdsa::SigningKey;
use rand::rngs::ThreadRng;
use serde_repr::Deserialize_repr;
use status::StatusDoc;
use tower_http::trace::{self, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

mod assignment;
mod status;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppState {
    prover_private_key: SigningKey,
    prover_address: ethereum_types::H160,
    min_optimistic_tier_fee: u64,
    min_sgx_tier_fee: u64,
    min_pse_zkevm_tier_fee: u64,
    min_sgx_and_pse_zkevm_tier_fee: u64,
    max_expiry: u64,
    max_slippage: u64,
    max_proposed_in: u64,
    propose_concurrency_guard: (),
    taiko_l1_address: ethereum_types::H160,
    assignment_hook_address: ethereum_types::H160,
    // rpc:                      *rpc.Client,
    // protocol_configs:          *bindings.TaikoDataConfig,
    liveness_bond: u64,
    is_guardian: bool,
    // db
}

impl Default for AppState {
    fn default() -> Self {
        let mut rng = ThreadRng::default();
        let prover_private_key = SigningKey::random(&mut rng);

        Self {
            prover_private_key,
            prover_address: ethereum_types::H160::zero(),
            min_optimistic_tier_fee: 0,
            min_sgx_tier_fee: 0,
            min_pse_zkevm_tier_fee: 0,
            min_sgx_and_pse_zkevm_tier_fee: 0,
            max_expiry: 0,
            max_slippage: 0,
            max_proposed_in: 0,
            propose_concurrency_guard: (),
            taiko_l1_address: ethereum_types::H160::zero(),
            assignment_hook_address: ethereum_types::H160::zero(),
            liveness_bond: 0,
            is_guardian: false,
        }
    }
}

#[derive(Debug, Deserialize_repr, PartialEq, Eq, ToSchema)]
#[repr(u8)]
enum Tier {
    Optimistic,
    Sgx,
    PseZkevm,
    SgxAndPseZkevm,
    Guardian,
}

impl ToString for Tier {
    fn to_string(&self) -> String {
        match self {
            Tier::Optimistic => "Optimistic",
            Tier::Sgx => "Sgx",
            Tier::PseZkevm => "PseZkevm",
            Tier::SgxAndPseZkevm => "SgxAndPseZkevm",
            Tier::Guardian => "Guardian",
        }
        .to_string()
    }
}

pub type ApiError = (StatusCode, String);
pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[utoipa::path(
    get, 
    path = "/healthz", 
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = ()),
    )
)]
#[debug_handler(state = AppState)]
async fn health() -> ApiResult<()> {
    Ok(Json(()))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health
    ),
    info(
        title = "Taiko Prover Server API",
        version = "1.0",
        description = "Taiko Prover Server API",
        contact (
            name = "API Support",
            url = "https://community.taiko.xyz",
            email = "info@taiko.xyz",
        ),
        license (
            name = "MIT",
            url = "https://github.com/taikoxyz/taiko-client/blob/main/LICENSE.md"
        ),
    )
)]
struct ApiDoc;

pub fn create_router() -> Router<AppState> {
    let mut doc = ApiDoc::openapi();
    let docs = [
        AssignmentDoc::openapi(),
        StatusDoc::openapi(),
    ];
    
    for sub_doc in docs {
        doc.merge(sub_doc);
    }

    let swagger = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", doc);

    Router::new()
        .merge(swagger)
        .route("/", get(health))
        .route("/healthz", get(health))
        .nest("/status", status::create_router())
        .nest("/assignment", assignment::create_router())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
}

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn ensure_env(name: &str) -> String {
    std::env::var(name).expect(&format!("{name} is not set"))
}
