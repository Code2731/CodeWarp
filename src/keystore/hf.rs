// keystore_hf.rs — HuggingFace token + OpenAI compat label (main.rs child module)
use keyring;

const SERVICE: &str = "codewarp";
const OPENAI_COMPAT_LABEL_USER: &str = "openai_compat_label";
const HF_TOKEN_USER: &str = "hf_token";

fn hf_token_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, HF_TOKEN_USER).map_err(|e| e.to_string())
}

pub fn read_hf_token() -> Option<String> {
    hf_token_entry().ok()?.get_password().ok()
}

pub fn write_hf_token(token: &str) -> Result<(), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return clear_hf_token();
    }
    hf_token_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_hf_token() -> Result<(), String> {
    match hf_token_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn openai_compat_label_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, OPENAI_COMPAT_LABEL_USER).map_err(|e| e.to_string())
}

pub fn read_openai_compat_label() -> Option<String> {
    openai_compat_label_entry().ok()?.get_password().ok()
}

pub fn write_openai_compat_label(label: &str) -> Result<(), String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return clear_openai_compat_label();
    }
    openai_compat_label_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_openai_compat_label() -> Result<(), String> {
    match openai_compat_label_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
