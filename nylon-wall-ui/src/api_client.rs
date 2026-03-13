use gloo_net::http::Request;
use serde::{Serialize, de::DeserializeOwned};

const API_BASE: &str = "/api/v1";

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{}{}", API_BASE, path);
    let resp = Request::get(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn post<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", API_BASE, path);
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

#[allow(dead_code)] // available for future edit forms
pub async fn put<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let url = format!("{}{}", API_BASE, path);
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
    let url = format!("{}{}", API_BASE, path);
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    Ok(())
}
