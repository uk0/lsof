use crate::model::{OpenFileInfo, ProcessInfo};
use crate::platform::PlatformProvider;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::widgets::{ListState, TableState};

pub enum ViewMode {
    Search,
    Detail,
}

pub enum DetailTab {
    OpenFiles,
    Network,
    FileTree,
    Summary,
}

pub struct AppState {
    pub mode: ViewMode,
    pub search_input: String,
    pub all_processes: Vec<ProcessInfo>,
    pub filtered_indices: Vec<usize>,
    pub list_state: ListState,
    pub selected_process: Option<ProcessInfo>,
    pub detail_tab: DetailTab,
    pub file_table_state: TableState,
    pub tree_list_state: ListState,
    pub should_quit: bool,
    #[allow(dead_code)]
    pub loading: bool,
    pub match_count: usize,
    pub total_count: usize,
    pub export_data: Option<String>,
}

impl AppState {
    pub fn new(processes: Vec<ProcessInfo>) -> Self {
        let total_count = processes.len();
        let filtered_indices: Vec<usize> = (0..total_count).collect();
        let match_count = total_count;

        let mut list_state = ListState::default();
        if !filtered_indices.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            mode: ViewMode::Search,
            search_input: String::new(),
            all_processes: processes,
            filtered_indices,
            list_state,
            selected_process: None,
            detail_tab: DetailTab::OpenFiles,
            file_table_state: TableState::default(),
            tree_list_state: ListState::default(),
            should_quit: false,
            loading: false,
            match_count,
            total_count,
            export_data: None,
        }
    }
    /// Apply fuzzy search to the process list based on current search_input.
    pub fn update_filter(&mut self) {
        if self.search_input.is_empty() {
            self.filtered_indices = (0..self.all_processes.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let query = &self.search_input;

            let mut scored: Vec<(usize, i64)> = self
                .all_processes
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let haystack = format!("{} {} {}", p.pid, p.comm, p.user);
                    matcher
                        .fuzzy_match(&haystack, query)
                        .map(|score| (i, score))
                })
                .collect();

            // Sort by score descending (best matches first)
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        }

        self.match_count = self.filtered_indices.len();

        // Reset selection to first item or none
        if self.filtered_indices.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    /// Reload the process list from the platform provider.
    pub fn refresh(&mut self, provider: &dyn PlatformProvider) {
        if let Ok(processes) = provider.list_processes() {
            self.total_count = processes.len();
            self.all_processes = processes;
            self.update_filter();
        }
    }

    /// Enter detail view for the currently selected process.
    pub fn select_current(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(&idx) = self.filtered_indices.get(selected) {
                if let Some(proc) = self.all_processes.get(idx) {
                    self.selected_process = Some(proc.clone());
                    self.mode = ViewMode::Detail;
                    self.detail_tab = DetailTab::OpenFiles;
                    self.file_table_state = TableState::default();
                    self.file_table_state.select(Some(0));
                    self.tree_list_state = ListState::default();
                    self.tree_list_state.select(Some(0));
                }
            }
        }
    }

    /// Return to search view from detail view.
    pub fn go_back(&mut self) {
        self.mode = ViewMode::Search;
        self.selected_process = None;
    }

    /// Switch to the next detail tab.
    pub fn next_tab(&mut self) {
        self.detail_tab = match self.detail_tab {
            DetailTab::OpenFiles => DetailTab::Network,
            DetailTab::Network => DetailTab::FileTree,
            DetailTab::FileTree => DetailTab::Summary,
            DetailTab::Summary => DetailTab::OpenFiles,
        };
        self.reset_detail_scroll();
    }

    /// Switch to the previous detail tab.
    pub fn prev_tab(&mut self) {
        self.detail_tab = match self.detail_tab {
            DetailTab::OpenFiles => DetailTab::Summary,
            DetailTab::Network => DetailTab::OpenFiles,
            DetailTab::FileTree => DetailTab::Network,
            DetailTab::Summary => DetailTab::FileTree,
        };
        self.reset_detail_scroll();
    }

    fn reset_detail_scroll(&mut self) {
        self.file_table_state = TableState::default();
        self.file_table_state.select(Some(0));
        self.tree_list_state = ListState::default();
        self.tree_list_state.select(Some(0));
    }

    /// Move selection up by one.
    pub fn move_up(&mut self) {
        match self.mode {
            ViewMode::Search => self.search_move(-1),
            ViewMode::Detail => self.detail_move(-1),
        }
    }

    /// Move selection down by one.
    pub fn move_down(&mut self) {
        match self.mode {
            ViewMode::Search => self.search_move(1),
            ViewMode::Detail => self.detail_move(1),
        }
    }

    /// Move selection up by a page (10 items).
    pub fn page_up(&mut self) {
        match self.mode {
            ViewMode::Search => self.search_move(-10),
            ViewMode::Detail => self.detail_move(-10),
        }
    }

    /// Move selection down by a page (10 items).
    pub fn page_down(&mut self) {
        match self.mode {
            ViewMode::Search => self.search_move(10),
            ViewMode::Detail => self.detail_move(10),
        }
    }

    fn search_move(&mut self, delta: i32) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let len = self.filtered_indices.len() as i32;
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = ((current + delta) % len + len) % len;
        self.list_state.select(Some(next as usize));
    }

    fn detail_move(&mut self, delta: i32) {
        let item_count = self.detail_item_count();
        if item_count == 0 {
            return;
        }
        let len = item_count as i32;

        match self.detail_tab {
            DetailTab::FileTree => {
                let current = self.tree_list_state.selected().unwrap_or(0) as i32;
                let next = (current + delta).clamp(0, len - 1);
                self.tree_list_state.select(Some(next as usize));
            }
            DetailTab::Summary => {
                // Summary is non-scrollable
            }
            _ => {
                // OpenFiles and Network use file_table_state
                let current = self.file_table_state.selected().unwrap_or(0) as i32;
                let next = (current + delta).clamp(0, len - 1);
                self.file_table_state.select(Some(next as usize));
            }
        }
    }

    fn detail_item_count(&self) -> usize {
        let proc = match &self.selected_process {
            Some(p) => p,
            None => return 0,
        };
        match self.detail_tab {
            DetailTab::OpenFiles => proc.open_files.len(),
            DetailTab::Network => proc
                .open_files
                .iter()
                .filter(|f| {
                    matches!(
                        f.file_type,
                        crate::model::FileType::IPv4
                            | crate::model::FileType::IPv6
                            | crate::model::FileType::Sock
                            | crate::model::FileType::Unix
                    )
                })
                .count(),
            DetailTab::FileTree => {
                // Approximate: directories + files
                proc.open_files.len() + 10
            }
            DetailTab::Summary => 0,
        }
    }

    /// Get the currently selected line text from the detail view for yanking.
    pub fn yank_selected_line(&self, open_files: &[OpenFileInfo]) -> Option<String> {
        match self.detail_tab {
            DetailTab::OpenFiles => {
                let idx = self.file_table_state.selected()?;
                let file = open_files.get(idx)?;
                Some(format!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    file.fd,
                    file.file_type,
                    file.device,
                    file.size_off.map(|s| s.to_string()).unwrap_or_default(),
                    file.node,
                    file.name
                ))
            }
            _ => None,
        }
    }

    /// Export full process data as formatted text.
    pub fn export_process_data(
        &self,
        process: &ProcessInfo,
        open_files: &[OpenFileInfo],
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!("PID: {}\n", process.pid));
        out.push_str(&format!("COMMAND: {}\n", process.comm));
        out.push_str(&format!("USER: {}\n", process.user));
        if let Some(ppid) = process.ppid {
            out.push_str(&format!("PPID: {}\n", ppid));
        }
        out.push_str(&format!("\nOpen Files ({}):\n", open_files.len()));
        out.push_str("FD\tTYPE\tDEVICE\tSIZE/OFF\tNODE\tNAME\n");
        for f in open_files {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                f.fd,
                f.file_type,
                f.device,
                f.size_off.map(|s| s.to_string()).unwrap_or_default(),
                f.node,
                f.name
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::open_file::{FdMode, FdType, FileType, OpenFileInfo};
    use crate::model::process::ProcessInfo;

    fn make_test_file(name: &str) -> OpenFileInfo {
        OpenFileInfo {
            fd: FdType::Numbered(1, FdMode::Read),
            file_type: FileType::Reg,
            device: "1,5".into(),
            size_off: Some(1024),
            node: "12345".into(),
            name: name.into(),
            mode: Some(FdMode::Read),
            link_target: None,
            send_queue: None,
            recv_queue: None,
        }
    }

    fn make_test_process() -> ProcessInfo {
        ProcessInfo {
            pid: 1234,
            command: "test-cmd".into(),
            comm: "test".into(),
            user: "root".into(),
            uid: 0,
            ppid: Some(1),
            pgid: None,
            open_files: vec![],
        }
    }

    #[test]
    fn test_export_process_data() {
        let app = AppState::new(vec![]);
        let process = make_test_process();
        let files = vec![make_test_file("/tmp/test.txt")];
        let data = app.export_process_data(&process, &files);
        assert!(data.contains("PID: 1234"));
        assert!(data.contains("COMMAND: test"));
        assert!(data.contains("USER: root"));
        assert!(data.contains("PPID: 1"));
        assert!(data.contains("/tmp/test.txt"));
        assert!(data.contains("Open Files (1):"));
    }

    #[test]
    fn test_export_process_data_no_ppid() {
        let app = AppState::new(vec![]);
        let mut process = make_test_process();
        process.ppid = None;
        let files = vec![];
        let data = app.export_process_data(&process, &files);
        assert!(data.contains("PID: 1234"));
        assert!(!data.contains("PPID:"));
        assert!(data.contains("Open Files (0):"));
    }

    #[test]
    fn test_yank_selected_line_open_files_tab() {
        let mut app = AppState::new(vec![]);
        app.detail_tab = DetailTab::OpenFiles;
        app.file_table_state.select(Some(0));
        let files = vec![make_test_file("/tmp/test.txt")];
        let line = app.yank_selected_line(&files);
        assert!(line.is_some());
        let line = line.unwrap();
        assert!(line.contains("/tmp/test.txt"));
        assert!(line.contains("REG"));
        assert!(line.contains("12345"));
    }

    #[test]
    fn test_yank_selected_line_no_selection() {
        let mut app = AppState::new(vec![]);
        app.detail_tab = DetailTab::OpenFiles;
        app.file_table_state.select(None);
        let files = vec![make_test_file("/tmp/test.txt")];
        let line = app.yank_selected_line(&files);
        assert!(line.is_none());
    }

    #[test]
    fn test_yank_selected_line_non_openfiles_tab() {
        let mut app = AppState::new(vec![]);
        app.detail_tab = DetailTab::Network;
        app.file_table_state.select(Some(0));
        let files = vec![make_test_file("/tmp/test.txt")];
        let line = app.yank_selected_line(&files);
        assert!(line.is_none());
    }

    #[test]
    fn test_export_data_field_default_none() {
        let app = AppState::new(vec![]);
        assert!(app.export_data.is_none());
    }
}
