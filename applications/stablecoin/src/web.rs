//! The local web dashboard: a single page served on the loopback interface, backed by the same
//! [`Issuer`] the command line uses.
//!
//! The server holds the decrypted key for as long as it runs, so every API call must carry the
//! random token printed at start-up.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use rand::RngCore;
use serde_json::json;
use tokio::{net::TcpListener, sync::Mutex};

use crate::issuer::{Action, Deployment, Issuer};

struct App {
    issuer: Mutex<Issuer>,
    deployment: Deployment,
    token: String,
}

pub async fn serve(issuer: Issuer, deployment: Deployment, port: u16) -> anyhow::Result<()> {
    let mut token = [0u8; 16];
    rand::rng().fill_bytes(&mut token);
    let token = hex::encode(token);

    let app = Arc::new(App {
        issuer: Mutex::new(issuer),
        deployment,
        token: token.clone(),
    });
    let router = Router::new()
        .route("/", get(index))
        .route("/api/status", get(status))
        .route("/api/action", post(action))
        .with_state(app);

    // Loopback only: the dashboard signs with the unlocked key.
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    println!("Dashboard: http://127.0.0.1:{port}/?token={token}");
    println!("Press Ctrl-C to stop.");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

async fn status(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return denied();
    }
    let issuer = app.issuer.lock().await;
    match issuer.status(&app.deployment).await {
        Ok(status) => Json(status).into_response(),
        Err(err) => failure(&err),
    }
}

async fn action(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(action): Json<Action>,
) -> Response {
    if !authorized(&app, &headers) {
        return denied();
    }
    let mut issuer = app.issuer.lock().await;
    match issuer.execute(&app.deployment, action).await {
        Ok((label, summary)) => {
            Json(json!({ "label": label, "transaction": summary })).into_response()
        }
        Err(err) => failure(&err),
    }
}

fn authorized(app: &App, headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        == Some(app.token.as_str())
}

fn denied() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Missing or wrong token" })),
    )
        .into_response()
}

fn failure(err: &anyhow::Error) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": format!("{err:#}") })),
    )
        .into_response()
}
