use super::App;
use crate::block::ApplyCandidate;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct TldrFileEntry {
    pub(crate) path: String,
    pub(crate) is_new_file: bool,
    pub(crate) proposed_lines: usize,
    pub(crate) existing_lines: usize,
}

pub(crate) fn compute_tldr(
    cwd: &Path,
    candidates: &[(ApplyCandidate, bool)],
) -> Vec<TldrFileEntry> {
    candidates
        .iter()
        .map(|(cand, _)| {
            let full = cwd.join(&cand.path);
            let proposed_lines = cand.content.lines().count();
            let (is_new_file, existing_lines) = if full.is_file() {
                let existing = std::fs::read_to_string(&full)
                    .map(|s| s.lines().count())
                    .unwrap_or(0);
                (false, existing)
            } else {
                (true, 0)
            };
            TldrFileEntry {
                path: cand.path.clone(),
                is_new_file,
                proposed_lines,
                existing_lines,
            }
        })
        .collect()
}

impl App {
    pub(crate) fn toggle_tldr_view(&mut self, block_id: u64) {
        if self.tldr_expanded.remove(&block_id) {
            return;
        }
        self.tldr_expanded.insert(block_id);
        if !self.tldr_data.contains_key(&block_id)
            && let Some(block) = self.blocks.iter().find(|b| b.id == block_id)
        {
            let entries = compute_tldr(&self.cwd, &block.apply_candidates);
            self.tldr_data.insert(block_id, entries);
        }
    }
}
