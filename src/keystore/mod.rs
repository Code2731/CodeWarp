// OS Credential Manager (Windows) / Keychain (macOS) / Secret Service (Linux)에
// OpenRouter API 키를 저장.

const SERVICE: &str = "codewarp";
const USER: &str = "openrouter_api_key";
const MODEL_USER: &str = "selected_model";
const CWD_USER: &str = "working_directory";

fn humanize_keyring_error(e: keyring::Error) -> String {
    match &e {
        keyring::Error::NoStorageAccess(_) => "자격 증명 저장소에 접근할 수 없습니다.".into(),
        keyring::Error::PlatformFailure(_) => "OS 자격 증명 저장소 오류입니다.".into(),
        keyring::Error::BadEncoding(_) => "자격 증명 데이터 형식이 잘못되었습니다.".into(),
        keyring::Error::TooLong(attr, _) => {
            format!("자격 증명 속성({attr})이 길이 제한을 초과했습니다.")
        }
        keyring::Error::Invalid(attr, reason) => {
            format!("자격 증명 속성({attr})이 유효하지 않습니다: {reason}")
        }
        keyring::Error::Ambiguous(_) => "일치하는 자격 증명이 여러 개 있습니다.".into(),
        _ => e.to_string(),
    }
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, USER).map_err(humanize_keyring_error)
}

fn model_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, MODEL_USER).map_err(|e| e.to_string())
}

fn cwd_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, CWD_USER).map_err(|e| e.to_string())
}

pub fn read_api_key() -> Result<String, String> {
    entry()?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => "API 키가 저장되어 있지 않습니다.".into(),
        other => humanize_keyring_error(other),
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
    keyring::Entry::new(SERVICE, USER)
        .and_then(|e| e.get_password())
        .is_ok()
}

pub fn read_selected_model() -> Option<String> {
    model_entry().ok()?.get_password().ok()
}

pub fn write_selected_model(model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Ok(());
    }
    model_entry()?
        .set_password(model)
        .map_err(|e| e.to_string())
}

pub fn clear_selected_model() -> Result<(), String> {
    let entry = model_entry()?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn read_cwd() -> Option<String> {
    cwd_entry().ok()?.get_password().ok()
}

pub fn write_cwd(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Ok(());
    }
    cwd_entry()?.set_password(path).map_err(|e| e.to_string())
}

mod hf;
mod inference;
mod tabby;
pub use hf::*;
pub use inference::*;
pub use tabby::*;
