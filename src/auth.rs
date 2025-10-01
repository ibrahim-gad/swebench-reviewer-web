use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
    pub expires_at: Option<u64>, // Added for tracking expiration
}

#[derive(Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

// In-memory session storage for OAuth states (in production, use Redis or database)
static mut OAUTH_STATES: Option<HashMap<String, OAuthState>> = None;

fn get_oauth_states() -> &'static mut HashMap<String, OAuthState> {
    unsafe {
        if OAUTH_STATES.is_none() {
            OAUTH_STATES = Some(HashMap::new());
        }
        OAUTH_STATES.as_mut().unwrap()
    }
}

// Google OAuth2 configuration
pub fn get_google_client_id() -> Result<String> {
    std::env::var("GOOGLE_CLIENT_ID")
        .or_else(|_: std::env::VarError| Ok("YOUR_GOOGLE_CLIENT_ID".to_string())) // Fallback for development
        .map_err(|e: std::env::VarError| anyhow!("GOOGLE_CLIENT_ID not set: {}", e))
}

pub fn get_google_client_secret() -> Result<String> {
    std::env::var("GOOGLE_CLIENT_SECRET")
        .or_else(|_: std::env::VarError| Ok("YOUR_GOOGLE_CLIENT_SECRET".to_string())) // Fallback for development
        .map_err(|e: std::env::VarError| anyhow!("GOOGLE_CLIENT_SECRET not set: {}", e))
}

// Generate PKCE code verifier and challenge
fn generate_pkce() -> (String, String) {
    let code_verifier = URL_SAFE_NO_PAD.encode(Uuid::new_v4().as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(
        &ring::digest::digest(&ring::digest::SHA256, code_verifier.as_bytes()).as_ref()
    );
    (code_verifier, challenge)
}

// Create OAuth2 authorization URL
pub fn create_oauth_url(redirect_uri: String) -> Result<(String, String)> {
    let client_id = get_google_client_id()?;
    let state = Uuid::new_v4().to_string();
    let (code_verifier, code_challenge) = generate_pkce();
    
    // Store OAuth state
    let oauth_state = OAuthState {
        state: state.clone(),
        code_verifier,
        redirect_uri: redirect_uri.clone(),
    };
    
    get_oauth_states().insert(state.clone(), oauth_state);
    
    let scope = "openid email profile https://www.googleapis.com/auth/drive.readonly";
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scope),
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge)
    );
    
    Ok((auth_url, state))
}

// Exchange authorization code for tokens
#[cfg(feature = "ssr")]
pub async fn exchange_code_for_tokens(code: String, state: String) -> Result<GoogleTokens> {
    use reqwest::Client;
    
    // Verify state and get stored OAuth data
    let oauth_state = get_oauth_states()
        .remove(&state)
        .ok_or_else(|| anyhow!("Invalid or expired OAuth state"))?;
    
    let client_id = get_google_client_id()?;
    let client_secret = get_google_client_secret()?;
    
    let client = Client::new();
    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", oauth_state.redirect_uri.as_str()),
        ("code_verifier", oauth_state.code_verifier.as_str()),
    ];
    
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Failed to exchange code for tokens: {}", error_text));
    }
    
    let mut tokens: GoogleTokens = response.json().await?;
    
    // Calculate expiration time
    if let Some(expires_in) = tokens.expires_in {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        tokens.expires_at = Some(now + expires_in);
    }
    
    Ok(tokens)
}

// Refresh access token using refresh token
#[cfg(feature = "ssr")]
pub async fn refresh_access_token(tokens: &GoogleTokens) -> Result<GoogleTokens> {
    use reqwest::Client;
    
    let client_id = get_google_client_id()?;
    let client_secret = get_google_client_secret()?;
    
    let client = Client::new();
    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("refresh_token", tokens.refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];
    
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Failed to refresh token: {}", error_text));
    }
    
    let response_json: serde_json::Value = response.json().await?;
    
    let access_token = response_json["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("No access_token in refresh response"))?
        .to_string();
    
    let id_token = response_json["id_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    
    let expires_in = response_json["expires_in"].as_u64();
    let scope = response_json["scope"].as_str().map(|s| s.to_string());
    let token_type = response_json["token_type"].as_str().map(|s| s.to_string());
    
    // Calculate expiration time
    let expires_at = if let Some(expires_in) = expires_in {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Some(now + expires_in)
    } else {
        None
    };
    
    Ok(GoogleTokens {
        access_token,
        refresh_token: tokens.refresh_token.clone(),
        id_token,
        expires_in,
        scope,
        token_type,
        expires_at,
    })
}

// Check if token is expired
pub fn is_token_expired(tokens: &GoogleTokens) -> bool {
    if let Some(expires_at) = tokens.expires_at {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        now >= expires_at.saturating_sub(300) // Refresh 5 minutes before expiry
    } else {
        false // If no expiry info, assume it's still valid
    }
}