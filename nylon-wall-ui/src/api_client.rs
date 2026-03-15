use gloo_net::http::Request;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

/// Read the API base URL from the JS global `window.__NYLON_API_URL__`.
/// Set this in index.html to match config.toml's `[ui] api_url`.
/// Falls back to relative `/api/v1` if the global is not set (proxy mode).
#[wasm_bindgen(inline_js = "
export function __nylon_get_api_url() { return (typeof window !== 'undefined' && window.__NYLON_API_URL__) || ''; }
export function __nylon_get_ws_url() {
    var base = (typeof window !== 'undefined' && window.__NYLON_API_URL__) || '';
    if (!base) {
        var proto = location.protocol === 'https:' ? 'wss' : 'ws';
        return proto + '://' + location.host + '/api/v1/ws/events';
    }
    var ws = base.replace('https://', 'wss://').replace('http://', 'ws://');
    return ws.replace(/\\/$/, '') + '/api/v1/ws/events';
}
export function __nylon_get_token() {
    try { return localStorage.getItem('nylon_auth_token') || ''; } catch(e) { return ''; }
}
export function __nylon_set_token(t) {
    try { localStorage.setItem('nylon_auth_token', t); } catch(e) {}
}
export function __nylon_clear_token() {
    try { localStorage.removeItem('nylon_auth_token'); } catch(e) {}
}
")]
extern "C" {
    fn __nylon_get_api_url() -> String;
    fn __nylon_get_ws_url() -> String;
    fn __nylon_get_token() -> String;
    fn __nylon_set_token(token: &str);
    fn __nylon_clear_token();
}

fn api_base() -> String {
    let base = __nylon_get_api_url();
    if base.is_empty() {
        "/api/v1".to_string()
    } else {
        format!("{}/api/v1", base.trim_end_matches('/'))
    }
}

/// Build the WebSocket URL for the real-time event stream.
/// Includes the auth token as a query parameter.
pub fn ws_url() -> String {
    let base = __nylon_get_ws_url();
    let token = get_token();
    if token.is_empty() {
        base
    } else {
        format!("{}?token={}", base, token)
    }
}

// === Token management ===

pub fn get_token() -> String {
    __nylon_get_token()
}

pub fn set_token(token: &str) {
    __nylon_set_token(token);
}

pub fn clear_token() {
    __nylon_clear_token();
}

pub fn has_token() -> bool {
    !get_token().is_empty()
}

/// Sentinel error returned on 401 so callers can detect auth failures.
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";

/// Attach auth header to a request builder if a token exists.
fn with_auth(req: gloo_net::http::RequestBuilder) -> gloo_net::http::RequestBuilder {
    let token = get_token();
    if token.is_empty() {
        req
    } else {
        req.header("Authorization", &format!("Bearer {}", token))
    }
}

/// Check response status, clear token on 401.
fn check_status(status: u16, status_text: &str) -> Result<(), String> {
    if status == 401 {
        clear_token();
        return Err(UNAUTHORIZED.to_string());
    }
    if status < 200 || status >= 300 {
        return Err(format!("HTTP {}: {}", status, status_text));
    }
    Ok(())
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = with_auth(Request::get(&url))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    check_status(resp.status(), &resp.status_text())?;
    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn post<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = with_auth(Request::post(&url))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    check_status(resp.status(), &resp.status_text())?;
    resp.json::<R>().await.map_err(|e| e.to_string())
}

pub async fn put<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = with_auth(Request::put(&url))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    check_status(resp.status(), &resp.status_text())?;
    resp.json::<R>().await.map_err(|e| e.to_string())
}

pub async fn delete(path: &str) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let resp = with_auth(Request::delete(&url))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    check_status(resp.status(), &resp.status_text())?;
    Ok(())
}
