use crate::*;

#[handler(HEAD)]
pub async fn head(app: &App, parts: &Parts) -> RestResponse {
    app.file_exists(&parts.uri.path()[1..])?;

    ok_head()
}

#[handler(GET)]
pub async fn get(app: &App, parts: &Parts, resource: String) -> RestResponse {
    if !ALLOWED_RESOURCES.contains(&resource.as_str()) {
        Err(mendes::Error::PathNotFound)?
    }

    let path = &parts.uri.path()[1..];
    let contents = app.read_file(path).await?;

    let body = match resource.as_str() {
        "domain" => validate_file!(contents, rdap_types::Domain),
        "nameserver" => validate_file!(contents, rdap_types::Nameserver),
        "entity" => validate_file!(contents, rdap_types::Entity),
        _ => return Err(Error::Status(StatusCode::NOT_IMPLEMENTED)),
    };

    Ok(response().body(Body::from(body))?)
}
