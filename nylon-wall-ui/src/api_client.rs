use gloo_net::http::Request;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

/// Read the API base URL from the JS global `window.__NYLON_API_URL__`.
/// Set this in index.html to match config.toml's `[ui] api_url`.
/// Falls back to relative `/api/v1` if the global is not set (proxy mode).
#[wasm_bindgen(inline_js = "export function __nylon_get_api_url() { return (typeof window !== 'undefined' && window.__NYLON_API_URL__) || ''; }")]
extern "C" {
    fn __nylon_get_api_url() -> String;
}

fn api_base() -> String {
    let base = __nylon_get_api_url();
    if base.is_empty() {
        "/api/v1".to_string()
    } else {
        format!("{}/api/v1", base.trim_end_matches('/'))
    }
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = Request::get(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn post<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = Request::post(&url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    resp.json::<R>().await.map_err(|e| e.to_string())
}

pub async fn put<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", api_base(), path);
    let resp = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    resp.json::<R>().await.map_err(|e| e.to_string())
}

pub async fn delete(path: &str) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    Ok(())
}
