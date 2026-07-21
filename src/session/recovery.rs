use std::io::Write;
use std::path::Path;
#[cfg(not(test))]
use std::path::PathBuf;

#[cfg(not(test))]
fn codewarp_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp"))
}

#[cfg(not(test))]
pub(crate) fn mark_clean_shutdown() -> Result<(), String> {
    let dir = codewarp_dir().ok_or_else(|| "recovery marker directory unavailable".to_string())?;
    mark_clean_shutdown_at(&dir)
}

#[cfg(test)]
pub(crate) fn mark_clean_shutdown() -> Result<(), String> {
    Ok(())
}

pub(crate) fn mark_clean_shutdown_at(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| {
        format!(
            "recovery marker directory {} creation failed: {e}",
            dir.display()
        )
    })?;
    let path = dir.join(".clean_shutdown");
    let temporary = dir.join(".clean_shutdown.tmp");
    if let Err(error) = std::fs::remove_file(&path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!(
            "recovery marker {} stale cleanup failed: {error}",
            path.display()
        ));
    }
    let write_result = (|| {
        let mut marker = std::fs::File::create(&temporary).map_err(|error| {
            format!(
                "recovery marker {} creation failed: {error}",
                temporary.display()
            )
        })?;
        marker.write_all(b"clean\n").map_err(|error| {
            format!(
                "recovery marker {} write failed: {error}",
                temporary.display()
            )
        })?;
        marker.sync_all().map_err(|error| {
            format!(
                "recovery marker {} sync failed: {error}",
                temporary.display()
            )
        })?;
        drop(marker);
        std::fs::rename(&temporary, &path).map_err(|error| {
            format!(
                "recovery marker {} promotion failed: {error}",
                path.display()
            )
        })
    })();
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temporary);
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(())
}

#[cfg(not(test))]
pub(crate) fn was_clean_shutdown() -> Result<bool, String> {
    let dir = codewarp_dir().ok_or_else(|| "recovery marker directory unavailable".to_string())?;
    was_clean_shutdown_at(&dir)
}

#[cfg(test)]
pub(crate) fn was_clean_shutdown() -> Result<bool, String> {
    Ok(true)
}

pub(crate) fn was_clean_shutdown_at(dir: &Path) -> Result<bool, String> {
    let path = dir.join(".clean_shutdown");
    let exists = path
        .try_exists()
        .map_err(|e| format!("recovery marker {} inspection failed: {e}", path.display()))?;
    if exists {
        std::fs::remove_file(&path)
            .map_err(|e| format!("recovery marker {} cleanup failed: {e}", path.display()))?;
    }
    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::{mark_clean_shutdown_at, was_clean_shutdown_at};
    use tempfile::TempDir;

    #[test]
    fn clean_marker_is_consumed_once() {
        // Given
        let tmp = TempDir::new().unwrap();
        mark_clean_shutdown_at(tmp.path()).unwrap();

        // When
        let was_clean = was_clean_shutdown_at(tmp.path()).unwrap();

        // Then
        assert!(was_clean);
        assert!(!tmp.path().join(".clean_shutdown").exists());
    }

    #[test]
    fn missing_clean_marker_reports_unclean_shutdown() {
        // Given
        let tmp = TempDir::new().unwrap();

        // When
        let was_clean = was_clean_shutdown_at(tmp.path()).unwrap();

        // Then
        assert!(!was_clean);
    }

    #[test]
    fn unwritable_marker_root_returns_actionable_error() {
        // Given
        let tmp = TempDir::new().unwrap();
        let marker_root = tmp.path().join("not-a-directory");
        std::fs::write(&marker_root, "blocking file").unwrap();

        // When
        let error = mark_clean_shutdown_at(&marker_root).unwrap_err();

        // Then
        assert!(error.contains("recovery marker directory"));
        assert!(error.contains(&marker_root.display().to_string()));
    }
}
