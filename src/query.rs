use crate::*;
use std::collections::HashMap;

#[handler(HEAD)]
pub async fn head(
    app: &App,
    path: String,
    #[query] query: HashMap<String, String>,
) -> RestResponse {
    get::call(app, path, query).map(|_| ok_head()).await
}

#[handler(GET)]
pub async fn get(app: &App, path: String, #[query] query: HashMap<String, String>) -> RestResponse {
    let resource = get_resource(&path)?;
    let search_results = format!("{}SearchResults", resource);

    ok_body(serde_json::json!(
    {
    "rdapConformance": ["rdap_level_0"],
    search_results: match path.as_str() {
        "domains" => app.db.get::<db::Whois>(&["/domain/*".into()], query).await?,
        "nameservers" => app.db.get::<db::Whois>(&["/nameserver/*".into()], query).await?,
        _ => return Err(NOT_IMPLEMENTED),

    }
    }
    ))
}

fn get_resource(path: &str) -> Result<&'static str> {
    let res = match path {
        "domains" => "domain",
        "nameservers" => "nameserver",
        "entities" => "entity",
        _ => return Err(NOT_IMPLEMENTED),
    };

    Ok(res)
}
