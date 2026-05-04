use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::dungeon::{MAP_HEIGHT, MAP_WIDTH, Tile};
use crate::entity::MonsterKind;
use crate::game::{Game, GameState};
use crate::protocol::NetGameState;

pub fn render_game(frame: &mut Frame, game: &Game) {
    // Fixed game area: dungeon + sidebar wide, top bar + dungeon + status bar tall.
    // Does not stretch to fill the terminal if it is larger than needed.
    let game_width = (MAP_WIDTH + 19) as u16;
    let game_height = (MAP_HEIGHT + 2) as u16;
    let area = Rect {
        x: 0,
        y: 0,
        width: game_width,
        height: game_height,
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(MAP_HEIGHT as u16),
            Constraint::Length(1),
        ])
        .split(area);

    render_top_bar(frame, game, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(MAP_WIDTH as u16), Constraint::Length(19)])
        .split(rows[1]);

    render_dungeon(frame, game, cols[0]);
    render_sidebar(frame, game, cols[1]);
    render_status_bar(frame, game, rows[2]);

    if game.state == GameState::GameOver {
        render_overlay(frame, area, "GAME OVER", "You have perished in Dagger Deep.", Color::Red, true);
    } else if game.state == GameState::Victory {
        render_overlay(frame, area, "VICTORY!", "You have conquered Dagger Deep!", Color::Yellow, true);
    } else if game.state == GameState::ConfirmQuit {
        render_confirm_quit(frame, area);
    } else if game.state == GameState::Disconnected {
        render_overlay(frame, area, "DISCONNECTED", "Connection lost.", Color::Gray, false);
    }
}

pub fn render_net_state(frame: &mut Frame, state: &NetGameState) {
    let game_width = (MAP_WIDTH + 19) as u16;
    let game_height = (MAP_HEIGHT + 2) as u16;
    let area = Rect { x: 0, y: 0, width: game_width, height: game_height };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(MAP_HEIGHT as u16),
            Constraint::Length(1),
        ])
        .split(area);

    // Top bar
    let title = format!("⚔  Return to Dagger Deep  ⚔        Level: {}/10", state.level);
    let para = Paragraph::new(title)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    frame.render_widget(para, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(MAP_WIDTH as u16), Constraint::Length(19)])
        .split(rows[1]);

    // Dungeon
    let mut lines: Vec<Line> = Vec::new();
    for y in 0..MAP_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..MAP_WIDTH {
            if state.player1.x == x && state.player1.y == y {
                spans.push(Span::styled("@", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                continue;
            }
            if let Some(ref p2) = state.player2
                && p2.x == x && p2.y == y {
                    spans.push(Span::styled("&", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                    continue;
                }
            if let Some(m) = state.monsters.iter().find(|m| m.alive && m.x == x && m.y == y) {
                let kind = MonsterKind::from(m.kind);
                let color = match kind {
                    MonsterKind::Goblin => Color::Blue,
                    MonsterKind::Mage => Color::Red,
                    MonsterKind::Rat => Color::LightCyan,
                    MonsterKind::Lich => Color::Magenta,
                };
                spans.push(Span::styled(
                    kind.char().to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            let tile = Tile::from(state.map_tiles.get(y).and_then(|row| row.get(x)).copied().unwrap_or(0));
            let (ch, style) = match tile {
                Tile::Wall => ('#', Style::default().fg(Color::DarkGray).bg(Color::Black)),
                Tile::Floor | Tile::Corridor => ('.', Style::default().fg(Color::DarkGray).bg(Color::Black)),
                Tile::Stair => ('>', Style::default().fg(Color::Yellow).bg(Color::Black)),
                Tile::Potion => ('P', Style::default().fg(Color::Green).bg(Color::Black)),
                Tile::Chest => ('$', Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).bg(Color::Black)),
                Tile::LichChest => ('$', Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD).bg(Color::Black)),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, cols[0]);

    // Sidebar
    let hp_color = |hp: i32, max_hp: i32| {
        if hp <= max_hp / 2 { Color::Red } else { Color::Green }
    };
    let mut sidebar_lines = vec![
        Line::from(Span::styled("PLAYER 1", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", state.player1.hp.max(0), state.player1.max_hp),
                Style::default().fg(hp_color(state.player1.hp, state.player1.max_hp)),
            ),
        ]),
        Line::raw(format!("Respawns: {}", state.player1.respawns_left)),
    ];
    if let Some(ref p2) = state.player2 {
        sidebar_lines.push(Line::raw(""));
        sidebar_lines.push(Line::from(Span::styled("PLAYER 2", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
        sidebar_lines.push(Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", p2.hp.max(0), p2.max_hp),
                Style::default().fg(hp_color(p2.hp, p2.max_hp)),
            ),
        ]));
        sidebar_lines.push(Line::raw(format!("Respawns: {}", p2.respawns_left)));
    }
    sidebar_lines.extend([
        Line::raw(""),
        Line::from(Span::styled("LEGEND", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("@ ", Style::default().fg(Color::White)), Span::raw("P1")]),
        Line::from(vec![Span::styled("& ", Style::default().fg(Color::Cyan)), Span::raw("P2")]),
        Line::from(vec![Span::styled("G ", Style::default().fg(Color::Blue)), Span::raw("Goblin")]),
        Line::from(vec![Span::styled("M ", Style::default().fg(Color::Red)), Span::raw("Mage")]),
        Line::from(vec![Span::styled("R ", Style::default().fg(Color::LightCyan)), Span::raw("Rat")]),
        Line::from(vec![Span::styled("L ", Style::default().fg(Color::Magenta)), Span::raw("Lich")]),
        Line::from(vec![Span::styled("P ", Style::default().fg(Color::Green)), Span::raw("Potion")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Yellow)), Span::raw("Chest")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Magenta)), Span::raw("Lich drop")]),
        Line::from(vec![Span::styled("> ", Style::default().fg(Color::Yellow)), Span::raw("Stairs")]),
    ]);
    let para = Paragraph::new(sidebar_lines)
        .block(Block::default().borders(Borders::LEFT).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, cols[1]);

    // Status bar (P2 sees their own messages)
    let para = Paragraph::new(state.status_msg2.clone())
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD).bg(Color::Black));
    frame.render_widget(para, rows[2]);

    // Overlays
    match state.state {
        1 => render_overlay(frame, area, "GAME OVER", "You have perished in Dagger Deep.", Color::Red, true),
        2 => render_overlay(frame, area, "VICTORY!", "You have conquered Dagger Deep!", Color::Yellow, true),
        4 => render_overlay(frame, area, "DISCONNECTED", "Connection lost.", Color::Gray, false),
        _ => {}
    }
}

pub fn render_connecting(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Connecting to host...",
            Style::default().fg(Color::Cyan),
        )),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

pub fn render_disconnected(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Connection lost.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Press any key to exit.",
            Style::default().fg(Color::Gray),
        )),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn render_top_bar(frame: &mut Frame, game: &Game, area: Rect) {
    let title = format!(
        "⚔  Return to Dagger Deep  ⚔        Level: {}/10",
        game.level
    );
    let para = Paragraph::new(title).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(para, area);
}

fn render_dungeon(frame: &mut Frame, game: &Game, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for y in 0..MAP_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..MAP_WIDTH {
            // Render priority: player > player2 > monster > tile
            if game.player.x == x && game.player.y == y {
                spans.push(Span::styled(
                    "@",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            if let Some(ref p2) = game.player2
                && p2.x == x && p2.y == y {
                    spans.push(Span::styled(
                        "&",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ));
                    continue;
                }

            if let Some(m) = game
                .monsters
                .iter()
                .find(|m| m.is_alive() && m.x == x && m.y == y)
            {
                let color = match m.kind {
                    MonsterKind::Goblin => Color::Blue,
                    MonsterKind::Mage => Color::Red,
                    MonsterKind::Rat => Color::LightCyan,
                    MonsterKind::Lich => Color::Magenta,
                };
                spans.push(Span::styled(
                    m.kind.char().to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            let (ch, style) = match game.map.get(x, y) {
                Tile::Wall => ('#', Style::default().fg(Color::DarkGray).bg(Color::Black)),
                Tile::Floor | Tile::Corridor => {
                    ('.', Style::default().fg(Color::DarkGray).bg(Color::Black))
                }
                Tile::Stair => ('>', Style::default().fg(Color::Yellow).bg(Color::Black)),
                Tile::Potion => ('P', Style::default().fg(Color::Green).bg(Color::Black)),
                Tile::Chest => ('$', Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).bg(Color::Black)),
                Tile::LichChest => ('$', Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD).bg(Color::Black)),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn render_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let hp_color = |hp: i32, max_hp: i32| {
        if hp <= max_hp / 2 { Color::Red } else { Color::Green }
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "PLAYER 1",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", game.player.hp.max(0), game.player.max_hp),
                Style::default().fg(hp_color(game.player.hp, game.player.max_hp)),
            ),
        ]),
    ];
    if game.multiplayer {
        lines.push(Line::raw(format!("Respawns: {}", game.player.respawns_left)));
    }

    if let Some(ref p2) = game.player2 {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "PLAYER 2",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", p2.hp.max(0), p2.max_hp),
                Style::default().fg(hp_color(p2.hp, p2.max_hp)),
            ),
        ]));
        lines.push(Line::raw(format!("Respawns: {}", p2.respawns_left)));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "LEGEND",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));
    lines.extend([
        Line::from(vec![Span::styled("@ ", Style::default().fg(Color::White)), Span::raw("P1")]),
        Line::from(vec![Span::styled("& ", Style::default().fg(Color::Cyan)), Span::raw("P2")]),
        Line::from(vec![Span::styled("G ", Style::default().fg(Color::Blue)), Span::raw("Goblin")]),
        Line::from(vec![Span::styled("M ", Style::default().fg(Color::Red)), Span::raw("Mage")]),
        Line::from(vec![Span::styled("R ", Style::default().fg(Color::LightCyan)), Span::raw("Rat")]),
        Line::from(vec![Span::styled("L ", Style::default().fg(Color::Magenta)), Span::raw("Lich")]),
        Line::from(vec![Span::styled("P ", Style::default().fg(Color::Green)), Span::raw("Potion")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Yellow)), Span::raw("Chest")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Magenta)), Span::raw("Lich drop")]),
        Line::from(vec![Span::styled("> ", Style::default().fg(Color::Yellow)), Span::raw("Stairs")]),
    ]);

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn render_status_bar(frame: &mut Frame, game: &Game, area: Rect) {
    let para = Paragraph::new(game.status_msg.clone()).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
            .bg(Color::Black),
    );
    frame.render_widget(para, area);
}

fn render_overlay(frame: &mut Frame, area: Rect, title: &str, subtitle: &str, color: Color, clear_bg: bool) {
    if clear_bg {
        frame.render_widget(Clear, area);
    }

    let popup_area = centered_rect(50, 20, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x.saturating_add(2),
        y: popup_area
            .y
            .saturating_add(popup_area.height / 2)
            .saturating_sub(1),
        width: popup_area.width.saturating_sub(4),
        height: 3_u16.min(popup_area.height.saturating_sub(2)),
    };

    let lines = vec![
        Line::from(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(subtitle)),
        Line::from(Span::styled(
            "Press any key to exit.",
            Style::default().fg(Color::Gray),
        )),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn render_confirm_quit(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(40, 20, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x.saturating_add(2),
        y: popup_area
            .y
            .saturating_add(popup_area.height / 2)
            .saturating_sub(1),
        width: popup_area.width.saturating_sub(4),
        height: 3_u16.min(popup_area.height.saturating_sub(2)),
    };

    let lines = vec![
        Line::from(Span::styled(
            "Quit game?",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "[Y] Yes   [N] No",
            Style::default().fg(Color::Gray),
        )),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
