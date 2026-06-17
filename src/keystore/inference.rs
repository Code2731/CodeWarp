// keystore_inference.rs — Inference binary / command / model dir (main.rs child module)
use keyring;

const SERVICE: &str = "codewarp";
const INFERENCE_CMD_USER: &str = "inference_start_command";
const INFERENCE_BIN_USER: &str = "inference_binary_path";
const MODEL_DIR_USER: &str = "model_download_dir";

fn inference_cmd_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, INFERENCE_CMD_USER).map_err(|e| e.to_string())
}

pub fn read_inference_command() -> Option<String> {
    inference_cmd_entry().ok()?.get_password().ok()
}

pub fn write_inference_command(cmd: &str) -> Result<(), String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return clear_inference_command();
    }
    inference_cmd_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_inference_command() -> Result<(), String> {
    match inference_cmd_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn inference_bin_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, INFERENCE_BIN_USER).map_err(|e| e.to_string())
}

pub fn read_inference_binary() -> Option<String> {
    inference_bin_entry().ok()?.get_password().ok()
}

pub fn write_inference_binary(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return clear_inference_binary();
    }
    inference_bin_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}

pub fn clear_inference_binary() -> Result<(), String> {
    match inference_bin_entry()?.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn model_dir_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, MODEL_DIR_USER).map_err(|e| e.to_string())
}

pub fn read_model_dir() -> Option<String> {
    model_dir_entry().ok()?.get_password().ok()
}

pub fn write_model_dir(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    model_dir_entry()?
        .set_password(trimmed)
        .map_err(|e| e.to_string())
}
