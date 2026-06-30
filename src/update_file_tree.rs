use super::{App, Message, Task};

impl App {
    pub(crate) fn toggle_file_tree_dir(&mut self, path: std::path::PathBuf) -> Task<Message> {
        if !self.file_tree_expanded.remove(&path) {
            self.file_tree_expanded.insert(path);
        }
        Task::none()
    }

    pub(crate) fn refresh_file_tree(&mut self) -> Task<Message> {
        self.file_tree_items = crate::util::file_tree::scan_file_tree(&self.cwd);
        self.file_tree_expanded.clear();
        Task::none()
    }
}
