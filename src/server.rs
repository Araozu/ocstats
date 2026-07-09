use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::{
    AnalyticsStore, Error, ImportSummary, ModelSummary, PeriodUsage, ProjectSummary,
    Reconciliation, SessionUsage, UsageFilter, extract_default,
};

type SharedStore = Arc<Mutex<AnalyticsStore>>;

#[derive(Clone)]
struct AppState {
    store: SharedStore,
}

#[derive(Debug, Deserialize)]
struct PeriodQuery {
    #[serde(flatten)]
    filter: UsageFilter,
    period_ms: i64,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
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
    let extraction = extract_default()?;
    let mut analytics_store = AnalyticsStore::open_default()?;
    analytics_store.import(&extraction)?;
    let store = Arc::new(Mutex::new(analytics_store));
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router(store)).await?;
    Ok(())
}

fn router(store: SharedStore) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/usage/sessions", get(session_usage))
        .route("/api/usage/periods", get(period_usage))
        .route("/api/reconciliation", get(reconciliation))
        .route("/api/projects", get(projects))
        .route("/api/models", get(models))
        .route("/api/import", post(import))
        // The frontend commonly runs from a separate local development origin.
        .layer(CorsLayer::permissive())
        .with_state(AppState { store })
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn session_usage(
    State(state): State<AppState>,
    Query(filter): Query<UsageFilter>,
) -> Result<Json<Vec<SessionUsage>>, ApiError> {
    with_store(&state, |store| store.session_usage_filtered(&filter)).map(Json)
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

async fn import(State(state): State<AppState>) -> Result<Json<ImportSummary>, ApiError> {
    import_data(&state).map(Json)
}

fn import_data(state: &AppState) -> Result<ImportSummary, ApiError> {
    let extraction = extract_default().map_err(api_error)?;
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
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_endpoint_responds() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let response = router(store)
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
    async fn usage_endpoints_accept_filters_and_reject_invalid_periods() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(
            AnalyticsStore::open(directory.path().join("analytics.db")).unwrap(),
        ));
        let app = router(store);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/usage/sessions?project_id=project-1")
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
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
