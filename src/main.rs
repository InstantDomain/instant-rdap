use async_trait::async_trait;
use futures_util::FutureExt;
use hyper::http::response::Builder;
use hyper::Body;
use mendes::application::{IntoResponse, Server};
use mendes::http::{request::Parts, Response, StatusCode};
use mendes::{handler, Application, Context};
use serde::Serialize;
use std::sync::Arc;

mod autnum;
mod db;
mod exact_match;
mod ip;
mod query;

type RestResponse = Result<Response<Body>>;
type Result<T> = std::result::Result<T, Error>;
const NOT_IMPLEMENTED: Error = Error::Status(StatusCode::NOT_IMPLEMENTED);
const NOT_FOUND: Error = Error::Status(StatusCode::NOT_FOUND);

#[macro_export]
macro_rules! endpoint {
    ($cx:expr, $mod:tt) => {
        async {
            use mendes::http::Method;
            let mut cx = $cx.write().await;

            match cx.method() {
                &Method::HEAD => $mod::head::handler(&mut cx).await,
                &Method::GET => $mod::get::handler(&mut cx).await,
                _ => Err(NOT_FOUND),
            }
            .into_response(cx.app.as_ref(), &cx.req)
        }
        .boxed()
    };
}

#[macro_export]
macro_rules! validate_file {
    ($contents:ident, $ty:ty) => {
        serde_json::to_string_pretty(&serde_json::from_str::<$ty>(&$contents)?)?
    };
}

fn response() -> Builder {
    Response::builder()
        .header("Content-Type", "application/rdap+json; charset=utf-8")
        .status(StatusCode::OK)
}

pub fn ok_body(obj: impl Serialize) -> RestResponse {
    Ok(response().body(Body::from(serde_json::to_string(&obj)?))?)
}

pub fn ok_head() -> RestResponse {
    Ok(response().body(Body::empty())?)
}

pub struct App {
    url_root: String,
    port43: String,
    db: db::Redis,
}

impl App {
    pub fn rdap_conformance(&self) -> Vec<String> {
        vec!["rdap_level_0".into()]
    }

    pub fn content_type(&self) -> String {
        "application/rdap+json".into()
    }

    pub fn port43(&self) -> &str {
        self.port43.as_ref()
    }

    pub fn url_root(&self) -> &str {
        self.url_root.as_ref()
    }

    pub fn db(&self) -> &db::Redis {
        &self.db
    }
}

#[async_trait]
impl Application for App {
    type RequestBody = Body;
    type ResponseBody = Body;
    type Error = Error;

    async fn handle(mut cx: Context<Self>) -> Response<Body> {
        let req_path = cx.req.uri.path().to_string();

        // skip root /rdap
        let _ = cx.path();

        let cx = Arc::new(tokio::sync::RwLock::new(cx));
        let context = cx.clone();

        let paths = vec![
            ("/rdap/autnum", endpoint!(cx, autnum)),
            ("/rdap/ip", endpoint!(cx, ip)),
            ("/rdap/nameservers", endpoint!(cx, query)),
            ("/rdap/domains", endpoint!(cx, query)),
            ("/rdap/entities", endpoint!(cx, query)),
            ("/rdap", endpoint!(cx, exact_match)),
        ];

        for (base, handler) in paths {
            if req_path.starts_with(base) {
                return handler.await;
            }
        }

        let cx = context.read().await;
        NOT_FOUND.into_response(cx.app.as_ref(), &cx.req)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("redis")]
    Redis(#[from] redis::RedisError),
    #[error("Decode")]
    Serde(#[from] serde_json::Error),
    #[error("HTTP")]
    Http(#[from] mendes::http::Error),
    #[error("Io error")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Mendes(#[from] mendes::Error),
    #[error("error")]
    Status(StatusCode),
}

impl From<&Error> for StatusCode {
    fn from(e: &Error) -> StatusCode {
        match e {
            Error::Mendes(e) => StatusCode::from(e),
            Error::Status(code) => *code,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse<App> for Error {
    fn into_response(self, _: &App, _: &Parts) -> Response<Body> {
        Response::builder()
            .status(StatusCode::from(&self))
            .body(self.to_string().into())
            .unwrap()
    }
}

#[tokio::main]
async fn main() {
    App {
        db: db::Redis::new("redis://localhost").await.unwrap(),
        port43: "localhost".to_owned(),
        url_root: "https://localhost".to_owned(),
    }
    .serve(&"0.0.0.0:11000".parse().unwrap())
    .await
    .unwrap();
}
