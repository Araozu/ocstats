use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower_http::cors::CorsLayer;

use crate::pricing::PricingRequests;
use crate::{
    AnalyticsStore, Error, ImportSummary, ModelSummary, ModelUsage, PeriodUsage, PricingCatalog,
    ProjectSummary, Reconciliation, SessionDetail, SessionUsage, TurnText, UsageFilter,
    check_database_path, default_analytics_path, default_database_path, extract_from_path,
};

type SharedStore = Arc<Mutex<AnalyticsStore>>;

#[derive(RustEmbed)]
#[folder = "frontend/build/"]
struct FrontendAssets;

#[derive(Clone)]
struct AppState {
    store: SharedStore,
    pricing: PricingCatalog,
    pricing_requests: Arc<Mutex<PricingRequests>>,
    source_database: Option<PathBuf>,
    auth: AuthState,
}

#[derive(Clone)]
struct AuthState {
    session_token: String,
    secure_cookie: bool,
}

#[derive(Debug, Deserialize)]
struct PeriodQuery {
    #[serde(flatten)]
    filter: UsageFilter,
    period_ms: i64,
}

#[derive(Debug, Deserialize)]
struct SessionDetailQuery {
    source: String,
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct TurnTextQuery {
    source: String,
    session_id: String,
    turn_id: String,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    database: &'static str,
}

#[derive(Serialize)]
struct AuthStatus {
    authenticated: bool,
}

#[derive(Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Serialize)]
struct ApiError {
    #[serde(skip)]
    status: StatusCode,
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}

pub async fn serve_default(port: u16) -> Result<(), Error> {
    let auth = AuthState::from_environment()?;
    if let Some(path) = std::env::var_os("OCSTATS_PRICING_FILE") {
        eprintln!(
            "ocstats: loading pricing catalog from {}",
            PathBuf::from(path).display()
        );
    } else {
        eprintln!("ocstats: loading embedded pricing catalog");
    }
    let pricing = PricingCatalog::load_default()?;
    let source_database = default_database_path()?;
    eprintln!(
        "ocstats: opening OpenCode database at {}",
        source_database.display()
    );
    let analytics_database = default_analytics_path()?;
    eprintln!(
        "ocstats: opening analytics database at {}",
        analytics_database.display()
    );
    let store = Arc::new(Mutex::new(AnalyticsStore::open(analytics_database)?));
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("ocstats: listening on http://{address}");
    axum::serve(
        listener,
        router_with_source(store, pricing, source_database, auth),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
fn router(store: SharedStore, catalog: PricingCatalog) -> Router {
    router_with_requests(store, catalog, PricingRequests::new("pricing-requests.txt"))
}

fn router_with_source(
    store: SharedStore,
    catalog: PricingCatalog,
    source_database: PathBuf,
    auth: AuthState,
) -> Router {
    router_with_requests_and_source(
        store,
        catalog,
        PricingRequests::new("pricing-requests.txt"),
        Some(source_database),
        auth,
    )
}

#[cfg(test)]
fn router_with_requests(
    store: SharedStore,
    catalog: PricingCatalog,
    pricing_requests: PricingRequests,
) -> Router {
    router_with_requests_and_source(
        store,
        catalog,
        pricing_requests,
        None,
        AuthState::for_test(),
    )
}

fn router_with_requests_and_source(
    store: SharedStore,
    catalog: PricingCatalog,
    pricing_requests: PricingRequests,
    source_database: Option<PathBuf>,
    auth: AuthState,
) -> Router {
    let state = AppState {
        store,
        pricing: catalog,
        pricing_requests: Arc::new(Mutex::new(pricing_requests)),
        source_database,
        auth,
    };
    let protected_api = Router::new()
        .route("/api/usage/sessions", get(session_usage))
        .route("/api/usage/models", get(model_usage))
        .route("/api/usage/session", get(session_detail))
        .route("/api/usage/turn-text", get(turn_text))
        .route("/api/usage/periods", get(period_usage))
        .route("/api/reconciliation", get(reconciliation))
        .route("/api/projects", get(projects))
        .route("/api/models", get(models))
        .route("/api/pricing", get(pricing_endpoint))
        .route("/api/pricing/request", post(request_pricing))
        .route("/api/import", post(import))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .merge(protected_api)
        .fallback(frontend_asset)
        // The frontend commonly runs from a separate local development origin.
        .layer(CorsLayer::permissive())
        .with_state(state)
}

impl AuthState {
    fn from_environment() -> Result<Self, Error> {
        let password = std::env::var("OCSTATS_PASSWORD")
            .map_err(|_| Error::Configuration("set OCSTATS_PASSWORD".into()))?;
        if password.is_empty() {
            return Err(Error::Configuration(
                "OCSTATS_PASSWORD must not be empty".into(),
            ));
        }
        let secure_cookie = std::env::var("OCSTATS_COOKIE_SECURE")
            .map(|value| value != "false")
            .unwrap_or(true);
        Ok(Self {
            session_token: format!("{:x}", Sha256::digest(password.as_bytes())),
            secure_cookie,
        })
    }

    #[cfg(test)]
    fn for_test() -> Self {
        Self {
            session_token: "test-session".into(),
            secure_cookie: false,
        }
    }
}

async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if is_authenticated(request.headers(), &state.auth) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn auth_status(State(state): State<AppState>, headers: HeaderMap) -> Json<AuthStatus> {
    Json(AuthStatus {
        authenticated: is_authenticated(&headers, &state.auth),
    })
}

async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<(HeaderMap, StatusCode), StatusCode> {
    if format!("{:x}", Sha256::digest(request.password.as_bytes())) != state.auth.session_token {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok((session_cookie(&state.auth, false), StatusCode::NO_CONTENT))
}

async fn logout(State(state): State<AppState>) -> (HeaderMap, StatusCode) {
    (session_cookie(&state.auth, true), StatusCode::NO_CONTENT)
}

fn is_authenticated(headers: &HeaderMap, auth: &AuthState) -> bool {
    headers
        .get("cookie")
        .and_then(|header| header.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("ocstats_session="))
        })
        .is_some_and(|token| token == auth.session_token)
}

fn session_cookie(auth: &AuthState, clear: bool) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let max_age = if clear {
        "Max-Age=0"
    } else {
        "Max-Age=2592000"
    };
    let secure = if auth.secure_cookie { "; Secure" } else { "" };
    let value = format!(
        "ocstats_session={}; Path=/; HttpOnly; SameSite=Strict; {max_age}{secure}",
        auth.session_token
    );
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&value).expect("session cookie is valid"),
    );
    headers
}

async fn frontend_asset(uri: Uri) -> Response {
    if uri.path() == "/api" || uri.path().starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let requested_path = uri.path().trim_start_matches('/');
    let requested_path = if requested_path.is_empty() {
        "index.html"
    } else {
        requested_path
    };
    let (asset_path, asset) = match FrontendAssets::get(requested_path) {
        Some(asset) => (requested_path, asset),
        None => match FrontendAssets::get("index.html") {
            Some(asset) => ("index.html", asset),
            None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    };

    (
        [(
            "content-type",
            mime_guess::from_path(asset_path)
                .first_or_octet_stream()
                .as_ref(),
        )],
        asset.data.into_owned(),
    )
        .into_response()
}

async fn health(State(state): State<AppState>) -> Result<Json<Health>, ApiError> {
    let database_path = state
        .source_database
        .clone()
        .map(Ok)
        .unwrap_or_else(default_database_path)
        .map_err(api_error)?;
    check_database_path(database_path).map_err(|error| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        error: error.to_string(),
    })?;
    Ok(Json(Health {
        status: "ok",
        database: "ok",
    }))
}

async fn session_usage(
    State(state): State<AppState>,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<SessionUsage>>, ApiError> {
    with_store(&state, |store| store.session_usage_filtered(&filter)).map(Json)
}

async fn model_usage(
    State(state): State<AppState>,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<ModelUsage>>, ApiError> {
    with_store(&state, |store| store.model_usage_filtered(&filter)).map(Json)
}

async fn session_detail(
    State(state): State<AppState>,
    Query(query): Query<SessionDetailQuery>,
) -> Result<Json<SessionDetail>, ApiError> {
    let detail = with_store(&state, |store| {
        store.session_detail(&query.source, &query.session_id)
    })?;
    detail.map(Json).ok_or(ApiError {
        status: StatusCode::NOT_FOUND,
        error: "session not found".into(),
    })
}

async fn turn_text(
    State(state): State<AppState>,
    Query(query): Query<TurnTextQuery>,
) -> Result<Json<TurnText>, ApiError> {
    let text = with_store(&state, |store| {
        store.turn_text(&query.source, &query.session_id, &query.turn_id)
    })?;
    text.map(Json).ok_or(ApiError {
        status: StatusCode::NOT_FOUND,
        error: "turn not found".into(),
    })
}

async fn period_usage(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Result<Json<Vec<PeriodUsage>>, ApiError> {
    with_store(&state, |store| {
        store.period_usage(&query.filter, query.period_ms)
    })
    .map(Json)
}

async fn reconciliation(
    State(state): State<AppState>,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<Reconciliation>>, ApiError> {
    with_store(&state, |store| store.reconcile(&filter)).map(Json)
}

async fn projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectSummary>>, ApiError> {
    with_store(&state, AnalyticsStore::projects).map(Json)
}

async fn models(State(state): State<AppState>) -> Result<Json<Vec<ModelSummary>>, ApiError> {
    with_store(&state, AnalyticsStore::models).map(Json)
}

async fn pricing_endpoint(State(state): State<AppState>) -> Json<PricingCatalog> {
    Json(state.pricing)
}

#[derive(Debug, Deserialize)]
struct PricingRequest {
    slug: String,
}

async fn request_pricing(
    State(state): State<AppState>,
    Json(request): Json<PricingRequest>,
) -> Result<StatusCode, ApiError> {
    if request.slug.trim().is_empty() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            error: "pricing request slug must not be empty".into(),
        });
    }

    let requests = state.pricing_requests.lock().map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        error: "pricing request lock was poisoned".into(),
    })?;
    requests.record(request.slug.trim()).map_err(api_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn import(State(state): State<AppState>) -> Result<Json<ImportSummary>, ApiError> {
    import_data(&state).map(Json)
}

fn import_data(state: &AppState) -> Result<ImportSummary, ApiError> {
    let source_database = state
        .source_database
        .clone()
        .map(Ok)
        .unwrap_or_else(default_database_path)
        .map_err(api_error)?;
    let extraction = extract_from_path(source_database).map_err(api_error)?;
    with_store_mut(state, |store| store.import(&extraction))
}

fn with_store<T>(
    state: &AppState,
    operation: impl FnOnce(&AnalyticsStore) -> Result<T, Error>,
) -> Result<T, ApiError> {
    let store = state.store.lock().map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        error: "analytics store lock was poisoned".into(),
    })?;
    operation(&store).map_err(api_error)
}

fn with_store_mut<T>(
    state: &AppState,
    operation: impl FnOnce(&mut AnalyticsStore) -> Result<T, Error>,
) -> Result<T, ApiError> {
    let mut store = state.store.lock().map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        error: "analytics store lock was poisoned".into(),
    })?;
    operation(&mut store).map_err(api_error)
}

fn api_error(error: Error) -> ApiError {
    ApiError {
        status: match &error {
            Error::InvalidPeriod | Error::DataPath(_) | Error::MissingDatabase(_) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_endpoint_responds() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("opencode.db");
        rusqlite::Connection::open(&source_path).unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let response = router_with_source(
            store,
            PricingCatalog { models: vec![] },
            source_path,
            AuthState::for_test(),
        )
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn import_endpoint_imports_the_configured_source_database() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("opencode.db");
        rusqlite::Connection::open(&source_path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE project (id TEXT, worktree TEXT, name TEXT);
                 CREATE TABLE session (
                    id TEXT, project_id TEXT, title TEXT, model TEXT, cost REAL,
                    tokens_input INTEGER, tokens_output INTEGER, tokens_reasoning INTEGER,
                    tokens_cache_read INTEGER, tokens_cache_write INTEGER,
                    time_created INTEGER, time_updated INTEGER, parent_id TEXT
                 );
                 CREATE TABLE message (
                    id TEXT, session_id TEXT, data TEXT, time_created INTEGER, time_updated INTEGER
                 );
                 CREATE TABLE part (
                    id TEXT, message_id TEXT, session_id TEXT, data TEXT,
                    time_created INTEGER, time_updated INTEGER
                 );",
            )
            .unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let response = router_with_source(
            store,
            PricingCatalog { models: vec![] },
            source_path,
            AuthState::for_test(),
        )
        .oneshot(
            Request::post("/api/import")
                .header("cookie", "ocstats_session=test-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn frontend_fallback_serves_the_spa_without_catching_api_routes() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let app = router(store, PricingCatalog { models: vec![] });

        let frontend_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(frontend_response.status(), StatusCode::OK);
        assert_eq!(frontend_response.headers()["content-type"], "text/html");

        let api_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(api_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn pricing_endpoint_responds_with_catalog() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let app = router_with_requests(
            store,
            PricingCatalog {
                models: vec![crate::ModelPricing {
                    provider: "test".into(),
                    slug: "test-model".into(),
                    prices: vec![crate::PricePeriod {
                        effective_from: "2026-01-01T00:00:00Z".into(),
                        input: 1.0,
                        cached_write: None,
                        cached_read: Some(0.1),
                        output: 2.0,
                    }],
                }],
            },
            crate::pricing::PricingRequests::new(directory.path().join("pricing-requests.txt")),
        );
        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/pricing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/pricing")
                    .header("cookie", "ocstats_session=test-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["models"][0]["input"], 1.0);
        assert_eq!(json["models"][0]["prices"][0]["input"], 1.0);
    }

    #[tokio::test]
    async fn pricing_request_endpoint_records_unique_slugs() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let requests_path = directory.path().join("pricing-requests.txt");
        let app = router_with_requests(
            store,
            PricingCatalog { models: vec![] },
            crate::pricing::PricingRequests::new(&requests_path),
        );

        for slug in ["gpt-unknown", "gpt-unknown", "claude-unknown"] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/pricing/request")
                        .header("content-type", "application/json")
                        .header("cookie", "ocstats_session=test-session")
                        .body(Body::from(format!(r#"{{"slug":"{slug}"}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        assert_eq!(
            std::fs::read_to_string(requests_path).unwrap(),
            "claude-unknown\ngpt-unknown\n"
        );
    }

    #[tokio::test]
    async fn usage_endpoints_accept_filters_and_reject_invalid_periods() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let app = router(store, PricingCatalog { models: vec![] });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/usage/sessions?project_id=project-1")
                    .header("cookie", "ocstats_session=test-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/usage/periods?period_ms=0")
                    .header("cookie", "ocstats_session=test-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
