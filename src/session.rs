use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::Arc,
};

use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, ACCEPT},
    Client,
};
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};

use crate::config::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub session: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UserData {
    pub email: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub id_token: String,
}

pub struct Session;

impl Session {
    pub async fn load(cookie_store: Arc<CookieStoreMutex>) -> Result<(bool, String)> {
        let session_id = {
            let store = cookie_store.lock().unwrap();
            store
                .get(RPC_ENDPOINT, "/", SESSION_COOKIE_NAME)
                .map(|cookie| cookie.value().to_string())
        };

        match &session_id {
            Some(token) => Ok((true, token.clone())),
            None => Ok((false, String::new())),
        }
    }

    pub async fn save(cookies: Arc<CookieStoreMutex>) -> Result<()> {
        let mut writer = File::create(COOKIES_PATH).map(BufWriter::new)?;
        let store = cookies.lock().unwrap();
        store.save_json(&mut writer).unwrap();
        writer.flush()?;
        println!("Session cookie saved to {COOKIES_PATH}");
        Ok(())
    }

    pub async fn claim(client: &Client, id_token: &str) -> Result<String> {
        let params = [("id_token", id_token)];
        let mut map = HeaderMap::new();
        map.insert(ACCEPT, "application/json".parse()?);

        let res = client
            .post(format!("{RPC_ADDR}/claim"))
            .form(&params)
            .headers(map)
            .send()
            .await
            .context("Failed to claim session")?;

        res.text()
            .await
            .context("Failed to read verification response")
    }

    pub async fn login(client: &Client, email: &str) -> Result<AuthResponse> {
        let params = [("email", email)];
        let mut map = HeaderMap::new();
        map.insert(ACCEPT, "application/json".parse()?);

        let res = client
            .post(format!("{RPC_ADDR}/login"))
            .form(&params)
            .headers(map)
            .send()
            .await
            .context("Failed to send request")?;

        res.json::<AuthResponse>()
            .await
            .context("Failed to parse response as JSON")
    }

    pub async fn confirm(client: &Client, session: &str, code: &str) -> Result<VerifyResponse> {
        let params = [("session", session), ("code", code)];
        let mut map = HeaderMap::new();
        map.insert(ACCEPT, "application/json".parse()?);

        let res = client
            .post(format!("{BROKER_ADDR}/confirm"))
            .form(&params)
            .headers(map)
            .send()
            .await
            .context("Failed to confirm session")?;
        res.json::<VerifyResponse>()
            .await
            .context("Failed to parse confirmation response as JSON")
    }

    pub async fn whoami(client: &Client) -> Result<UserData> {
        let mut map = HeaderMap::new();
        map.insert(ACCEPT, "application/json".parse()?);

        let res = client
            .get(format!("{RPC_ADDR}/whoami"))
            .headers(map)
            .send()
            .await
            .context("Failed to send request")?;

        res.json::<UserData>()
            .await
            .context("Failed to parse user data as JSON")
    }
}
