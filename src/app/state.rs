use ratatui::widgets::{ListState, TableState};
use crate::model::ProcessInfo;
use crate::platform::PlatformProvider;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

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
    pub loading: bool,
    pub match_count: usize,
    pub total_count: usize,
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
                    matcher.fuzzy_match(&haystack, query).map(|score| (i, score))
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
                .filter(|f| matches!(
                    f.file_type,
                    crate::model::FileType::IPv4
                        | crate::model::FileType::IPv6
                        | crate::model::FileType::Sock
                        | crate::model::FileType::Unix
                ))
                .count(),
            DetailTab::FileTree => {
                // Approximate: directories + files
                proc.open_files.len() + 10
            }
            DetailTab::Summary => 0,
        }
    }
}
