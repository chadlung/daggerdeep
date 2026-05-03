use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

pub enum MenuResult {
    SinglePlayer,
    Host(TcpStream),
    Join(TcpStream),
    Quit,
}

pub fn run_menu(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<MenuResult, io::Error> {
    loop {
        terminal.draw(render_main_menu)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => return Ok(MenuResult::SinglePlayer),
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    if let Some(stream) = run_host_screen(terminal)? {
                        return Ok(MenuResult::Host(stream));
                    }
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    if let Some(stream) = run_join_screen(terminal)? {
                        return Ok(MenuResult::Join(stream));
                    }
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    return Ok(MenuResult::Quit);
                }
                _ => {}
            }
        }
    }
}

fn render_main_menu(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "⚔  Return to Dagger Deep  ⚔",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[S]  Single Player", Style::default().fg(Color::White))),
        Line::from(Span::styled("[H]  Host Game", Style::default().fg(Color::White))),
        Line::from(Span::styled("[J]  Join Game", Style::default().fg(Color::White))),
        Line::raw(""),
        Line::from(Span::styled("[Q]  Quit", Style::default().fg(Color::DarkGray))),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn run_host_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<TcpStream>, io::Error> {
    let listener = TcpListener::bind("0.0.0.0:4444")
        .map_err(|e| io::Error::new(e.kind(), format!("Could not bind port 4444: {}", e)))?;
    listener.set_nonblocking(true)?;

    let (tx, rx) = mpsc::channel::<TcpStream>();
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        loop {
            if cancel_rx.try_recv().is_ok() {
                break;
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = tx.send(stream);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }
    });

    let spinner = ['|', '/', '-', '\\'];
    let mut spin_idx: usize = 0;

    loop {
        let ch = spinner[spin_idx % 4];
        terminal.draw(|frame| render_waiting(frame, ch))?;
        spin_idx += 1;

        if event::poll(Duration::from_millis(120))?
            && let Event::Key(key) = event::read()?
                && key.code == KeyCode::Esc {
                    cancel_tx.send(()).ok();
                    return Ok(None);
                }

        if let Ok(stream) = rx.try_recv() {
            return Ok(Some(stream));
        }
    }
}

fn render_waiting(frame: &mut Frame, spinner: char) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Waiting for player 2 on port 4444...",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            spinner.to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray))),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn run_join_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<TcpStream>, io::Error> {
    let mut input = String::new();
    let mut error_msg = String::new();

    loop {
        let input_clone = input.clone();
        let err_clone = error_msg.clone();
        terminal.draw(move |frame| render_join(frame, &input_clone, &err_clone))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => {
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        error_msg = String::from("Enter an IP address.");
                        continue;
                    }
                    match trimmed.parse::<IpAddr>() {
                        Err(_) => {
                            error_msg = String::from("Invalid IP address.");
                        }
                        Ok(ip) => {
                            let sock_addr = SocketAddr::new(ip, 4444);
                            match TcpStream::connect_timeout(&sock_addr, Duration::from_secs(5)) {
                                Ok(stream) => return Ok(Some(stream)),
                                Err(e) => {
                                    error_msg = format!("Connection failed: {}", e);
                                }
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    error_msg.clear();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    error_msg.clear();
                }
                _ => {}
            }
        }
    }
}

fn render_join(frame: &mut Frame, input: &str, error: &str) {
    let area = frame.area();
    let mut lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Enter host IP address:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            format!("> {}_", input),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[Enter] Connect   [Esc] Back", Style::default().fg(Color::DarkGray))),
    ];
    if !error.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(error.to_string(), Style::default().fg(Color::Red))));
    }
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}
