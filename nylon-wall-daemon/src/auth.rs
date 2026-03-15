use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::AppState;
use crate::db::Database;

const PASSWORD_KEY: &str = "auth:admin_password";
const JWT_KEY_KEY: &str = "auth:jwt_ed25519_pkcs8";
const TOKEN_EXPIRY_SECS: u64 = 86400; // 24 hours
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOCKOUT_DURATION_SECS: u64 = 900; // 15 minutes

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

/// Pre-computed JWT signing/verification keys (Ed25519).
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

// === Brute-force protection ===

struct AttemptRecord {
    count: u32,
    first_attempt: Instant,
}

pub struct LoginTracker {
    attempts: Mutex<HashMap<IpAddr, AttemptRecord>>,
}

impl LoginTracker {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Check if an IP is currently locked out. Returns remaining seconds if locked.
    pub async fn check_lockout(&self, ip: IpAddr) -> Option<u64> {
        let attempts = self.attempts.lock().await;
        if let Some(record) = attempts.get(&ip) {
            if record.count >= MAX_LOGIN_ATTEMPTS {
                let elapsed = record.first_attempt.elapsed().as_secs();
                if elapsed < LOCKOUT_DURATION_SECS {
                    return Some(LOCKOUT_DURATION_SECS - elapsed);
                }
            }
        }
        None
    }

    /// Record a failed login attempt. Returns remaining seconds if now locked out.
    pub async fn record_failure(&self, ip: IpAddr) -> Option<u64> {
        let mut attempts = self.attempts.lock().await;
        let record = attempts.entry(ip).or_insert(AttemptRecord {
            count: 0,
            first_attempt: Instant::now(),
        });

        // Reset if lockout window has expired
        if record.first_attempt.elapsed().as_secs() >= LOCKOUT_DURATION_SECS {
            record.count = 0;
            record.first_attempt = Instant::now();
        }

        record.count += 1;

        if record.count >= MAX_LOGIN_ATTEMPTS {
            let elapsed = record.first_attempt.elapsed().as_secs();
            Some(LOCKOUT_DURATION_SECS - elapsed)
        } else {
            None
        }
    }

    /// Clear attempts for an IP after successful login.
    pub async fn clear(&self, ip: IpAddr) {
        self.attempts.lock().await.remove(&ip);
    }
}

// === Password management ===

pub async fn is_setup_required(db: &Database) -> bool {
    db.get::<String>(PASSWORD_KEY).await.ok().flatten().is_none()
}

pub async fn set_password(db: &Database, plaintext: &str) -> Result<(), String> {
    let hash = bcrypt::hash(plaintext, 12).map_err(|e| e.to_string())?;
    db.put(PASSWORD_KEY, &hash).await.map_err(|e| e.to_string())
}

pub async fn verify_password(db: &Database, plaintext: &str) -> bool {
    match db.get::<String>(PASSWORD_KEY).await {
        Ok(Some(hash)) => bcrypt::verify(plaintext, &hash).unwrap_or(false),
        _ => false,
    }
}

// === Ed25519 JWT keys ===

pub async fn load_or_create_jwt_keys(db: &Database) -> JwtKeys {
    // Try to load existing PKCS8 private key
    let pkcs8_bytes = match db.get::<Vec<u8>>(JWT_KEY_KEY).await {
        Ok(Some(bytes)) => bytes,
        _ => {
            // Generate new Ed25519 keypair
            let rng = ring::rand::SystemRandom::new();
            let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)
                .expect("Ed25519 key generation failed");
            let bytes = pkcs8.as_ref().to_vec();
            let _ = db.put(JWT_KEY_KEY, &bytes).await;
            bytes
        }
    };

    // Derive public key from private key
    use ring::signature::KeyPair;
    let key_pair = ring::signature::Ed25519KeyPair::from_pkcs8(&pkcs8_bytes)
        .expect("Invalid Ed25519 PKCS8 key");
    let pub_key_bytes = key_pair.public_key().as_ref().to_vec();

    JwtKeys {
        encoding: EncodingKey::from_ed_der(&pkcs8_bytes),
        decoding: DecodingKey::from_ed_der(&pub_key_bytes),
    }
}

// === JWT token ===

pub fn create_token(keys: &JwtKeys) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let jti = format!("{:x}", rand::random::<u64>());
    let claims = Claims {
        sub: "admin".to_string(),
        exp: now + TOKEN_EXPIRY_SECS as usize,
        iat: now,
        jti,
    };
    let header = Header::new(Algorithm::EdDSA);
    jsonwebtoken::encode(&header, &claims, &keys.encoding)
        .map_err(|e| e.to_string())
}

pub fn validate_token(keys: &JwtKeys, token: &str) -> Result<Claims, String> {
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_required_spec_claims(&["exp", "sub"]);
    jsonwebtoken::decode::<Claims>(token, &keys.decoding, &validation)
        .map(|data| data.claims)
        .map_err(|e| e.to_string())
}

// === Middleware ===

/// Extract Bearer token from Authorization header or `token` query parameter.
fn extract_token(headers: &HeaderMap, uri: &axum::http::Uri) -> Option<String> {
    // Try Authorization header first
    if let Some(auth) = headers.get("authorization") {
        if let Ok(val) = auth.to_str() {
            if let Some(token) = val.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    // Fall back to query parameter (for WebSocket)
    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            if let Some(token) = pair.strip_prefix("token=") {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // If no password is set (first-run), skip auth
    if is_setup_required(&state.db).await {
        return Ok(next.run(request).await);
    }

    let token = extract_token(request.headers(), request.uri())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = validate_token(&state.jwt_keys, &token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if token is revoked
    if state.revoked_tokens.read().await.contains(&claims.jti) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
