use anyhow::{Context, Result};
use reqwest::{header::HeaderMap, header::ACCEPT, Client};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{stdin, stdout, BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
};

const COOKIES_PATH: &str = "cookies.json";
const RPC_ENDPOINT: &str = "127.0.0.1";
const SESSION_COOKIE_NAME: &str = "id";
const RPC_ADDR: &str = "http://127.0.0.1:8000";
const BROKER_ADDR: &str = "http://127.0.0.1:3333";

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    session: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UserData {
    email: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct VerifyResponse {
    id_token: String,
}

async fn login(client: &Client, email: &str) -> Result<AuthResponse> {
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

async fn confirm_auth(client: &Client, session: &str, code: &str) -> Result<VerifyResponse> {
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

async fn claim_session(client: &Client, id_token: &str) -> Result<String> {
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

async fn whoami(client: &Client) -> Result<UserData> {
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

async fn load_or_create_store() -> Result<Arc<CookieStoreMutex>> {
    let cookie_store = if Path::new(COOKIES_PATH).exists() {
        println!("Opening cookie store located in {}", COOKIES_PATH);
        let file = File::open(COOKIES_PATH).map(BufReader::new)?;
        CookieStore::load_json(file).unwrap()
    } else {
        File::create(COOKIES_PATH)?;
        CookieStore::default()
    };

    Ok(Arc::new(CookieStoreMutex::new(cookie_store)))
}

async fn load_session(cookie_store: Arc<CookieStoreMutex>) -> Result<(bool, String)> {
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

async fn save_session(cookies: Arc<CookieStoreMutex>) -> Result<()> {
    let mut writer = File::create(COOKIES_PATH).map(BufWriter::new)?;
    let store = cookies.lock().unwrap();
    store.save_json(&mut writer).unwrap();
    writer.flush()?;
    println!("Session cookie saved to {COOKIES_PATH}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cookie_store = load_or_create_store().await?;
    let (has_session, session) = load_session(cookie_store.clone()).await?;

    let client = Client::builder()
        .cookie_provider(cookie_store.clone())
        .build()?;

    if !has_session {
        println!("[!] Unable to find valid session, please login (You'll recieve an email with a code to input next):");
        let email = {
            print!("[.] Email: ");
            stdout().flush()?;
            let mut email = String::new();
            stdin().read_line(&mut email)?;
            email.trim().to_owned()
        };

        let session = login(&client, &email).await?.session;
        println!("Initializing session: {session}");

        let code = {
            print!("[.] Authorization code: ");
            stdout().flush()?;
            let mut code = String::new();
            stdin().read_line(&mut code)?;
            code.trim().to_owned()
        };

        let id_token = confirm_auth(&client, &session, &code).await?.id_token;

        let session = claim_session(&client, &id_token).await?;
        save_session(cookie_store).await?;
        session
    } else {
        println!("[~] Found active session: {}", session);
        session
    };

    if let Some(user) = whoami(&client).await?.email {
        println!("[~] Logged in as: {user}");
    } else {
        println!("[!] Unable to login. Please try again");
    }
    Ok(())
}
