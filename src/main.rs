use async_trait::async_trait;
use futures_util::FutureExt;
use hyper::http::response::Builder;
use hyper::Body;
use mendes::application::{IntoResponse, Server};
use mendes::http::{request::Parts, Response, StatusCode};
use mendes::{handler, Application, Context};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod autnum;
mod exact_match;
mod ip;
mod query;

type RestResponse = Result<Response<Body>>;
type Result<T> = std::result::Result<T, Error>;
const ALLOWED_RESOURCES: [&str; 5] = ["ip", "domain", "autnum", "nameserver", "entity"];

#[macro_export]
macro_rules! endpoint {
    ($cx:expr, $mod:tt) => {
        async {
            use mendes::http::Method;
            let mut cx = $cx.write().await;

            match cx.method() {
                &Method::HEAD => $mod::head::handler(&mut cx).await,
                &Method::GET => $mod::get::handler(&mut cx).await,
                _ => invalid_path(),
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

fn invalid_path() -> RestResponse {
    Err(Error::Mendes(mendes::Error::MethodNotAllowed))
}

pub fn ok_body(obj: impl Serialize) -> RestResponse {
    Ok(response().body(Body::from(serde_json::to_string(&obj)?))?)
}

pub fn ok_head() -> RestResponse {
    Ok(response().body(Body::empty())?)
}

pub struct App {
    dir: PathBuf,
}

impl App {
    fn with_path(dir: impl AsRef<Path>) -> std::result::Result<App, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&dir)?;

        Ok(App {
            dir: dir.as_ref().to_owned(),
        })
    }

    fn file_exists(&self, resource: &str, handle: &str) -> Result<()> {
        if !ALLOWED_RESOURCES.contains(&resource) {
            Err(mendes::Error::PathNotFound)?
        }

        let path = self.dir.join("rdap").join(resource).join(handle);
        if path.exists() && path.is_file() {
            Ok(())
        } else {
            Err(mendes::Error::PathNotFound)?
        }
    }

    async fn read_file(&self, resource: &str, handle: &str) -> Result<String> {
        if !ALLOWED_RESOURCES.contains(&resource) {
            Err(mendes::Error::PathNotFound)?
        }

        let path = self.dir.join("rdap").join(resource).join(handle);
        Ok(tokio::fs::read_to_string(path)
            .await
            .map_err(|_| mendes::Error::PathNotFound)?)
    }

    // this really should be async, but typing through Stream<Item =
    // Value> transformations is a lot of work
    fn search_in_json(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = serde_json::Value>> {
        Ok(
            std::fs::read_dir(self.dir.join("rdap").join(path))?.filter_map(|f| {
                let f = std::fs::File::open(f.ok()?.path()).ok()?;
                serde_json::from_reader(f).ok()
            }),
        )
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
        invalid_path().into_response(cx.app.as_ref(), &cx.req)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    App::with_path("./")
        .unwrap()
        .serve(&"0.0.0.0:11000".parse().unwrap())
        .await
        .unwrap();
}
