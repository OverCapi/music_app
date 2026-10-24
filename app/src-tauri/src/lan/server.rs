//! The HTTP side of the sync, served to the other devices on the network.
//!
//! `POST /request` is open (the user of this device decides); `GET /manifest` and
//! `GET /file/<key>` need the session token given when a request was made or
//! accepted (`Authorization: Bearer`). Only files of library tracks are served.

use std::net::SocketAddr;
use std::path::Path;

use axum::body::Body;
use axum::extract::{ConnectInfo, Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio_util::io::ReaderStream;

use super::merge::Manifest;
use super::{Answer, IncomingError, Lan, RequestBody};
use crate::error::{AppError, AppResult};

impl Lan {
    /// Binds a free port on all interfaces and serves in the background. Returns the port.
    pub(super) async fn serve(&self) -> AppResult<u16> {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
        let port = listener.local_addr()?.port();
        let app = Router::new()
            .route("/request", post(request))
            .route("/manifest", get(manifest))
            .route("/file/{key}", get(file))
            .with_state(self.clone());
        tauri::async_runtime::spawn(async move {
            let service = app.into_make_service_with_connect_info::<SocketAddr>();
            if let Err(e) = axum::serve(listener, service).await {
                tracing::error!("LAN server stopped: {e}");
            }
        });
        Ok(port)
    }

    fn manifest(&self) -> AppResult<Manifest> {
        Manifest::of(&self.inner.library, &self.inner.playlists)
    }
}

async fn request(
    State(lan): State<Lan>,
    ConnectInfo(from): ConnectInfo<SocketAddr>,
    Json(body): Json<RequestBody>,
) -> Result<Json<Answer>, StatusCode> {
    match lan.incoming(body, from.ip()).await {
        Ok(answer) => Ok(Json(answer)),
        Err(IncomingError::Invalid) => Err(StatusCode::BAD_REQUEST),
        Err(IncomingError::Busy) => Err(StatusCode::CONFLICT),
    }
}

async fn manifest(State(lan): State<Lan>, headers: HeaderMap) -> Response {
    if !authorized(&lan, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match lan.manifest() {
        Ok(manifest) => Json(manifest).into_response(),
        Err(e) => internal(e),
    }
}

async fn file(State(lan): State<Lan>, headers: HeaderMap, UrlPath(key): UrlPath<i64>) -> Response {
    if !authorized(&lan, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(path) = lan
        .inner
        .library
        .get(key)
        .ok()
        .and_then(|t| t.path)
        .filter(|p| Path::new(p).is_file())
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(e) => return internal(e.into()),
    };
    let mut response = Body::from_stream(ReaderStream::new(file)).into_response();
    if let Ok(len) = tokio::fs::metadata(&path).await.map(|m| m.len()) {
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, len.into());
    }
    response
}

fn authorized(lan: &Lan, headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| lan.check_token(token))
}

fn internal(e: AppError) -> Response {
    tracing::warn!("LAN request failed: {e}");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
