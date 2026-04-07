use std::sync::Arc;

use apalis::prelude::BoxDynError;
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use clap::Args;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{job::Job, server::Server};

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Http {
    host: String,
    port: u16,
}

pub type HttpServer = Arc<RwLock<Server<Http>>>;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FetchRequest {
    #[serde(default)]
    queues: Vec<String>,
    #[serde(default)]
    worker_id: String,
}

// GET /health - Health check
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

// GET /stats - Server statistics
async fn get_stats(State(server): State<HttpServer>) -> impl IntoResponse {
    let _server = server.read().await;
    // let stats = server.get_stats();
    Json(ApiResponse::success(420))
}

// POST /jobs - Push a new job
async fn push_job(State(server): State<HttpServer>, Json(job): Json<Job>) -> impl IntoResponse {
    let mut server = server.write().await;
    match server.push_job(job).await {
        Ok(jid) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(serde_json::json!({"jid": jid}))),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(e)),
        ),
    }
}

// POST /jobs/fetch - Fetch a job
async fn fetch_job(
    State(server): State<HttpServer>,
    Json(req): Json<FetchRequest>,
) -> impl IntoResponse {
    let mut server = server.write().await;
    // server.process_scheduled_jobs();
    // server.process_retries();
    // server.check_expired_jobs();

    match server.fetch_job(&req.queues, &req.worker_id).await {
        Some(job) => {
            server
                .working_jobs
                .insert(job.jid.clone().unwrap(), req.worker_id);
            (StatusCode::OK, Json(ApiResponse::success(job)))
        }
        None => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<Job>::error("No jobs available".to_string())),
        ),
    }
}

// POST /jobs/{jid}/ack - Acknowledge job completion
async fn ack_job(
    State(server): State<HttpServer>,
    axum::extract::Path(jid): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut server = server.write().await;
    match server.ack_job(&jid).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(
                serde_json::json!({"message": "Job acknowledged"}),
            )),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(e)),
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FailCommand {
    jid: String,
    errtype: String,
    message: String,
    backtrace: Vec<String>,
}

// POST /jobs/{jid}/fail - Report job failure
async fn fail_job(
    State(server): State<HttpServer>,
    axum::extract::Path(jid): axum::extract::Path<String>,
    Json(fail_cmd): Json<FailCommand>,
) -> impl IntoResponse {
    let mut server = server.write().await;
    match server
        .fail_job(
            &jid,
            &fail_cmd.errtype,
            &fail_cmd.message,
            fail_cmd.backtrace,
        )
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(
                serde_json::json!({"message": "Job marked as failed"}),
            )),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(e)),
        ),
    }
}

// DELETE /jobs - Flush all jobs
async fn flush_jobs(State(server): State<HttpServer>) -> impl IntoResponse {
    let mut server = server.write().await;
    server.flush();
    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::json!({"message": "All jobs flushed"}),
        )),
    )
}

// GET /queues - List all queues
async fn list_queues(State(server): State<HttpServer>) -> impl IntoResponse {
    let server = server.read().await;
    let queues: Vec<String> = server.queues.keys().cloned().collect();
    Json(ApiResponse::success(queues))
}

// GET /workers - List all workers
async fn list_workers(State(server): State<HttpServer>) -> impl IntoResponse {
    let server = server.read().await;
    let workers: Vec<_> = server
        .workers
        .values()
        .map(|w| {
            serde_json::json!({
                "wid": w.wid,
                "hostname": w.hostname,
                "pid": w.pid,
                "labels": w.labels,
            })
        })
        .collect();
    Json(ApiResponse::success(workers))
}

pub async fn run_http_server(server: HttpServer) -> Result<(), BoxDynError> {
    let addr = {
        let server = server.read().await;
        format!("{}:{}", server.mode.host, server.mode.port)
    };
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/jobs", post(push_job).delete(flush_jobs))
        .route("/jobs/fetch", post(fetch_job))
        .route("/jobs/{jid}/ack", post(ack_job))
        .route("/jobs/{jid}/fail", post(fail_job))
        .route("/queues", get(list_queues))
        .route("/workers", get(list_workers))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}
