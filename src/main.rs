mod app;
mod cli;
mod error;
mod event;
mod filter;
mod model;
mod output;
mod platform;
mod ui;

use std::time::Duration;

use clap::Parser;
use cli::{preprocess_args, CliArgs};
use filter::FilterConfig;
use output::OutputFormatter;
use platform::{create_provider, ProviderConfig};

use app::action::map_key_to_action;
use app::{Action, AppState};
use event::{AppEvent, EventHandler};

fn main() {
    // Preprocess args to convert +D/+d/+c into --long flags for clap.
    let raw_args: Vec<String> = std::env::args().collect();
    let processed = preprocess_args(raw_args);
    let args = CliArgs::parse_from(processed);

    let config = ProviderConfig {
        avoid_stat: args.avoid_stat,
        follow_symlinks: args.follow_symlinks,
    };
    let provider = create_provider(config);

    if args.interactive {
        if let Err(e) = run_tui(&*provider) {
            ratatui::restore();
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Build filter config and output formatter from CLI args.
    let filter_config = match FilterConfig::from_cli(&args) {
        Ok(fc) => fc,
        Err(e) => {
            eprintln!("Error parsing filters: {}", e);
            std::process::exit(1);
        }
    };
    let formatter = OutputFormatter::from_cli(&args);

    // Handle repeat mode (-r)
    let repeat_interval = args.repeat;

    loop {
        if let Err(e) = run_once(&*provider, &filter_config, &formatter) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        match repeat_interval {
            Some(secs) => {
                std::thread::sleep(std::time::Duration::from_secs(secs));
            }
            None => break,
        }
    }
}

fn run_once(
    provider: &dyn platform::PlatformProvider,
    filter_config: &FilterConfig,
    formatter: &OutputFormatter,
) -> error::Result<()> {
    let mut processes = provider.list_processes()?;

    // Step 1: Filter processes by process-level criteria.
    processes.retain(|p| filter_config.matches_process(p));

    // Step 2: For each matching process, get open files and apply file-level filters.
    let has_file_filters = filter_config.inet.is_some()
        || filter_config.dir_tree.is_some()
        || filter_config.dir.is_some()
        || !filter_config.names.is_empty();

    for proc in &mut processes {
        // Populate open files from the platform provider.
        match provider.list_open_files(proc.pid) {
            Ok(files) => proc.open_files = files,
            Err(_) => {
                // Permission denied or process gone -- skip silently.
                continue;
            }
        }

        // Apply file-level filters if any are active.
        if has_file_filters {
            proc.open_files.retain(|f| filter_config.matches_file(f));
        }
    }

    // If file-level filters are active, remove processes with no matching files.
    if has_file_filters {
        processes.retain(|p| !p.open_files.is_empty());
    }

    // Step 3: Output.
    if formatter.terse {
        formatter.print_terse(&processes);
    } else if formatter.field_output.is_some() {
        for proc in &processes {
            formatter.print_field_output(proc);
        }
    } else {
        formatter.print_header();
        for proc in &processes {
            if proc.open_files.is_empty() {
                // Print at least one line for the process even without open files.
                let user_display = if formatter.list_uid {
                    proc.uid.to_string()
                } else {
                    proc.user.clone()
                };
                let cmd = if proc.comm.len() > formatter.cmd_width {
                    &proc.comm[..formatter.cmd_width]
                } else {
                    &proc.comm
                };
                if formatter.show_ppid {
                    println!(
                        "{:<width$} {:>5} {:>5} {:<8}",
                        cmd,
                        proc.pid,
                        proc.ppid.map(|p| p.to_string()).unwrap_or_default(),
                        user_display,
                        width = formatter.cmd_width,
                    );
                } else {
                    println!(
                        "{:<width$} {:>5} {:<8}",
                        cmd,
                        proc.pid,
                        user_display,
                        width = formatter.cmd_width,
                    );
                }
            } else {
                formatter.print_process_files(proc);
            }
        }
    }

    Ok(())
}

fn run_tui(provider: &dyn platform::PlatformProvider) -> std::io::Result<()> {
    // Load initial process list
    let processes = provider
        .list_processes()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let mut state = AppState::new(processes);
    let event_handler = EventHandler::new(Duration::from_millis(100));

    // Initialize terminal
    let mut terminal = ratatui::init();

    loop {
        // Draw the UI
        terminal.draw(|frame| {
            ui::render(frame, &mut state);
        })?;

        // Handle events
        match event_handler.next()? {
            AppEvent::Key(key) => {
                let search_empty = state.search_input.is_empty();
                if let Some(action) = map_key_to_action(key, &state.mode, search_empty) {
                    dispatch_action(&mut state, action, provider);
                }
            }
            AppEvent::Resize(_, _) => {
                // Terminal will redraw on next loop iteration
            }
            AppEvent::Tick => {
                // Nothing to do on tick for now
            }
        }

        if state.should_quit {
            break;
        }
    }

    // Restore terminal
    ratatui::restore();

    // Print export data if the user triggered Ctrl+E export
    if let Some(data) = &state.export_data {
        println!("{}", data);
    }

    Ok(())
}

fn dispatch_action(
    state: &mut AppState,
    action: Action,
    provider: &dyn platform::PlatformProvider,
) {
    match action {
        Action::Quit => {
            state.should_quit = true;
        }
        Action::SearchInput(c) => {
            state.search_input.push(c);
            state.update_filter();
        }
        Action::SearchBackspace => {
            state.search_input.pop();
            state.update_filter();
        }
        Action::SearchClear => {
            state.search_input.clear();
            state.update_filter();
        }
        Action::MoveUp => {
            state.move_up();
        }
        Action::MoveDown => {
            state.move_down();
        }
        Action::PageUp => {
            state.page_up();
        }
        Action::PageDown => {
            state.page_down();
        }
        Action::Select => {
            state.select_current();
            // Populate open files for the selected process
            if let Some(ref mut proc) = state.selected_process {
                if let Ok(files) = provider.list_open_files(proc.pid) {
                    proc.open_files = files;
                }
            }
        }
        Action::Back => {
            state.go_back();
        }
        Action::NextTab => {
            state.next_tab();
        }
        Action::PrevTab => {
            state.prev_tab();
        }
        Action::Refresh => {
            state.refresh(provider);
        }
        Action::YankSelected => {
            if let Some(ref proc) = state.selected_process {
                if let Some(line) = state.yank_selected_line(&proc.open_files) {
                    // Print to stderr so it doesn't interfere with TUI
                    eprintln!("\x1b[33m[Yanked]\x1b[0m {}", line);
                }
            }
        }
        Action::ExportProcess => {
            if let Some(ref proc) = state.selected_process {
                let data = state.export_process_data(proc, &proc.open_files);
                state.export_data = Some(data);
                state.should_quit = true;
            }
        }
    }
}
