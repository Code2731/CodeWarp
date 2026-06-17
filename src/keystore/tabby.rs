// keystore_tabby.rs — Tabby Base URL / Token (main.rs child module)
use keyring;

const SERVICE: &str = "codewarp";
const TABBY_URL_USER: &str = "tabby_base_url";
const TABBY_TOKEN_USER: &str = "tabby_token";

fn tabby_url_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, TABBY_URL_USER).map_err(|e| e.to_string())
}

fn tabby_token_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, TABBY_TOKEN_USER).map_err(|e| e.to_string())
}

pub fn read_tabby_base_url() -> Option<String> {
    tabby_url_entry().ok()?.get_password().ok()
}

pub fn write_tabby_base_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return clear_tabby_base_url();
    }
    tabby_url_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_tabby_base_url() -> Result<(), String> {
    match tabby_url_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn read_tabby_token() -> Option<String> {
    tabby_token_entry().ok()?.get_password().ok()
}

pub fn write_tabby_token(token: &str) -> Result<(), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return clear_tabby_token();
    }
    tabby_token_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_tabby_token() -> Result<(), String> {
    match tabby_token_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
