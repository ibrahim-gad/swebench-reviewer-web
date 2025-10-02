use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

// Global token cache
#[cfg(feature = "ssr")]
static ACCESS_TOKEN_CACHE: once_cell::sync::Lazy<Arc<Mutex<Option<(String, u64)>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Get a valid access token for Google Drive API using service account
#[cfg(feature = "ssr")]
pub async fn get_access_token() -> Result<String> {
    // Check if we have a cached token that's still valid
    {
        let cache = ACCESS_TOKEN_CACHE.lock().unwrap();
        if let Some((token, expires_at)) = cache.as_ref() {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            // Token is valid if it expires more than 5 minutes from now
            if *expires_at > now + 300 {
                return Ok(token.clone());
            }
        }
    }

    // Get new token
    let token = fetch_new_token().await?;
    
    Ok(token)
}

#[cfg(feature = "ssr")]
async fn fetch_new_token() -> Result<String> {
    // Read service account key file
    let credentials_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| anyhow!("GOOGLE_APPLICATION_CREDENTIALS environment variable not set"))?;

    let key_content = std::fs::read_to_string(&credentials_path)
        .map_err(|e| anyhow!("Failed to read service account key from {}: {}", credentials_path, e))?;

    let service_account: ServiceAccountKey = serde_json::from_str(&key_content)
        .map_err(|e| anyhow!("Failed to parse service account JSON: {}", e))?;

    // Create JWT for authentication
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let claims = Claims {
        iss: service_account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/drive.readonly".to_string(),
        aud: service_account.token_uri.clone(),
        exp: now + 3600, // Token expires in 1 hour
        iat: now,
    };

    let header = Header::new(Algorithm::RS256);
    
    // Remove the header and footer from the PEM key
    let private_key = service_account.private_key
        .replace("-----BEGIN PRIVATE KEY-----", "")
        .replace("-----END PRIVATE KEY-----", "")
        .replace("\n", "");

    let key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;

    let jwt = encode(&header, &claims, &key)
        .map_err(|e| anyhow!("Failed to create JWT: {}", e))?;

    // Exchange JWT for access token
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt),
    ];

    let response = client
        .post(&service_account.token_uri)
        .form(&params)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to request access token: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Failed to get access token: {}", error_text));
    }

    let token_response: serde_json::Value = response.json().await
        .map_err(|e| anyhow!("Failed to parse token response: {}", e))?;

    let access_token = token_response["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("No access_token in response"))?
        .to_string();

    let expires_in = token_response["expires_in"]
        .as_u64()
        .unwrap_or(3600);

    // Cache the token
    let expires_at = now + expires_in;
    {
        let mut cache = ACCESS_TOKEN_CACHE.lock().unwrap();
        *cache = Some((access_token.clone(), expires_at));
    }

    Ok(access_token)
}

/// Initialize service account auth (just validates that credentials exist)
#[cfg(feature = "ssr")]
pub async fn init_service_account_auth() -> Result<()> {
    let credentials_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| anyhow!("GOOGLE_APPLICATION_CREDENTIALS environment variable not set"))?;

    if !std::path::Path::new(&credentials_path).exists() {
        return Err(anyhow!("Service account key file not found at: {}", credentials_path));
    }

    // Try to get a token to validate the credentials
    get_access_token().await?;

    Ok(())
}
