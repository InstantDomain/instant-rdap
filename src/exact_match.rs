use crate::*;

macro_rules! get_path {
    ($app:ident, $path:ident, $ty:ty) => {
        ok_body(
            $app.db()
                .get::<$ty>(&[$path], Default::default())
                .await?
                .pop()
                .ok_or_else(|| Error::Mendes(mendes::Error::PathNotFound))?
                .to_rdap($app)
                .await?,
        )
    };
}

#[handler(HEAD)]
pub async fn head(app: &App, resource: String, handle: String) -> RestResponse {
    get::call(app, resource, handle).map(|_| ok_head()).await
}

#[handler(GET)]
pub async fn get(app: &App, resource: String, handle: String) -> RestResponse {
    let path = format!("/{}/{}", resource, handle);

    match resource.as_str() {
        "domain" => get_path!(app, path, db::Whois),
        "nameserver" => get_path!(app, path, db::Nameserver),
        _ => Err(NOT_IMPLEMENTED),
    }
}
