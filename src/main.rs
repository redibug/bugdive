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

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use game::{Focus, Game};

/// Facing keys: WASD (either case) and Shift+arrows. Facing is FREE —
/// it never spends a turn — and works in both map and chip focus, so
/// you can swing your aim around while the folder is open and watch
/// the yellow preview follow.
fn facing_from_key(key: &KeyEvent) -> Option<(i32, i32)> {
    match key.code {
        KeyCode::Char('w') | KeyCode::Char('W') => Some((0, -1)),
        KeyCode::Char('s') | KeyCode::Char('S') => Some((0, 1)),
        KeyCode::Char('a') | KeyCode::Char('A') => Some((-1, 0)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some((1, 0)),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => Some((0, -1)),
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => Some((0, 1)),
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => Some((-1, 0)),
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => Some((1, 0)),
        _ => None,
    }
}

fn seed_from_clock() -> u64 {
    // BUGDIVE_SEED=<n> makes a run reproducible (bug reports, tests).
    if let Ok(s) = std::env::var("BUGDIVE_SEED") {
        if let Ok(n) = s.trim().parse::<u64>() {
            return n;
        }
    }
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

        // Poll (not blocking read) so the 0.5s attack flash clears on
        // time even when nobody is pressing anything.
        let timeout = if game.flash_active() {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(250)
        };
        if !event::poll(timeout)? {
            continue;
        }
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

        // Facing first: free, turnless, both focuses.
        if let Some((dx, dy)) = facing_from_key(&key) {
            game.face(dx, dy);
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
        // (WASD is facing now, intercepted in run() — never movement.)
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
        // (Shift+D / 'D' is facing-right now, intercepted in run().
        //  The DISCARD row is one ↓ past the last chip — selection wraps.)
        _ => {}
    }
    Flow::Stay
}
