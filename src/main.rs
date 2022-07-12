use async_trait::async_trait;
use hyper::http::response::Builder;
use hyper::Body;
use mendes::application::{IntoResponse, Server};
use mendes::http::{request::Parts, Response, StatusCode};
use mendes::{handler, route, Application, Context};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Error>;
const ALLOWED_RESOURCES: [&str; 5] = ["ip", "domain", "autnum", "nameserver", "entity"];

// #[query] opts: std::collections::HashMap<String, String>,

macro_rules! validate_file {
    ($contents:ident, $ty:ty) => {
        serde_json::to_string_pretty(&serde_json::from_str::<$ty>(&$contents)?)?
    };
}

fn invalid_path() -> Result<Response<Body>> {
    Err(Error::Mendes(mendes::Error::MethodNotAllowed))
}

fn response() -> Builder {
    Response::builder()
        .header("Content-Type", "application/rdap+json; charset=utf-8")
        .status(StatusCode::OK)
}

#[handler(HEAD)]
async fn head(app: &App, parts: &Parts) -> Result<Response<Body>> {
    app.file_exists(&parts.uri.path()[1..])?;

    Ok(response().body(Body::empty())?)
}

#[handler(GET)]
async fn get(app: &App, parts: &Parts, resource: String) -> Result<Response<Body>> {
    if !ALLOWED_RESOURCES.contains(&resource.as_str()) {
        Err(mendes::Error::PathNotFound)?
    }

    let path = &parts.uri.path()[1..];
    let contents = app.read_file(path).await?;

    let body = match resource.as_str() {
        "ip" => validate_file!(contents, rdap_types::IpNetwork),
        "domain" => validate_file!(contents, rdap_types::Domain),
        "autnum" => validate_file!(contents, rdap_types::AutNum),
        "nameserver" => validate_file!(contents, rdap_types::Nameserver),
        "entity" => validate_file!(contents, rdap_types::Entity),
        _ => return invalid_path(),
    };

    Ok(response().body(Body::from(body))?)
}

struct App {
    dir: PathBuf,
}

impl App {
    fn with_path(dir: impl AsRef<Path>) -> std::result::Result<App, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&dir)?;

        Ok(App {
            dir: dir.as_ref().to_owned(),
        })
    }

    fn file_exists(&self, path_segment: impl AsRef<Path>) -> Result<()> {
        let path = self.dir.join(&path_segment);

        if path.exists() && path.is_file() {
            Ok(())
        } else {
            Err(mendes::Error::PathNotFound)?
        }
    }

    async fn read_file(&self, path_segment: impl AsRef<Path>) -> Result<String> {
        println!("{:?}", path_segment.as_ref());
        Ok(tokio::fs::read_to_string(self.dir.join(path_segment))
            .await
            .map_err(|_| mendes::Error::PathNotFound)?)
    }

    async fn search_in_json(&self, path: PathBuf) -> Result<String> {
        todo!()
    }
}

#[async_trait]
impl Application for App {
    type RequestBody = Body;
    type ResponseBody = Body;
    type Error = Error;

    async fn handle(mut cx: Context<Self>) -> Response<Body> {
        route!(match cx.path() {
            Some("rdap") => match cx.method() {
                HEAD => head,
                GET => get,
            },
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Decode")]
    Serde(#[from] serde_json::Error),
    #[error("HTTP")]
    Http(#[from] mendes::http::Error),
    #[error("Io error")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Mendes(#[from] mendes::Error),
}

impl From<&Error> for StatusCode {
    fn from(e: &Error) -> StatusCode {
        match e {
            Error::Mendes(e) => StatusCode::from(e),
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
