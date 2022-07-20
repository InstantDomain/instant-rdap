use crate::*;
use rdap_types::AutNum;

#[handler(HEAD)]
pub async fn head(app: &App, resource: String, asn: u32) -> RestResponse {
    find_asn(app, resource, asn).and_then(|_| ok_head())
}

#[handler(GET)]
pub async fn get(app: &App, resource: String, asn: u32) -> RestResponse {
    find_asn(app, resource, asn).and_then(ok_body)
}

fn find_asn(app: &App, resource: String, asn: u32) -> Result<AutNum> {
    todo!()
    // for json in app.search_in_json(resource)? {
    //     let aut: AutNum = serde_json::from_value(json)?;

    //     let start_asn = aut.start_autnum.unwrap_or(0);
    //     let end_asn = aut.end_autnum.unwrap_or(start_asn);

    //     if (start_asn <= asn && asn <= end_asn) || aut.handle == format!("AS{}", asn) {
    //         return Ok(aut);
    //     }
    // }

    // Err(mendes::Error::PathNotFound)?
}
