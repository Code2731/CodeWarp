// OS Credential Manager (Windows) / Keychain (macOS) / Secret Service (Linux)에
// 키/토큰 저장.

#[macro_export]
macro_rules! keystore_entry_std {
    ($entry_fn:ident, $read_fn:ident, $write_fn:ident, $clear_fn:ident, $user:literal) => {
        fn $entry_fn() -> Result<keyring::Entry, String> {
            keyring::Entry::new("codewarp", $user).map_err(|e| e.to_string())
        }
        pub(crate) fn $read_fn() -> Option<String> {
            $entry_fn().ok()?.get_password().ok()
        }
        pub(crate) fn $write_fn(value: &str) -> Result<(), String> {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return $clear_fn();
            }
            $entry_fn()?
                .set_password(trimmed)
                .map_err(|e| e.to_string())
        }
        pub(crate) fn $clear_fn() -> Result<(), String> {
            match $entry_fn()?.delete_credential() {
                Ok(_) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    };
}

#[macro_export]
macro_rules! keystore_entry_no_clear {
    ($entry_fn:ident, $read_fn:ident, $write_fn:ident, $user:literal) => {
        fn $entry_fn() -> Result<keyring::Entry, String> {
            keyring::Entry::new("codewarp", $user).map_err(|e| e.to_string())
        }
        pub(crate) fn $read_fn() -> Option<String> {
            $entry_fn().ok()?.get_password().ok()
        }
        pub(crate) fn $write_fn(value: &str) -> Result<(), String> {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(());
            }
            $entry_fn()?
                .set_password(trimmed)
                .map_err(|e| e.to_string())
        }
    };
}
#[macro_export]
macro_rules! keystore_entry_no_clear_with_clear {
    ($entry_fn:ident, $read_fn:ident, $write_fn:ident, $clear_fn:ident, $user:literal) => {
        keystore_entry_no_clear!($entry_fn, $read_fn, $write_fn, $user);
        pub(crate) fn $clear_fn() -> Result<(), String> {
            match $entry_fn()?.delete_credential() {
                Ok(_) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    };
}

const SERVICE: &str = "codewarp";
const USER: &str = "openrouter_api_key";
const CWD_USER: &str = "working_directory";

fn humanize_keyring_error(e: &keyring::Error) -> String {
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
    keyring::Entry::new(SERVICE, USER).map_err(|e| humanize_keyring_error(&e))
}

fn cwd_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, CWD_USER).map_err(|e| e.to_string())
}

pub(crate) fn read_api_key() -> Result<String, String> {
    entry()?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => "API 키가 저장되어 있지 않습니다.".into(),
        ref other => humanize_keyring_error(other),
    })
}

pub(crate) fn write_api_key(key: &str) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API 키가 비어 있습니다.".into());
    }
    entry()?.set_password(trimmed).map_err(|e| e.to_string())
}

pub(crate) fn delete_api_key() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn has_api_key() -> bool {
    keyring::Entry::new(SERVICE, USER)
        .and_then(|e| e.get_password())
        .is_ok()
}

keystore_entry_no_clear_with_clear!(
    model_entry,
    read_selected_model,
    write_selected_model,
    clear_selected_model,
    "selected_model"
);

pub(crate) fn read_cwd() -> Option<String> {
    cwd_entry().ok()?.get_password().ok()
}

pub(crate) fn write_cwd(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Ok(());
    }
    cwd_entry()?.set_password(path).map_err(|e| e.to_string())
}

mod hf;
mod inference;
mod tabby;
pub(crate) use hf::*;
pub(crate) use inference::*;
pub(crate) use tabby::*;
