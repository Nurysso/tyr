//  Copyright (C) 2026 Dawood Khan
//  SPDX-License-Identifier: GPL-3.0-or-later

// Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me

/*
  Shared terminal-UI framework for TYR.
  Targets ratatui 0.30 / crossterm 0.29.
*/

#![allow(dead_code)]

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear as RatatuiClear, Gauge, LineGauge, List, ListItem,
        Paragraph, Wrap,
    },
    Frame, Terminal,
};

use std::io::{self, Stdout};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::time::Duration;
pub type Term = Terminal<CrosstermBackend<Stdout>>;

pub const OK: Color = Color::Green;
pub const WARN: Color = Color::Yellow;
pub const ERR: Color = Color::Red;
pub const INFO: Color = Color::Cyan;
pub const MUTED: Color = Color::DarkGray;
pub const ML: Color = Color::Magenta;
pub const DIM: Color = Color::Indexed(61);

pub enum Flow {
    Continue,
    Quit,
}

pub trait App {
    fn tick(&mut self) {}
    fn on_key(&mut self, key: KeyCode) -> Flow;
    fn draw(&self, f: &mut Frame);
    fn needs_clear(&self) -> bool {
        false
    }
}

pub fn run(app: &mut impl App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let mut term = Terminal::new(CrosstermBackend::new(out))?;

    let outcome = catch_unwind(AssertUnwindSafe(|| event_loop(app, &mut term)));

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    outcome.unwrap_or_else(|p| std::panic::resume_unwind(p))
}

fn event_loop(app: &mut impl App, term: &mut Term) -> io::Result<()> {
    loop {
        app.tick();
        let clear = app.needs_clear();
        term.draw(|f| {
            let area = f.area();
            if clear {
                f.render_widget(RatatuiClear, area);
            }
            f.render_widget(
                Block::default().style(Style::default().bg(Color::Black)),
                area,
            );
            app.draw(f);
        })?;
        if let Some(flow) = poll_key(app)? {
            if matches!(flow, Flow::Quit) {
                return Ok(());
            }
        }
    }
}

fn poll_key(app: &mut impl App) -> io::Result<Option<Flow>> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(k) => Ok(Some(app.on_key(k.code))),
        _ => Ok(None),
    }
}

//  Background jobs

pub enum Progress<T> {
    Update(String),
    Done(T),
}

pub struct Job<T> {
    rx: mpsc::Receiver<Progress<T>>,
    pub message: String,
}

impl<T: Send + 'static> Job<T> {
    pub fn spawn(work: impl FnOnce(mpsc::Sender<Progress<T>>) -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let done_tx = tx.clone();
        std::thread::spawn(move || {
            let result = work(tx);
            let _ = done_tx.send(Progress::Done(result));
        });
        Self {
            rx,
            message: String::new(),
        }
    }

    pub fn poll(&mut self) -> Option<T> {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Progress::Update(t) => self.message = t,
                Progress::Done(v) => return Some(v),
            }
        }
        None
    }
}

pub fn vsplit(area: Rect, constraints: &[Constraint]) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.to_vec())
        .split(area)
}

pub fn hsplit(area: Rect, constraints: &[Constraint]) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints.to_vec())
        .split(area)
}

/// Standard three-column layout used by every screen.
pub fn three_col(area: Rect, left_pct: u16, right_split_pct: u16) -> [Rect; 3] {
    let cols = hsplit(
        area,
        &[Constraint::Percentage(left_pct), Constraint::Min(0)],
    );

    let top_pct = right_split_pct.clamp(10, 90);
    let right_rows = vsplit(
        cols[1],
        &[
            Constraint::Percentage(top_pct),
            Constraint::Percentage(100 - top_pct),
        ],
    );

    [cols[0], right_rows[0], right_rows[1]]
}


pub fn panel(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Indexed(236))) // very dark gray border
        .title(Span::styled(
            format!("╴{title}"),
            Style::default().fg(MUTED),
        ))
}

/// Title bar: wordmark T / Y / R in three colors + a subtitle.
pub fn wordmark_bar(subtitle: &str) -> Paragraph<'static> {
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled("T", Style::default().fg(INFO).add_modifier(Modifier::BOLD)),
        Span::styled("Y", Style::default().fg(ML).add_modifier(Modifier::BOLD)),
        Span::styled("R", Style::default().fg(OK).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(subtitle.to_string(), Style::default().fg(MUTED)),
    ]);
    Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    )
}

/// Header row at the top of a screen: title on the left, status chip on the right.
pub fn header_row(f: &mut Frame, area: Rect, subtitle: &str, status: &str, status_color: Color) {
    let cols = hsplit(area, &[Constraint::Min(0), Constraint::Length(24)]);
    f.render_widget(wordmark_bar(subtitle), cols[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::raw(" "), chip(status, status_color)])).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        cols[1],
    );
}

pub fn controls_bar(bindings: &[(&str, &str)]) -> Paragraph<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, action)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  │  ",
                Style::default().fg(Color::Indexed(237)),
            ));
        }
        spans.push(Span::styled(
            format!("[{key}]"),
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(action.to_string(), Style::default().fg(MUTED)));
    }
    Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Indexed(236))),
    )
}

pub fn paragraph(lines: Vec<Line<'static>>, title: &str) -> Paragraph<'static> {
    Paragraph::new(lines)
        .block(panel(title))
        .wrap(Wrap { trim: false })
}

pub fn section_label(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_uppercase(),
        Style::default()
            .fg(Color::Indexed(240))
            .add_modifier(Modifier::BOLD),
    ))
}

pub fn header(text: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

pub fn stat(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label}: "), Style::default().fg(MUTED)),
        Span::styled(
            value.into(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

pub fn stat_inline(label: &str, value: impl Into<String>, color: Color) -> Span<'static> {
    // For building stat rows with multiple values on one Line.
    Span::styled(
        format!(" {label}: {}", value.into()),
        Style::default().fg(color),
    )
}

pub fn bullet(text: impl Into<String>) -> Line<'static> {
    Line::from(format!("  • {}", text.into()))
}

pub fn blank() -> Line<'static> {
    Line::from("")
}

pub fn more(count: usize, noun: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("▾ {count} more {noun}"),
        Style::default().fg(MUTED),
    ))
}

pub fn path_line(path: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(path.to_string(), Style::default().fg(INFO)),
    ])
}

pub fn chip(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

/// Subtle tinted chip: colored text on a near-black background.
/// Use when a full inverse-video chip would be too heavy (e.g. status tags).
pub fn chip_subtle(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

pub fn chip_row(chips: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = Vec::with_capacity(chips.len() * 2);
    for (i, c) in chips.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(c);
    }
    Line::from(spans)
}

pub fn spinner(tick: usize) -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    FRAMES[tick % FRAMES.len()]
}


pub fn busy_gauge(f: &mut Frame, area: Rect, title: &str, label: &str, color: Color, tick: usize) {
    let block = panel(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let phase = tick % 40;
    let percent = if phase < 20 {
        phase * 5
    } else {
        (40 - phase) * 5
    };
    let ratio = (percent as f64 / 100.0).clamp(0.01, 0.99);

    let rows = vsplit(inner, &[Constraint::Length(1), Constraint::Min(0)]);
    let gauge = LineGauge::default()
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::Indexed(236)))
        .label(format!(" {} {label}", spinner(tick)))
        .ratio(ratio);
    f.render_widget(gauge, rows[0]);
}

/// Per-category determinate LineGauge (slim, one row).
/// Use this inside a loop to draw one row per file category.
pub fn category_gauge(f: &mut Frame, area: Rect, label: &str, ratio: f64, color: Color) {
    let rows = hsplit(
        area,
        &[
            Constraint::Length(12),
            Constraint::Min(0),
            Constraint::Length(5),
        ],
    );
    f.render_widget(
        Paragraph::new(Span::styled(label.to_string(), Style::default().fg(MUTED))),
        rows[0],
    );
    let gauge = LineGauge::default()
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::Indexed(236)))
        .ratio(ratio.clamp(0.0, 1.0));
    f.render_widget(gauge, rows[1]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{:3.0}%", ratio * 100.0),
            Style::default().fg(DIM),
        )),
        rows[2],
    );
}

pub fn progress_gauge(
    f: &mut Frame,
    area: Rect,
    title: &str,
    label: &str,
    ratio: f64,
    color: Color,
) {
    let gauge = Gauge::default()
        .block(panel(title))
        .gauge_style(Style::default().fg(color).bg(Color::Indexed(234)))
        .ratio(ratio.clamp(0.0, 1.0))
        .label(label.to_string());
    f.render_widget(gauge, area);
}


pub fn bar_chart(f: &mut Frame, area: Rect, title: &str, entries: &[(String, usize, Color)]) {
    let block = panel(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if entries.is_empty() || inner.height == 0 || inner.width == 0 {
        return;
    }

    let max = entries.iter().map(|(_, n, _)| *n).max().unwrap_or(1).max(1);
    let label_w = entries
        .iter()
        .map(|(n, _, _)| n.chars().count())
        .max()
        .unwrap_or(0)
        .min(14);
    let count_w = entries
        .iter()
        .map(|(_, n, _)| n.to_string().len())
        .max()
        .unwrap_or(1);
    let gutter = label_w + count_w + 4;
    let bar_width = (inner.width as usize).saturating_sub(gutter).max(1);
    let visible = inner.height as usize;
    let (shown, overflow) = if entries.len() > visible && visible > 0 {
        (visible.saturating_sub(1), entries.len() - (visible - 1))
    } else {
        (entries.len(), 0)
    };

    let mut constraints: Vec<Constraint> = vec![Constraint::Length(1); shown];
    if overflow > 0 {
        constraints.push(Constraint::Length(1));
    }
    let rows = vsplit(inner, &constraints);

    for (row, (name, count, color)) in rows.iter().zip(entries.iter().take(shown)) {
        let filled = ((*count as f64 / max as f64) * bar_width as f64).round() as usize;
        let filled = filled.clamp(if *count > 0 { 1 } else { 0 }, bar_width);
        let bar = "█".repeat(filled);
        let line = Line::from(vec![
            Span::styled(format!("{name:<label_w$} "), Style::default().fg(MUTED)),
            Span::styled(bar, Style::default().fg(*color)),
            Span::raw(format!(" {count:>count_w$}")),
        ]);
        f.render_widget(Paragraph::new(line), *row);
    }
    if overflow > 0 {
        if let Some(last) = rows.last() {
            f.render_widget(Paragraph::new(more(overflow, "categories")), *last);
        }
    }
}

#[derive(Clone, Copy)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Err,
}

impl Level {
    fn icon(self) -> &'static str {
        match self {
            Self::Info => "·",
            Self::Ok => "✓",
            Self::Warn => "⚠",
            Self::Err => "✗",
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Info => MUTED,
            Self::Ok => OK,
            Self::Warn => WARN,
            Self::Err => ERR,
        }
    }
}

pub fn log_list<'a>(title: &str, entries: impl Iterator<Item = (Level, &'a str)>) -> List<'static> {
    let items: Vec<ListItem> = entries
        .map(|(lvl, msg)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", lvl.icon()), Style::default().fg(lvl.color())),
                Span::styled(
                    msg.to_string(),
                    Style::default().fg(match lvl {
                        Level::Info => Color::Indexed(245),
                        _ => lvl.color(),
                    }),
                ),
            ]))
        })
        .collect();
    List::new(items).block(panel(title))
}

pub fn file_row(
    name: &str,
    size: &str,
    dest: &str,
    selected: bool,
    dest_color: Color,
) -> Line<'static> {
    let icon = if selected { "▸" } else { " " };
    let base = if selected {
        Style::default().bg(Color::Indexed(234))
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(format!("{icon} "), base.fg(INFO)),
        Span::styled(format!("{name:<30}"), base.fg(Color::White)),
        Span::styled(format!("{size:>7}  "), base.fg(MUTED)),
        Span::styled(dest.to_string(), base.fg(dest_color)),
    ])
}

pub fn title_bar(text: &str, color: Color) -> Paragraph<'static> {
    Paragraph::new(format!(" {text}"))
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL))
}

pub fn stat_row(pairs: &[(&str, &str, Color)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (label, value, color)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default()));
        }
        spans.push(Span::styled(
            format!("{label}: "),
            Style::default().fg(MUTED),
        ));
        spans.push(Span::styled(
            value.to_string(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}


pub fn screen_frame(
    f: &mut Frame,
    header_area: Rect,
    body_area: Rect,
    footer_area: Rect,
    subtitle: &str,
    status: &str,
    status_color: Color,
    bindings: &[(&str, &str)],
    left_pct: u16,
    right_pct: u16,
) -> [Rect; 3] {
    header_row(f, header_area, subtitle, status, status_color);
    f.render_widget(controls_bar(bindings), footer_area);
    three_col(body_area, left_pct, right_pct)
}

/// Renders a 2×2 grid of large-number summary cells inside `area`.
/// `cells` = [(value_str, label, color); up to 4]
pub fn summary_grid(f: &mut Frame, area: Rect, cells: &[(&str, &str, Color)]) {
    let rows = vsplit(area, &[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
    for (row_idx, row) in rows.iter().enumerate() {
        let pair = &cells[(row_idx * 2)..((row_idx * 2 + 2).min(cells.len()))];
        let cols = hsplit(*row, &[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        for (col_idx, (val, lbl, color)) in pair.iter().enumerate() {
            let lines = vec![
                Line::from(Span::styled(
                    val.to_string(),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(lbl.to_string(), Style::default().fg(MUTED))),
            ];
            f.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Indexed(236))),
                ),
                cols[col_idx],
            );
        }
    }
}
