use std::{fs::File, io::BufReader, path::Path, sync::Arc};

use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

use crate::{config::*, Result};
pub struct Store;
impl Store {
    pub async fn load() -> Result<Arc<CookieStoreMutex>> {
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
}
