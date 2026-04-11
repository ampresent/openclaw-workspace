use anyhow::Result;
use clap::Args;
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde::Serialize;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Args)]
pub struct StatusArgs {
    /// Show only packages with pending patches
    #[arg(long)]
    pending: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Interactive live TUI dashboard
    #[arg(long)]
    live: bool,
}

#[derive(Serialize, Clone)]
pub struct StatusReport {
    pub root: String,
    pub frozen: bool,
    pub packages: Vec<PkgStatus>,
    pub last_refresh: String,
}

#[derive(Serialize, Clone)]
pub struct PkgStatus {
    pub name: String,
    pub patches: usize,
    pub has_changes: bool,
    pub files: usize,
    pub patch_names: Vec<String>,
}

pub fn run(args: StatusArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    if args.live {
        return run_live_tui(&root);
    }

    // Static output (original behavior)
    let report = collect_status(&root, args.pending)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    render_static(&report);
    Ok(())
}

/// Collect current system status into a report
fn collect_status(root: &Path, pending_only: bool) -> Result<StatusReport> {
    let packages = super::util::discover_packages(root)?;
    let frozen = root.join(".evo").join("frozen").exists();

    let mut statuses = Vec::new();
    for pkg in &packages {
        let src = root.join("src").join(pkg);
        let patches_dir = root.join("patches").join(pkg);
        let patches = super::util::list_patches(&patches_dir)?;
        let has_changes = super::util::has_changes(&src);
        let files = super::util::count_files(&src);

        if pending_only && !has_changes && patches.is_empty() {
            continue;
        }

        let patch_names: Vec<String> = patches
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        statuses.push(PkgStatus {
            name: pkg.clone(),
            patches: patches.len(),
            has_changes,
            files,
            patch_names,
        });
    }

    Ok(StatusReport {
        root: root.display().to_string(),
        frozen,
        packages: statuses,
        last_refresh: chrono::Local::now().format("%H:%M:%S").to_string(),
    })
}

/// Static (non-interactive) status display
fn render_static(report: &StatusReport) {
    println!(
        "╔══════════════════════════════════════════════════════════╗"
    );
    println!(
        "║              Evolution OS — evo status                  ║"
    );
    println!(
        "╠══════════════════════════════════════════════════════════╣"
    );
    println!(
        "║  Source root: {:<43} ║",
        truncate(&report.root, 43)
    );
    println!(
        "║  Packages:    {:<43} ║",
        report.packages.len()
    );
    println!(
        "║  Frozen:      {:<43} ║",
        if report.frozen {
            "yes ⛔".to_string()
        } else {
            "no".to_string()
        }
    );
    println!(
        "╠══════════════════════════════════════════════════════════╣"
    );

    if report.packages.is_empty() {
        println!(
            "║  No packages initialized.                               ║"
        );
        println!(
            "║  Run: evo init <package>                                ║"
        );
    } else {
        println!(
            "║  {:<20} {:>8} {:>8} {:>8}  {} ║",
            "Package", "Patches", "Files", "Dirty", ""
        );
        println!(
            "║  {:<20} {:>8} {:>8} {:>8}  {} ║",
            "────────────────────", "────────", "────────", "────────", ""
        );
        for s in &report.packages {
            let dirty = if s.has_changes {
                "yes".yellow().to_string()
            } else {
                "no".dimmed().to_string()
            };
            println!(
                "║  {:<20} {:>8} {:>8} {:>14}    ║",
                truncate(&s.name, 20),
                s.patches,
                s.files,
                dirty
            );
        }
    }

    println!(
        "╚══════════════════════════════════════════════════════════╝"
    );
}

// ── Interactive TUI ─────────────────────────────────────────

struct App {
    report: StatusReport,
    root: std::path::PathBuf,
    selected: usize,
    last_refresh: Instant,
    auto_refresh_secs: u64,
    should_quit: bool,
    show_patches: bool,
}

impl App {
    fn new(root: std::path::PathBuf) -> Result<Self> {
        let report = collect_status(&root, false)?;
        Ok(Self {
            report,
            root,
            selected: 0,
            last_refresh: Instant::now(),
            auto_refresh_secs: 5,
            should_quit: false,
            show_patches: false,
        })
    }

    fn refresh(&mut self) -> Result<()> {
        self.report = collect_status(&self.root, false)?;
        self.last_refresh = Instant::now();
        // Clamp selection
        if self.selected >= self.report.packages.len() && !self.report.packages.is_empty() {
            self.selected = self.report.packages.len() - 1;
        }
        Ok(())
    }

    fn next(&mut self) {
        if !self.report.packages.is_empty() {
            self.selected = (self.selected + 1) % self.report.packages.len();
        }
    }

    fn prev(&mut self) {
        if !self.report.packages.is_empty() {
            if self.selected == 0 {
                self.selected = self.report.packages.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }
}

fn run_live_tui(root: &Path) -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new(root.to_path_buf())?;
    let result = run_tui_loop(&mut terminal, &mut app);

    // Restore terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| draw_tui(frame, app))?;

        // Auto-refresh
        if app.last_refresh.elapsed() >= Duration::from_secs(app.auto_refresh_secs) {
            let _ = app.refresh();
        }

        // Poll for events with timeout
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('r') => {
                        let _ = app.refresh();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.prev();
                    }
                    KeyCode::Enter => {
                        app.show_patches = !app.show_patches;
                    }
                    KeyCode::Char('f') => {
                        // Toggle freeze
                        let evo_dir = app.root.join(".evo");
                        let lock_path = evo_dir.join("frozen");
                        if lock_path.exists() {
                            std::fs::remove_file(&lock_path).ok();
                        } else {
                            std::fs::create_dir_all(&evo_dir).ok();
                            let ts = chrono::Local::now().to_rfc3339();
                            std::fs::write(&lock_path, &ts).ok();
                        }
                        let _ = app.refresh();
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn draw_tui(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Main layout: header + body + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),          // header
            Constraint::Min(8),             // package table
            Constraint::Length(if app.show_patches { 8 } else { 0 }), // patch detail
            Constraint::Length(2),          // footer
        ])
        .split(size);

    // ── Header ──
    let frozen_str = if app.report.frozen { "⛔ FROZEN" } else { "🟢 LIVE" };
    let header_text = vec![
        Line::from(vec![
            Span::styled("  Evolution OS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  —  "),
            Span::styled(frozen_str, Style::default().fg(if app.report.frozen { Color::Red } else { Color::Green })),
        ]),
        Line::from(vec![
            Span::raw("  root: "),
            Span::styled(&app.report.root, Style::default().fg(Color::DarkGray)),
            Span::raw(format!("  │  packages: {}  │  refresh: {}",
                app.report.packages.len(), app.report.last_refresh)),
        ]),
    ];
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(header, chunks[0]);

    // ── Package Table ──
    let header_row = Row::new(vec!["Package", "Patches", "Files", "Dirty"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.report.packages.iter().enumerate().map(|(i, pkg)| {
        let dirty = if pkg.has_changes { "yes" } else { "—" };
        let style = if i == app.selected {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else if pkg.has_changes {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        Row::new(vec![
            pkg.name.clone(),
            pkg.patches.to_string(),
            pkg.files.to_string(),
            dirty.to_string(),
        ]).style(style)
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(" Packages ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, chunks[1]);

    // ── Patch Detail (if toggled) ──
    if app.show_patches && !app.report.packages.is_empty() {
        let pkg = &app.report.packages[app.selected.min(app.report.packages.len() - 1)];
        let patch_lines: Vec<Line> = if pkg.patch_names.is_empty() {
            vec![Line::from("  No patches in stack.").style(Style::default().fg(Color::DarkGray))]
        } else {
            pkg.patch_names.iter().enumerate().map(|(i, name)| {
                Line::from(format!("  {}. {}", i + 1, name))
                    .style(Style::default().fg(Color::Cyan))
            }).collect()
        };

        let patch_detail = Paragraph::new(patch_lines)
            .block(
                Block::default()
                    .title(format!(" Patches: {} ", pkg.name))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(patch_detail, chunks[2]);
    }

    // ── Footer ──
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" quit  "),
        Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" refresh  "),
        Span::styled("↑↓/jk", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" patches  "),
        Span::styled("f", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" freeze/unfreeze"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
