#[cfg(feature = "std")]
mod inner {
    use serde::{Deserialize, Serialize};

    /// Supported OAuth/OIDC provider types.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum OAuthProviderType {
        Google,
        GitHub,
        Oidc,
    }

    impl OAuthProviderType {
        pub fn label(&self) -> &'static str {
            match self {
                OAuthProviderType::Google => "Google",
                OAuthProviderType::GitHub => "GitHub",
                OAuthProviderType::Oidc => "Custom OIDC",
            }
        }
    }

    /// Stored OAuth provider configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct OAuthProvider {
        pub id: u32,
        pub provider_type: OAuthProviderType,
        pub name: String,
        pub client_id: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub client_secret: String,
        pub enabled: bool,
        /// OIDC discovery URL (e.g. https://accounts.google.com/.well-known/openid-configuration)
        /// For Google/GitHub this is pre-filled automatically.
        #[serde(default)]
        pub issuer_url: String,
        /// Override authorize endpoint (optional, auto-discovered for OIDC).
        #[serde(default)]
        pub authorize_url: String,
        /// Override token endpoint (optional, auto-discovered for OIDC).
        #[serde(default)]
        pub token_url: String,
        /// Override userinfo endpoint (optional).
        #[serde(default)]
        pub userinfo_url: String,
        /// OAuth scopes.
        #[serde(default)]
        pub scopes: Vec<String>,
    }

    impl OAuthProvider {
        /// Fill well-known URLs for built-in providers.
        pub fn fill_defaults(&mut self) {
            match self.provider_type {
                OAuthProviderType::Google => {
                    if self.authorize_url.is_empty() {
                        self.authorize_url =
                            "https://accounts.google.com/o/oauth2/v2/auth".to_string();
                    }
                    if self.token_url.is_empty() {
                        self.token_url =
                            "https://oauth2.googleapis.com/token".to_string();
                    }
                    if self.userinfo_url.is_empty() {
                        self.userinfo_url =
                            "https://openidconnect.googleapis.com/v1/userinfo".to_string();
                    }
                    if self.scopes.is_empty() {
                        self.scopes = vec!["openid".to_string(), "email".to_string()];
                    }
                }
                OAuthProviderType::GitHub => {
                    if self.authorize_url.is_empty() {
                        self.authorize_url =
                            "https://github.com/login/oauth/authorize".to_string();
                    }
                    if self.token_url.is_empty() {
                        self.token_url =
                            "https://github.com/login/oauth/access_token".to_string();
                    }
                    if self.userinfo_url.is_empty() {
                        self.userinfo_url = "https://api.github.com/user".to_string();
                    }
                    if self.scopes.is_empty() {
                        self.scopes = vec!["read:user".to_string(), "user:email".to_string()];
                    }
                }
                OAuthProviderType::Oidc => {
                    // Custom OIDC — user must provide URLs
                    if self.scopes.is_empty() {
                        self.scopes = vec!["openid".to_string(), "email".to_string()];
                    }
                }
            }
        }
    }
}

#[cfg(feature = "std")]
pub use inner::{OAuthProvider, OAuthProviderType};
