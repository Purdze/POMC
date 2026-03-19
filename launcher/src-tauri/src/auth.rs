use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "00000000441cc96b";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthAccount {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub expires_at: u64,
}

#[derive(Serialize, Clone)]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct MsaTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblXui>,
}

#[derive(Deserialize)]
struct XblXui {
    uhs: String,
}

#[derive(Deserialize)]
struct McAuthResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

const KEYRING_SERVICE: &str = "pomc-launcher";
const KEYRING_ACCOUNTS: &str = "minecraft-accounts";
const KEYRING_REFRESH: &str = "minecraft-refresh-tokens";

pub fn get_all_accounts() -> Vec<AuthAccount> {
    let entry = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNTS) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let json = match entry.get_password() {
        Ok(j) => j,
        Err(_) => return vec![],
    };
    serde_json::from_str(&json).unwrap_or_default()
}

fn get_refresh_tokens() -> std::collections::HashMap<String, String> {
    let entry = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH) {
        Ok(e) => e,
        Err(_) => return Default::default(),
    };
    let json = match entry.get_password() {
        Ok(j) => j,
        Err(_) => return Default::default(),
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub fn try_restore(uuid: &str) -> Option<AuthAccount> {
    get_all_accounts()
        .into_iter()
        .find(|a| a.uuid == uuid && a.expires_at > unix_now())
}

pub async fn try_refresh(uuid: &str) -> Option<AuthAccount> {
    let tokens = get_refresh_tokens();
    let refresh_token = tokens.get(uuid)?;
    refresh_msa_token(refresh_token).await.ok()
}

pub async fn try_restore_or_refresh(uuid: &str) -> Option<AuthAccount> {
    if let Some(account) = try_restore(uuid) {
        return Some(account);
    }
    try_refresh(uuid).await
}

async fn refresh_msa_token(refresh_token: &str) -> Result<AuthAccount, String> {
    let client = reqwest::Client::new();

    let msa: MsaTokenResponse = client
        .post(format!(
            "https://login.live.com/oauth20_token.srf?client_id={CLIENT_ID}"
        ))
        .form(&[
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|e| format!("Refresh failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Refresh parse failed: {e}"))?;

    let account = exchange_msa_to_minecraft(&client, &msa.access_token).await?;

    if let Some(new_refresh) = &msa.refresh_token {
        save_refresh_token(&account.uuid, new_refresh);
    }

    Ok(account)
}

fn save_account(account: &AuthAccount) {
    let mut accounts = get_all_accounts();
    accounts.retain(|a| a.uuid != account.uuid);
    accounts.push(account.clone());
    if let Ok(json) = serde_json::to_string(&accounts) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNTS) {
            let _ = entry.set_password(&json);
        }
    }
}

fn save_refresh_token(uuid: &str, token: &str) {
    let mut tokens = get_refresh_tokens();
    tokens.insert(uuid.to_string(), token.to_string());
    if let Ok(json) = serde_json::to_string(&tokens) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH) {
            let _ = entry.set_password(&json);
        }
    }
}

pub fn remove_account(uuid: &str) {
    let mut accounts = get_all_accounts();
    accounts.retain(|a| a.uuid != uuid);
    if let Ok(json) = serde_json::to_string(&accounts) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNTS) {
            let _ = entry.set_password(&json);
        }
    }
    let mut tokens = get_refresh_tokens();
    tokens.remove(uuid);
    if let Ok(json) = serde_json::to_string(&tokens) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH) {
            let _ = entry.set_password(&json);
        }
    }
}

pub async fn start_device_code_flow() -> Result<(DeviceCodeInfo, String, u64, u64), String> {
    let client = reqwest::Client::new();

    let resp: DeviceCodeResponse = client
        .post("https://login.live.com/oauth20_connect.srf")
        .form(&[
            ("scope", SCOPE),
            ("client_id", CLIENT_ID),
            ("response_type", "device_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Device code request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Device code parse failed: {e}"))?;

    let login_url = format!("{}?otc={}", resp.verification_uri, resp.user_code);
    let _ = open::that(&login_url);

    Ok((
        DeviceCodeInfo {
            user_code: resp.user_code,
            verification_uri: resp.verification_uri,
        },
        resp.device_code,
        resp.expires_in,
        resp.interval,
    ))
}

pub async fn poll_for_token(
    device_code: &str,
    expires_in: u64,
    interval: u64,
) -> Result<AuthAccount, String> {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(expires_in);
    let poll_interval = Duration::from_secs(interval);

    let msa = loop {
        tokio::time::sleep(poll_interval).await;
        if tokio::time::Instant::now() > deadline {
            return Err("Authentication timed out".to_string());
        }

        let resp = client
            .post(format!(
                "https://login.live.com/oauth20_token.srf?client_id={CLIENT_ID}"
            ))
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| format!("Token poll failed: {e}"))?;

        if let Ok(token) = resp.json::<MsaTokenResponse>().await {
            break token;
        }
    };

    let account = exchange_msa_to_minecraft(&reqwest::Client::new(), &msa.access_token).await?;

    if let Some(refresh) = &msa.refresh_token {
        save_refresh_token(&account.uuid, refresh);
    }

    Ok(account)
}

async fn exchange_msa_to_minecraft(
    client: &reqwest::Client,
    msa_token: &str,
) -> Result<AuthAccount, String> {
    let xbl: XblResponse = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": msa_token,
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send()
        .await
        .map_err(|e| format!("Xbox Live auth failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Xbox Live parse failed: {e}"))?;

    let user_hash = xbl
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or("No user hash in XBL response")?;

    let xsts: XblResponse = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [&xbl.token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await
        .map_err(|e| format!("XSTS auth failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("XSTS parse failed: {e}"))?;

    let mc: McAuthResponse = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={user_hash};{}", xsts.token),
        }))
        .send()
        .await
        .map_err(|e| format!("MC auth failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("MC auth parse failed: {e}"))?;

    let profile: McProfileResponse = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc.access_token)
        .send()
        .await
        .map_err(|e| format!("Profile fetch failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Profile parse failed: {e}"))?;

    let account = AuthAccount {
        username: profile.name,
        uuid: profile.id,
        access_token: mc.access_token,
        expires_at: unix_now() + 86400,
    };

    save_account(&account);
    Ok(account)
}
