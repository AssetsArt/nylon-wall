use std::collections::HashMap;

use nylon_wall_common::oauth::{OAuthProvider, OAuthProviderType};
use tokio::sync::Mutex;

/// CSRF state tokens — maps state → (provider_id, redirect_uri, created_at)
pub struct OAuthStateStore {
    states: Mutex<HashMap<String, (u32, std::time::Instant)>>,
}

impl OAuthStateStore {
    pub fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
        }
    }

    /// Generate a new CSRF state token for a provider.
    pub async fn create(&self, provider_id: u32) -> String {
        let state = format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());
        self.states
            .lock()
            .await
            .insert(state.clone(), (provider_id, std::time::Instant::now()));
        state
    }

    /// Consume and validate a state token. Returns provider_id if valid.
    /// Tokens expire after 10 minutes.
    pub async fn consume(&self, state: &str) -> Option<u32> {
        let mut states = self.states.lock().await;
        if let Some((provider_id, created_at)) = states.remove(state) {
            if created_at.elapsed().as_secs() < 600 {
                return Some(provider_id);
            }
        }
        None
    }

    /// Clean up expired state tokens.
    pub async fn cleanup(&self) {
        let mut states = self.states.lock().await;
        states.retain(|_, (_, created_at)| created_at.elapsed().as_secs() < 600);
    }
}

/// Build the authorization URL for a provider.
pub fn build_authorize_url(
    provider: &OAuthProvider,
    state: &str,
    redirect_uri: &str,
) -> String {
    let scopes = provider.scopes.join(" ");
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        provider.authorize_url,
        urlencoding::encode(&provider.client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&scopes),
        urlencoding::encode(state),
    )
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    provider: &OAuthProvider,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, String> {
    let client = reqwest::Client::new();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);
    params.insert("redirect_uri", redirect_uri);
    params.insert("client_id", &provider.client_id);
    params.insert("client_secret", &provider.client_secret);

    let mut req = client.post(&provider.token_url).form(&params);

    // GitHub requires Accept: application/json
    if provider.provider_type == OAuthProviderType::GitHub {
        req = req.header("Accept", "application/json");
    }

    let resp = req.send().await.map_err(|e| format!("Token exchange failed: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange error: {}", body));
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))
}

#[derive(serde::Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub id_token: Option<String>,
}

/// Fetch user info from the provider's userinfo endpoint.
pub async fn fetch_user_info(
    provider: &OAuthProvider,
    access_token: &str,
) -> Result<UserInfo, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(&provider.userinfo_url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        // GitHub API requires User-Agent
        .header("User-Agent", "NylonWall")
        .send()
        .await
        .map_err(|e| format!("Userinfo request failed: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Userinfo error: {}", body));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse userinfo: {}", e))?;

    // Extract email — different providers use different fields
    let email = json
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let name = json
        .get("name")
        .or_else(|| json.get("login")) // GitHub uses "login"
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let sub = json
        .get("sub")
        .or_else(|| json.get("id")) // GitHub uses "id"
        .map(|v| v.to_string())
        .unwrap_or_default();

    if email.is_empty() && sub.is_empty() {
        return Err("Could not extract user identity from provider".to_string());
    }

    Ok(UserInfo { sub, email, name })
}

pub struct UserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
}

/// URL-encode helper (minimal, no external crate needed).
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 3);
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}
