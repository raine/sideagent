use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Wrap},
};
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use super::runs::RunState;
use super::{MonitorCore, RunSummary};

pub(crate) fn run(args: crate::MonitorArgs) -> Result<()> {
    let runs_root = match args.runs_root {
        Some(path) => path,
        None => MonitorCore::default_root()?,
    };
    let poll_interval = Duration::from_millis(args.poll_interval_ms.max(50));
    let mut app = MonitorApp::new(MonitorCore::new(runs_root), poll_interval);
    app.poll()?;

    if args.once {
        print_snapshot(&app);
        return Ok(());
    }

    run_tui(app)
}

struct MonitorApp {
    core: MonitorCore,
    poll_interval: Duration,
    runs: Vec<RunSummary>,
    selected: usize,
    selected_stdout_file: Option<PathBuf>,
    transcript: VecDeque<String>,
    pending_raw: Option<String>,
}

impl MonitorApp {
    const MAX_TRANSCRIPT_LINES: usize = 1_000;

    fn new(core: MonitorCore, poll_interval: Duration) -> Self {
        Self {
            core,
            poll_interval,
            runs: Vec::new(),
            selected: 0,
            selected_stdout_file: None,
            transcript: VecDeque::new(),
            pending_raw: None,
        }
    }

    fn poll(&mut self) -> Result<()> {
        let previous = self.selected_stdout_file.clone();
        self.runs = self.core.poll_runs()?;
        if self.runs.is_empty() {
            self.selected = 0;
            self.selected_stdout_file = None;
            self.transcript.clear();
            self.pending_raw = None;
            return Ok(());
        }
        self.selected = self.selected.min(self.runs.len() - 1);
        let stdout_file = self.runs[self.selected].stdout_file.clone();
        if previous.as_ref() != Some(&stdout_file) {
            self.transcript.clear();
            self.pending_raw = None;
            self.selected_stdout_file = Some(stdout_file);
        }
        let update = self.core.poll_stdout(&self.runs[self.selected])?;
        for line in update.lines {
            if self.transcript.len() == Self::MAX_TRANSCRIPT_LINES {
                self.transcript.pop_front();
            }
            self.transcript.push_back(line);
        }
        self.pending_raw = update.pending_raw;
        Ok(())
    }

    fn select_next(&mut self) {
        if !self.runs.is_empty() {
            self.selected = (self.selected + 1).min(self.runs.len() - 1);
        }
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn select_first(&mut self) {
        self.selected = 0;
    }

    fn select_last(&mut self) {
        if !self.runs.is_empty() {
            self.selected = self.runs.len() - 1;
        }
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let entered = (|| -> Result<Self> {
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            terminal.clear()?;
            Ok(Self { terminal })
        })();

        if entered.is_err() {
            let _ = terminal::disable_raw_mode();
        }

        entered
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn run_tui(mut app: MonitorApp) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    loop {
        guard.terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(app.poll_interval)?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                KeyCode::Home | KeyCode::Char('g') => app.select_first(),
                KeyCode::End | KeyCode::Char('G') => app.select_last(),
                _ => {}
            }
        }

        app.poll()?;
    }
    Ok(())
}

fn state_label(state: RunState) -> &'static str {
    match state {
        RunState::Active => "running",
        RunState::Success => "success",
        RunState::Failed => "failed",
        RunState::Unknown => "unknown",
    }
}

fn detail_text(app: &MonitorApp, max_transcript_lines: usize) -> Vec<String> {
    if app.runs.is_empty() {
        return vec!["No headless run archives found.".to_string()];
    }

    let run = &app.runs[app.selected];
    let mut lines = vec![
        format!("id: {}", run.id),
        format!("state: {}", state_label(run.state)),
        format!(
            "profile: {}",
            run.profile_name.as_deref().unwrap_or("unknown")
        ),
        format!(
            "command: {}",
            run.profile_command.as_deref().unwrap_or("unknown")
        ),
        format!(
            "interface: {}",
            run.interface.as_deref().unwrap_or("unknown")
        ),
        format!(
            "started: {}",
            run.started_at.as_deref().unwrap_or("unknown")
        ),
    ];

    if let Some(completed_at) = run.completed_at.as_deref() {
        lines.push(format!("completed: {completed_at}"));
    }
    if let Some(exit_code) = run.exit_code {
        lines.push(format!("exit code: {exit_code}"));
    }
    if let Some(failure) = run.failure.as_deref() {
        lines.push(format!("failure: {failure}"));
    }
    if let Some(error) = run.metadata_error.as_deref() {
        lines.push(format!("metadata error: {error}"));
    }

    lines.push(String::new());
    lines.push("transcript:".to_string());

    let transcript_lines: Vec<&String> = app
        .transcript
        .iter()
        .rev()
        .take(max_transcript_lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if transcript_lines.is_empty() {
        lines.push("  (no output yet)".to_string());
    } else {
        for line in transcript_lines {
            lines.push(line.clone());
        }
    }

    if let Some(pending) = app.pending_raw.as_deref() {
        lines.push(format!("  (partial) {pending}"));
    }

    lines
}

fn detail_lines(app: &MonitorApp, max_transcript_lines: usize) -> Vec<Line<'static>> {
    detail_text(app, max_transcript_lines)
        .into_iter()
        .map(|line| Line::from(Span::raw(line)))
        .collect()
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &MonitorApp) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(root[0]);

    let runs = app.runs.iter().enumerate().map(|(index, run)| {
        let selected = index == app.selected;
        let state = state_label(run.state);
        let title = run.profile_name.as_deref().unwrap_or(&run.id);
        let started = run.started_at.as_deref().unwrap_or("unknown time");
        let prefix = if selected { "> " } else { "  " };
        ListItem::new(vec![
            Line::from(format!("{prefix}{state} {title}")),
            Line::from(format!("  {started}")),
        ])
    });

    let list = List::new(runs.collect::<Vec<_>>()).block(Block::bordered().title("Headless runs"));
    frame.render_widget(list, chunks[0]);

    let detail_height = chunks[1].height.saturating_sub(2) as usize;
    let detail = Paragraph::new(detail_lines(app, detail_height))
        .block(Block::bordered().title("Run detail"))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::raw("j/k or arrows: navigate  "),
        Span::raw("g/G or Home/End: first/last  "),
        Span::raw("q or Esc: quit"),
    ]))
    .style(Style::default());
    frame.render_widget(footer, root[1]);
}

fn print_snapshot(app: &MonitorApp) {
    println!("Headless runs");
    if app.runs.is_empty() {
        println!("No headless run archives found.");
        return;
    }

    for (index, run) in app.runs.iter().enumerate() {
        let marker = if index == app.selected { ">" } else { " " };
        println!(
            "{marker} {} {}",
            state_label(run.state),
            run.profile_name.as_deref().unwrap_or(&run.id)
        );
    }

    println!();
    println!("Run detail");
    for line in detail_text(app, usize::MAX) {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn app_poll_loads_run_and_transcript() {
        let dir = tempfile::TempDir::new().unwrap();
        let run_path = dir.path().join("run-1");
        fs::create_dir_all(&run_path).unwrap();
        fs::write(
            run_path.join("metadata.json"),
            r#"{"profile":{"name":"demo","command":"agent","args":[]},"interface":"claude","started_at":"2026-06-09T00:00:00Z","status":"running"}"#,
        )
        .unwrap();
        fs::write(
            run_path.join("stdout.jsonl"),
            b"{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\n",
        )
        .unwrap();

        let mut app = MonitorApp::new(
            MonitorCore::new(dir.path().to_path_buf()),
            Duration::from_millis(50),
        );
        app.poll().unwrap();

        assert_eq!(app.runs.len(), 1);
        assert_eq!(app.transcript, vec!["[text]  hello"]);
    }
}
