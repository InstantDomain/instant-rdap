use crate::*;

#[handler(HEAD)]
pub async fn head(app: &App, resource: String, handle: String) -> RestResponse {
    app.file_exists(&resource, &handle)?;

    ok_head()
}

#[handler(GET)]
pub async fn get(app: &App, resource: String, handle: String) -> RestResponse {
    let contents = app.read_file(&resource, &handle).await?;

    let body = match resource.as_str() {
        "domain" => validate_file!(contents, rdap_types::Domain),
        "nameserver" => validate_file!(contents, rdap_types::Nameserver),
        "entity" => validate_file!(contents, rdap_types::Entity),
        _ => return Err(Error::Status(StatusCode::NOT_IMPLEMENTED)),
    };

    Ok(response().body(Body::from(body))?)
}
