use crate::*;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[async_trait]
pub trait ToRdap {
    type Rdap: Serialize;
    async fn to_rdap(self, app: &App) -> Result<Self::Rdap>;
}

#[async_trait]
impl ToRdap for serde_json::Value {
    type Rdap = Self;

    async fn to_rdap(self, _app: &App) -> Result<Self::Rdap> {
        Ok(self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Whois {
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub name_servers: Vec<String>,
    pub status: Option<Vec<rdap_types::Status>>,
    pub dnssec: Option<rdap_types::SecureDns>,
    pub unicode_name: String,
    pub ldh_name: String,
}

#[async_trait]
impl ToRdap for Whois {
    type Rdap = rdap_types::Domain;

    async fn to_rdap(self, app: &App) -> Result<rdap_types::Domain> {
        let mut events = rdap_types::Events(vec![rdap_types::Event {
            event_action: rdap_types::EventAction::Registration,
            event_date: self.created_at.into(),
            event_actor: None,
            links: None,
        }]);

        if let Some(expiry) = self.expires_at {
            events.0.push(rdap_types::Event {
                event_action: rdap_types::EventAction::Expiration,
                event_date: expiry.into(),
                event_actor: None,
                links: None,
            });
        }

        let nameservers = futures_util::future::join_all(
            app.db()
                .get::<Nameserver>(
                    &self
                        .name_servers
                        .iter()
                        .map(|ns| format!("/nameserver/{}", ns))
                        .collect::<Vec<_>>(),
                    Default::default(),
                )
                .await?
                .into_iter()
                .map(|ns| ns.to_rdap(app)),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

        let link = format!("{}/domain/{}", app.url_root(), self.ldh_name);
        let links = vec![rdap_types::Link {
            value: Some(link.clone()),
            rel: Some("self".into()),
            href: link.clone(),
            hreflang: Some(vec!["en".into()]),
            title: None,
            media: None,
            typ: Some(app.content_type()),
        }];

        Ok(rdap_types::Domain {
            events,
            object_class_name: "domain".into(),
            status: self.status,
            entities: vec![],
            secure_dns: self.dnssec,
            handle: Some(self.ldh_name),
            nameservers: Some(nameservers),
            port43: Some(app.port43().into()),
            rdap_conformance: Some(app.rdap_conformance()),
            links: Some(links),
            variants: None,
            remarks: None,
            network: None,
            notices: None,
            lang: None,
            fred_keyset: None,
            fred_nsset: None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Nameserver {
    ldh_name: String,
    unicode_name: String,
    ip_addresses: rdap_types::IpAddresses,
    status: Option<Vec<rdap_types::Status>>,
}

#[async_trait]
impl ToRdap for Nameserver {
    type Rdap = rdap_types::Nameserver;

    async fn to_rdap(self, app: &App) -> Result<rdap_types::Nameserver> {
        let link = format!("{}/nameserver/{}", app.url_root(), self.ldh_name);
        let links = vec![rdap_types::Link {
            value: Some(link.clone()),
            rel: Some("self".into()),
            href: link.clone(),
            hreflang: Some(vec!["en".into()]),
            title: None,
            media: None,
            typ: Some(app.content_type()),
        }];

        Ok(rdap_types::Nameserver {
            object_class_name: "nameserver".into(),
            handle: Some(self.ldh_name.clone()),
            ldh_name: self.ldh_name,
            unicode_name: Some(self.unicode_name),
            ip_addresses: Some(self.ip_addresses),
            status: Some(vec![rdap_types::Status::Active]),
            entities: None,
            remarks: None,
            notices: None,
            links: Some(links),
        })
    }
}

pub struct Redis {
    conn: Arc<redis::Client>,
}

impl Redis {
    pub async fn new(url: &str) -> Result<Self> {
        Ok(Self {
            conn: redis::Client::open(url)?.into(),
        })
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        key: &[String],
        query: HashMap<String, String>,
    ) -> Result<Vec<T>> {
        let mut redis = self.conn.get_async_connection().await?;
        let results: Vec<String> = redis::cmd("MGET").arg(key).query_async(&mut redis).await?;

        let matchers = query
            .into_iter()
            .map(|(k, v)| Ok((k, globset::Glob::new(&v)?.compile_matcher())))
            .collect::<std::result::Result<HashMap<String, _>, globset::Error>>()
            .map_err(|_| Error::Status(StatusCode::UNPROCESSABLE_ENTITY))?;

        use serde_json::Value;
        let results = results
            .iter()
            .filter_map(|value| {
                let json: serde_json::Value = serde_json::from_str(value).ok()?;

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
            .filter_map(|val| serde_json::from_value(val).ok())
            .collect();

        Ok(results)
    }
}
