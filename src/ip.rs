use crate::*;
use ipnet::{IpBitAnd, IpNet, IpSub};
use rdap_types::IpNetwork;
use std::{borrow::Cow, net::IpAddr};

macro_rules! get_net_and_size {
    ($start:ident, $end:ident, $size:expr) => {{
        let size = $end.saturating_sub($start) as u128;

        let net = IpNet::new(
            IpAddr::from($start.bitand($end)),
            $size - (size as f64).log(2.0).ceil() as u8,
        )
        .unwrap();

        (net, size)
    }};
}

#[handler(HEAD)]
pub async fn head(app: &App, _resource: String, #[rest] ip: Cow<'_, str>) -> RestResponse {
    find_network(app, parse_net(ip)?)
        .await
        .and_then(|_| ok_head())
}

#[handler(GET)]
pub async fn get(app: &App, _resource: String, #[rest] ip: Cow<'_, str>) -> RestResponse {
    find_network(app, parse_net(ip)?).await.and_then(ok_body)
}

fn parse_net(path: impl AsRef<str>) -> Result<IpNet> {
    let ip = path.as_ref();
    ip.parse()
        .or_else(|_| Ok(IpNet::from(ip.parse::<IpAddr>()?)))
        .map_err(|_: Box<dyn std::error::Error>| Error::Mendes(mendes::Error::PathDecode))
}

async fn find_network(app: &App, ip: ipnet::IpNet) -> Result<IpNetwork> {
    let mut result = Err(Error::Mendes(mendes::Error::PathNotFound));
    let mut size = u128::MAX;

    for json in app.db.get(&["/ip/*".into()], Default::default()).await? {
        let ipn: IpNetwork = serde_json::from_value(json)?;

        let (net, net_size) = match (ipn.start_address, ipn.end_address) {
            (IpAddr::V4(start), IpAddr::V4(end)) => get_net_and_size!(start, end, 32),
            (IpAddr::V6(start), IpAddr::V6(end)) => get_net_and_size!(start, end, 128),
            _ => continue,
        };

        let query_size = ip.hosts().count() as u128;

        if net.contains(&ip.network()) && net_size >= query_size && net_size < size {
            size = net_size;
            result = Ok(ipn);
        }
    }
    result
}
