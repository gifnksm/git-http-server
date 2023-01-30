use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use clap::Parser;
use eyre::Result;
use repository::Repository;

mod repository;

#[derive(Parser, Debug)]
struct Args {
    #[clap(
        short,
        long = "bind",
        value_name = "ADDRESS",
        env,
        default_value = "127.0.0.1:8080"
    )]
    bind_addr: SocketAddr,
    #[clap(
        short,
        long = "project-root",
        value_name = "PATH",
        env,
        default_value = "/srv/git"
    )]
    project_root: PathBuf,
}

#[derive(Debug)]
struct State {
    project_root: PathBuf,
}

#[derive(Debug)]
struct AppError(eyre::Report);

impl From<eyre::Report> for AppError {
    fn from(e: eyre::Report) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let AppError(report) = self;
        tracing::warn!("Internal server error: {report:?}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal server error: {report:?}"),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let Args {
        bind_addr,
        project_root,
    } = Args::parse();
    eyre::ensure!(
        project_root.is_dir(),
        "project root must be a directory: {}",
        project_root.display()
    );

    tracing::info!("project_root={}", project_root.display());
    tracing::debug!("listening on {bind_addr}");

    let state = State { project_root };

    let app = Router::new()
        .route("/", get(root))
        .route("/list", get(list_repositories))
        .route("/create", post(create_repository))
        .layer(Extension(Arc::new(state)))
        .layer(middleware::from_fn(print_request_response));

    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn print_request_response(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::trace!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}

#[tracing::instrument]
async fn root() -> &'static str {
    "Hello, world!"
}

#[tracing::instrument]
async fn list_repositories(
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<Vec<Repository>>, AppError> {
    let list = repository::iter(&state.project_root)?
        .filter_map(|repo| match repo {
            Ok(repo) => Some(repo),
            Err(err) => {
                tracing::trace!("failed to list repository: {}", err);
                None
            }
        })
        .collect();
    Ok(Json(list))
}

#[tracing::instrument]
async fn create_repository(
    Extension(state): Extension<Arc<State>>,
    Json(repo): Json<Repository>,
) -> Result<(), AppError> {
    tracing::debug!("repo: {repo:?}");
    repo.create(&state.project_root)?;
    Ok(())
}
