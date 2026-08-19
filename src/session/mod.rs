use std::fs;
use std::io::Read;
use std::path::Path;

mod favorites;
mod persist;
#[cfg(test)]
mod persist_atomic_tests;
#[cfg(test)]
mod persist_tests;
mod recovery;
mod theme;
mod theme_contrast;
mod usage;

const MAX_AUXILIARY_FILE_BYTES: u64 = 1024 * 1024;

fn read_auxiliary_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_AUXILIARY_FILE_BYTES {
        return None;
    }
    let capacity = usize::try_from(metadata.len()).ok()?;
    let mut bytes = Vec::with_capacity(capacity);
    fs::File::open(path)
        .ok()?
        .take(MAX_AUXILIARY_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_AUXILIARY_FILE_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}

pub(crate) use favorites::{read_favorites, write_favorites};
pub(crate) use persist::{
    PersistedAllSessions, PersistedBlock, PersistedSessionData, SessionLoadNotice,
    load_all_with_notice, save_all,
};
#[cfg(test)]
pub(crate) use persist::{load_all_at, load_all_with_notice_at, save_all_at};
pub(crate) use recovery::{mark_clean_shutdown, was_clean_shutdown};
#[cfg(test)]
pub(crate) use recovery::{mark_clean_shutdown_at, was_clean_shutdown_at};
pub(crate) use theme::{ThemeConfig, read_theme, theme_presets, write_theme};
pub(crate) use usage::{ModelUsage, UsageStore, load_usage, save_usage};

#[cfg(test)]
mod tests {
    use super::{MAX_AUXILIARY_FILE_BYTES, read_auxiliary_text};
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn oversized_auxiliary_state_is_ignored_without_reading_contents() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("state.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_AUXILIARY_FILE_BYTES + 1).unwrap();

        assert!(read_auxiliary_text(&path).is_none());
    }
}
