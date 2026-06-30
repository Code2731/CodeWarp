use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 4;
const MAX_ITEMS: usize = 500;

#[derive(Debug, Clone)]
pub(crate) struct FileTreeItem {
    pub(crate) depth: usize,
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_dir: bool,
}

pub(crate) fn scan_file_tree(root: &Path) -> Vec<FileTreeItem> {
    let mut items = Vec::new();
    walk_dir(root, 0, &mut items);
    items
}

fn walk_dir(dir: &Path, depth: usize, items: &mut Vec<FileTreeItem>) {
    if depth > MAX_DEPTH || items.len() >= MAX_ITEMS {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dirs: Vec<_> = Vec::new();
    let mut files: Vec<_> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name.starts_with('.') || name.starts_with("node_modules") || name == "target" {
            continue;
        }
        if path.is_dir() {
            dirs.push((name, path));
        } else if is_text_file(&name) {
            files.push((name, path));
        }
    }
    dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    for (name, path) in &dirs {
        items.push(FileTreeItem {
            depth,
            name: name.clone(),
            path: path.clone(),
            is_dir: true,
        });
        walk_dir(path, depth + 1, items);
    }
    for (name, path) in &files {
        items.push(FileTreeItem {
            depth,
            name: name.clone(),
            path: path.clone(),
            is_dir: false,
        });
        if items.len() >= MAX_ITEMS {
            break;
        }
    }
}

fn is_text_file(name: &str) -> bool {
    let exts = [
        "rs",
        "toml",
        "md",
        "txt",
        "json",
        "yaml",
        "yml",
        "xml",
        "html",
        "css",
        "js",
        "ts",
        "jsx",
        "tsx",
        "py",
        "rb",
        "go",
        "java",
        "kt",
        "swift",
        "c",
        "cpp",
        "h",
        "hpp",
        "sql",
        "sh",
        "bash",
        "zsh",
        "ps1",
        "bat",
        "cfg",
        "ini",
        "env",
        "lock",
        "gradle",
        "properties",
        "conf",
        "dockerfile",
        "gitignore",
        "gitattributes",
    ];
    let lower = name.to_lowercase();
    // Check common text extensions
    if let Some(dot) = lower.rfind('.') {
        let ext = &lower[dot + 1..];
        if exts.contains(&ext) {
            return true;
        }
    }
    // Also accept files without extension like "Dockerfile", "Makefile"
    let no_ext_names = ["dockerfile", "makefile", "cmakelists"];
    if no_ext_names.contains(&lower.as_str()) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_empty_dir() {
        let d = std::env::temp_dir().join("codewarp_test_empty");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let items = scan_file_tree(&d);
        assert!(items.is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn scan_skips_hidden() {
        let d = std::env::temp_dir().join("codewarp_test_hidden");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join(".hidden")).unwrap();
        fs::write(d.join("visible.txt"), "").unwrap();
        let items = scan_file_tree(&d);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "visible.txt");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn scan_dirs_first() {
        let d = std::env::temp_dir().join("codewarp_test_order");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("zzz_dir")).unwrap();
        fs::write(d.join("aaa_file.txt"), "").unwrap();
        let items = scan_file_tree(&d);
        assert_eq!(items.len(), 2);
        assert!(items[0].is_dir);
        assert!(!items[1].is_dir);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn is_text_file_known_extensions() {
        assert!(is_text_file("main.rs"));
        assert!(is_text_file("Cargo.toml"));
        assert!(is_text_file("README.md"));
        assert!(is_text_file("Dockerfile"));
        assert!(!is_text_file("image.png"));
        assert!(!is_text_file("archive.zip"));
    }
}
