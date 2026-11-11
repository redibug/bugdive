//! BUGDIVE.EXE — a cyberspace roguelike.
//!
//! You are Bugchan, a living computer bug, diving the net. Every floor is
//! deeper, darker, and full of nastier viruses. Your weapons are a humble
//! BugBuster and a folder of battle chips: a hand of 5, single-use, with
//! multi-discard cycling straight out of Battle Network.
//!
//! Layout: top bar (area + HP) / map | sidebar / log. No images.

mod chip;
mod game;
mod map;
mod rng;
mod ui;
mod virus;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use game::{Focus, Game};

fn seed_from_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_nanos() & 0xFFFF_FFFF_FFFF_FFFF) as u64)
        .unwrap_or(0xB06C_4A4E)
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut game = Game::new(seed_from_clock());
    let result = run(&mut terminal, &mut game);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, game: &mut Game) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, game))?;

        let Event::Key(key) = event::read()? else {
            continue; // resize etc: just redraw
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
        {
            return Ok(());
        }

        if game.game_over {
            match key.code {
                KeyCode::Char('r') | KeyCode::Char('R') => game.restart(seed_from_clock()),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(()),
                _ => {}
            }
            continue;
        }

        match game.focus {
            Focus::Map => {
                if handle_map_key(game, key.code) == Flow::Quit {
                    return Ok(());
                }
            }
            Focus::Sidebar => {
                if handle_sidebar_key(game, key.code) == Flow::Quit {
                    return Ok(());
                }
            }
        }
    }
}

#[derive(PartialEq)]
enum Flow {
    Stay,
    Quit,
}

fn handle_map_key(game: &mut Game, code: KeyCode) -> Flow {
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Flow::Quit,
        // Movement: arrows, wasd-ish hjkl, and diagonals.
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            game.try_move(0, -1);
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            game.try_move(0, 1);
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            game.try_move(-1, 0);
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            game.try_move(1, 0);
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            game.try_move(-1, -1);
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            game.try_move(1, -1);
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            game.try_move(-1, 1);
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            game.try_move(1, 1);
        }
        KeyCode::Char('w') => {
            game.try_move(0, -1);
        }
        KeyCode::Char('s') => {
            game.try_move(0, 1);
        }
        KeyCode::Char('a') => {
            game.try_move(-1, 0);
        }
        KeyCode::Char('d') => {
            game.try_move(1, 0);
        }
        KeyCode::Char('.') | KeyCode::Char(' ') => game.wait_turn(),
        KeyCode::Char('>') => game.descend(),
        KeyCode::Enter if game.on_stairs() => game.descend(),
        // Folder / chips
        KeyCode::Tab | KeyCode::Char('c') | KeyCode::Char('C') => {
            game.focus = Focus::Sidebar;
            game.sidebar_sel = 0.min(game.hand.len());
        }
        KeyCode::Char(ch @ '1'..='5') => {
            let idx = (ch as usize) - ('1' as usize);
            game.use_chip(idx);
        }
        _ => {}
    }
    Flow::Stay
}

fn handle_sidebar_key(game: &mut Game, code: KeyCode) -> Flow {
    let rows = game.hand.len() + 1; // chips + DISCARD row
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') => return Flow::Quit,
        KeyCode::Esc | KeyCode::Tab => game.focus = Focus::Map,
        KeyCode::Up | KeyCode::Char('k') => {
            game.sidebar_sel = if game.sidebar_sel == 0 { rows - 1 } else { game.sidebar_sel - 1 };
        }
        KeyCode::Down | KeyCode::Char('j') => {
            game.sidebar_sel = (game.sidebar_sel + 1) % rows;
        }
        KeyCode::Char(' ') | KeyCode::Char('x') | KeyCode::Char('X') => {
            if game.sidebar_sel < game.hand.len() {
                game.toggle_mark(game.sidebar_sel);
            }
        }
        KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') | KeyCode::Char('4') | KeyCode::Char('5') => {
            if let KeyCode::Char(ch) = code {
                game.sidebar_sel = ((ch as usize) - ('1' as usize)).min(game.hand.len());
            }
        }
        KeyCode::Enter => {
            if game.sidebar_sel < game.hand.len() {
                game.use_chip(game.sidebar_sel);
            } else {
                game.discard_marked();
            }
        }
        KeyCode::Char('D') => {
            game.sidebar_sel = game.hand.len(); // jump to DISCARD
        }
        _ => {}
    }
    Flow::Stay
}
