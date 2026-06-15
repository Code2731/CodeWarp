use serde::Deserialize;

pub(crate) const HF_BASE: &str = "https://huggingface.co";
pub(crate) const PROGRESS_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
pub(crate) struct ModelInfo {
    pub(crate) siblings: Vec<Sibling>,
}

#[derive(Deserialize)]
pub(crate) struct Sibling {
    pub(crate) rfilename: String,
}

#[derive(Deserialize)]
pub(crate) struct TreeEntry {
    pub(crate) path: String,
    #[serde(rename = "type")]
    pub(crate) kind: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RepoRefs {
    pub(crate) branches: Vec<RepoBranch>,
}

#[derive(Deserialize)]
pub(crate) struct RepoBranch {
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Started {
        total_files: usize,
    },
    FileStart {
        idx: usize,
        name: String,
        size: Option<u64>,
    },
    FileProgress {
        idx: usize,
        bytes_done: u64,
        bytes_total: Option<u64>,
    },
    FileDone,
    AllDone,
    Error(String),
}
