// OS Credential Manager (Windows) / Keychain (macOS) / Secret Service (Linux)에
// OpenRouter API 키를 저장.

const SERVICE: &str = "codewarp";
const USER: &str = "openrouter_api_key";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, USER).map_err(|e| e.to_string())
}

pub fn read_api_key() -> Result<String, String> {
    entry()?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => "API 키가 저장되어 있지 않습니다.".into(),
        other => other.to_string(),
    })
}

pub fn write_api_key(key: &str) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API 키가 비어 있습니다.".into());
    }
    entry()?.set_password(trimmed).map_err(|e| e.to_string())
}

pub fn delete_api_key() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn has_api_key() -> bool {
    matches!(
        keyring::Entry::new(SERVICE, USER).and_then(|e| e.get_password()),
        Ok(_)
    )
}
