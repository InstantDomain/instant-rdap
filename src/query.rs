use crate::*;
use std::collections::HashMap;

#[handler(HEAD)]
pub async fn head(
    app: &App,
    path: String,
    #[query] query: HashMap<String, String>,
) -> RestResponse {
    if filter_json(app, get_resource(&path)?, query)?.len() > 0 {
        ok_head()
    } else {
        Err(Error::Status(StatusCode::NOT_FOUND))
    }
}

#[handler(GET)]
pub async fn get(app: &App, path: String, #[query] query: HashMap<String, String>) -> RestResponse {
    let resource = get_resource(&path)?;
    let search_results = format!("{}SearchResults", resource);
    let results = filter_json(app, resource, query)?;

    ok_body(serde_json::json!(
    {
    "rdapConformance": ["rdap_level_0"],
    search_results: results
    }
    ))
}

fn get_resource(path: &str) -> Result<&'static str> {
    let res = match path {
        "domains" => "domain",
        "nameservers" => "nameserver",
        "entities" => "entity",
        _ => return Err(Error::Mendes(mendes::Error::PathNotFound)),
    };

    Ok(res)
}

fn filter_json(
    app: &App,
    resource: &str,
    query: HashMap<String, String>,
) -> Result<Vec<serde_json::Value>> {
    let matchers = query
        .into_iter()
        .map(|(k, v)| Ok((k, globset::Glob::new(&v)?.compile_matcher())))
        .collect::<std::result::Result<HashMap<String, _>, globset::Error>>()
        .map_err(|_| Error::Status(StatusCode::UNPROCESSABLE_ENTITY))?;

    use serde_json::Value;
    let results = app
        .search_in_json(resource)?
        .filter_map(|json| {
            if matchers.iter().all(|(k, m)| match json.get(k) {
                Some(Value::String(val)) => m.is_match(val),
                Some(Value::Bool(val)) => m.is_match(&val.to_string()),
                Some(Value::Number(val)) => m.is_match(&val.to_string()),
                _ => false,
            }) {
                Some(json)
            } else {
                None
            }
        })
        .collect();

    Ok(results)
}
