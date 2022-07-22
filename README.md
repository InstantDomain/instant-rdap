# Instant RDAP server

This is an RDAP server written in Rust, using [Mendes](https://crates.io/crates/mendes) and Redis as a backend.

To get going, use the usual Rust incantation:

    cargo run --release
	
## Status

Currently the featureset is pretty minimal, and experimental.
The RDAP resources are simply picked from Redis, and deserialized as JSON.

What is working:
 
 * queries on `/rdap/domain/<domain>`, `/rdap/nameserver/<nameserver>` and `/rdap/entity/<entity>`
 * filtering based on JSON fields and wildcard queries, based on glob syntax eg.: `/rdap/domains?ldhName=foo*.com`
 * queries on `/ip/<address>/<subnet>` and `/autnum/<asn>` endpoints
 
## Data expectations

Data is stored in Redis as JSON under the following paths:

 * `/domain/<name>`: Domain records
 * `/nameserver/<name>`: Nameserver records
 * `/entity/<id>`: Entities (vcard)
 * `/ip/<any identifier>`: IP network information
 * `/autnum/<asn>`: Autonomous system information
 
Except for types defined in [`src/db.rs`], JSON objects are
deserialized as types from the
[`rdap_types`](https://github.com/rsdy/rdap_client/tree/master/rdap_types)
crate.

## License

Licensed under the Apache2 license.
