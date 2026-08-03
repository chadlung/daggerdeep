mod combat;
mod dungeon;
mod entity;
mod game;
mod menu;
mod net;
mod protocol;
mod render;

use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::io::ErrorKind;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use game::{Game, GameState};
use menu::MenuResult;
use protocol::{ClientMsg, ServerMsg};

fn check_terminal_size() -> Result<(), String> {
    let (cols, rows) = size().map_err(|e| e.to_string())?;
    if cols < 100 || rows < 30 {
        Err(format!(
            "Terminal too small: {}×{}. Minimum required: 100×30.",
            cols, rows
        ))
    } else {
        Ok(())
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

/// Restores the terminal on every exit path, including early `?` returns.
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn main() -> Result<(), io::Error> {
    if let Err(msg) = check_terminal_size() {
        eprintln!("Error: {}", msg);
        std::process::exit(1);
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let _guard = TermGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    match menu::run_menu(terminal)? {
        MenuResult::Quit => {}
        MenuResult::SinglePlayer => {
            let mut game = Game::new(false);
            run_loop(terminal, &mut game)?;
        }
        MenuResult::Host(stream) => {
            let mut game = Game::new(true);
            run_loop_host(terminal, &mut game, stream)?;
        }
        MenuResult::Join(stream) => {
            run_loop_client(terminal, stream)?;
        }
    }
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|frame| render::render_game(frame, game))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press {
            match game.state {
                GameState::Playing => match key.code {
                    KeyCode::Esc => game.state = GameState::ConfirmQuit,
                    KeyCode::Char('+') => {
                        if game.level < 10 {
                            game.next_level();
                        }
                    }
                    KeyCode::Up => {
                        game.move_player(0, -1);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Down => {
                        game.move_player(0, 1);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Left => {
                        game.move_player(-1, 0);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Right => {
                        game.move_player(1, 0);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    _ => {}
                },
                GameState::ConfirmQuit => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        game.state = GameState::Playing;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                },
                GameState::GameOver | GameState::Victory | GameState::Disconnected => break,
            }
        }
    }
    Ok(())
}

fn run_loop_host(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
    stream: TcpStream,
) -> Result<(), io::Error> {
    let read_stream = stream.try_clone()?;
    let mut write_stream = stream;

    let (tx, rx) = mpsc::channel::<ClientMsg>();
    thread::spawn(move || {
        let mut s = read_stream;
        loop {
            match net::recv_msg::<ClientMsg>(&mut s) {
                Ok(msg) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(ClientMsg::Quit);
                    break;
                }
            }
        }
    });

    let mut p2_connected = true;

    loop {
        terminal.draw(|frame| render::render_game(frame, game))?;

        let mut action_taken = false;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press {
                match game.state {
                    GameState::Playing => match key.code {
                        KeyCode::Esc => game.state = GameState::ConfirmQuit,
                        KeyCode::Up => { game.move_player(0, -1); action_taken = true; }
                        KeyCode::Down => { game.move_player(0, 1); action_taken = true; }
                        KeyCode::Left => { game.move_player(-1, 0); action_taken = true; }
                        KeyCode::Right => { game.move_player(1, 0); action_taken = true; }
                        _ => {}
                    },
                    GameState::ConfirmQuit => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break,
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            game.state = GameState::Playing;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        _ => {}
                    },
                    GameState::GameOver | GameState::Victory | GameState::Disconnected => break,
                }
            }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ClientMsg::Move(dx, dy) => {
                    if game.state == GameState::Playing {
                        game.move_player2(dx, dy);
                        action_taken = true;
                    }
                }
                ClientMsg::Quit => {
                    game.player2 = None;
                    game.status_msg = String::from("P2 quit. Continuing solo.");
                    p2_connected = false;
                }
            }
        }

        if game.state == GameState::Playing && action_taken {
            game.move_monsters();
        }

        if p2_connected {
            let net_state = game.to_net_state();
            if net::send_msg(&mut write_stream, &ServerMsg::State(net_state)).is_err() {
                game.player2 = None;
                game.status_msg = String::from("P2 disconnected. Continuing solo.");
                p2_connected = false;
            }
        }
    }
    // Send final state so P2 sees the end screen, then disconnect.
    if p2_connected {
        let net_state = game.to_net_state();
        let _ = net::send_msg(&mut write_stream, &ServerMsg::State(net_state));
        let _ = net::send_msg(&mut write_stream, &ServerMsg::Disconnected);
    }
    let _ = write_stream.shutdown(std::net::Shutdown::Both);
    Ok(())
}

fn run_loop_client(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    stream: TcpStream,
) -> Result<(), io::Error> {
    let read_stream = stream.try_clone()?;
    let mut write_stream = stream;

    let (tx, rx) = mpsc::channel::<ServerMsg>();
    thread::spawn(move || {
        let mut s = read_stream;
        loop {
            match net::recv_msg::<ServerMsg>(&mut s) {
                Ok(msg) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    continue;
                }
                Err(_) => {
                    let _ = tx.send(ServerMsg::Disconnected);
                    break;
                }
            }
        }
    });

    let mut net_state: Option<protocol::NetGameState> = None;
    let mut solo_game: Option<Game> = None;

    'client: loop {
        terminal.draw(|frame| {
            if let Some(ref s) = net_state {
                render::render_net_state(frame, s);
            } else {
                render::render_connecting(frame);
            }
        })?;

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ServerMsg::State(s) => {
                    net_state = Some(s);
                }
                ServerMsg::Disconnected => {
                    let is_terminal = net_state.as_ref().is_some_and(|s| s.state == 1 || s.state == 2);
                    if !is_terminal {
                        solo_game = net_state.as_ref().and_then(Game::from_net_state_solo);
                    }
                    break 'client;
                }
            }
        }

        if let Some(ref s) = net_state
            && (s.state == 1 || s.state == 2) {
                if event::poll(Duration::from_millis(50))? {
                    event::read()?;
                    break;
                }
                continue;
            }

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Quit);
                        break;
                    }
                    KeyCode::Up => {
                        if net::send_msg(&mut write_stream, &ClientMsg::Move(0, -1)).is_err() {
                            solo_game = net_state.as_ref().and_then(Game::from_net_state_solo);
                            break 'client;
                        }
                    }
                    KeyCode::Down => {
                        if net::send_msg(&mut write_stream, &ClientMsg::Move(0, 1)).is_err() {
                            solo_game = net_state.as_ref().and_then(Game::from_net_state_solo);
                            break 'client;
                        }
                    }
                    KeyCode::Left => {
                        if net::send_msg(&mut write_stream, &ClientMsg::Move(-1, 0)).is_err() {
                            solo_game = net_state.as_ref().and_then(Game::from_net_state_solo);
                            break 'client;
                        }
                    }
                    KeyCode::Right => {
                        if net::send_msg(&mut write_stream, &ClientMsg::Move(1, 0)).is_err() {
                            solo_game = net_state.as_ref().and_then(Game::from_net_state_solo);
                            break 'client;
                        }
                    }
                    _ => {}
                }
            }
    }

    let _ = write_stream.shutdown(std::net::Shutdown::Both);

    if let Some(mut g) = solo_game {
        run_loop(terminal, &mut g)?;
    } else if net_state.as_ref().is_some_and(|s| s.state == 1 || s.state == 2) {
        // Game ended normally — show the end screen and wait for a keypress.
        loop {
            if let Some(ref s) = net_state {
                terminal.draw(|frame| render::render_net_state(frame, s))?;
            }
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    let _ = key;
                    break;
            }
        }
    } else {
        terminal.draw(render::render_disconnected)?;
        event::read()?;
    }

    Ok(())
}
