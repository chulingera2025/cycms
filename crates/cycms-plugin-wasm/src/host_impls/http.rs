//! `http` host 组：无白名单的出站 HTTP 客户端，基于 `reqwest`。
//!
//! 协议（JSON 字符串穿透）：
//!
//! - request：`{ method, url, headers: [[k, v], ..], body-base64 }`
//! - response：`{ status, headers: [[k, v], ..], body-base64 }`
//!
//! `body-base64` 为 RFC 4648 标准 base64 编码，空串代表无 body。

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

use crate::bindings::cycms::plugin::http::Host;
use crate::host::HostState;

#[derive(Deserialize)]
struct WasmHttpRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default, rename = "body-base64")]
    body_base64: String,
}

#[derive(Serialize)]
struct WasmHttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    #[serde(rename = "body-base64")]
    body_base64: String,
}

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client init")
    })
}

async fn do_fetch(req: WasmHttpRequest) -> Result<WasmHttpResponse, String> {
    let method = Method::from_bytes(req.method.as_bytes())
        .map_err(|e| format!("http.fetch: invalid method {:?}: {e}", req.method))?;
    let mut builder = client().request(method, &req.url);
    let mut header_map = HashMap::new();
    for (k, v) in req.headers {
        header_map.insert(k, v);
    }
    for (k, v) in header_map {
        builder = builder.header(k, v);
    }
    if !req.body_base64.is_empty() {
        let bytes = BASE64
            .decode(req.body_base64.as_bytes())
            .map_err(|e| format!("http.fetch: invalid body-base64: {e}"))?;
        builder = builder.body(bytes);
    }
    let resp = builder
        .send()
        .await
        .map_err(|e| format!("http.fetch: send: {e}"))?;
    let status = resp.status().as_u16();
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("").to_owned()))
        .collect();
    let body_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("http.fetch: read body: {e}"))?;
    Ok(WasmHttpResponse {
        status,
        headers,
        body_base64: BASE64.encode(&body_bytes),
    })
}

impl Host for HostState {
    async fn fetch(&mut self, request_json: String) -> wasmtime::Result<Result<String, String>> {
        let req: WasmHttpRequest = match serde_json::from_str(&request_json) {
            Ok(r) => r,
            Err(e) => return Ok(Err(format!("http.fetch: invalid request json: {e}"))),
        };
        match do_fetch(req).await {
            Ok(resp) => match serde_json::to_string(&resp) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("http.fetch: serialize response: {e}"))),
            },
            Err(msg) => Ok(Err(msg)),
        }
    }
}
