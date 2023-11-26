use std::io::{stdin, stdout, Write};

use anyhow::Result;
use reqwest::Client;
mod config;
mod session;
mod store;
use session::Session;
use store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::load().await?;
    let (has_session, session) = Session::load(store.clone()).await?;

    let client = Client::builder().cookie_provider(store.clone()).build()?;

    if !has_session {
        println!(
            "[!] Unable to find valid session, please login (You'll recieve an email with a code to input next):"
        );
        let email = {
            print!("[.] Email: ");
            stdout().flush()?;
            let mut email = String::new();
            stdin().read_line(&mut email)?;
            email.trim().to_owned()
        };

        let session = Session::login(&client, &email).await?.session;
        println!("Initializing session: {session}");

        let code = {
            print!("[.] Authorization code: ");
            stdout().flush()?;
            let mut code = String::new();
            stdin().read_line(&mut code)?;
            code.trim().to_owned()
        };

        let id_token = Session::confirm(&client, &session, &code).await?.id_token;

        let session = Session::claim(&client, &id_token).await?;
        Session::save(store).await?;
        session
    } else {
        println!("[~] Found active session: {}", session);
        session
    };

    if let Some(user) = Session::whoami(&client).await?.email {
        println!("[~] Logged in as: {user}");
    } else {
        println!("[!] Unable to login. Please try again");
    }
    Ok(())
}
