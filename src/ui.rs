//! Layout, exactly as specced:
//!
//! ```text
//! ┌ top bar: area name (left) ............ HP etc (right) ┐  1 line
//! ├──────────────────────┬───────────────────────────────┤
//! │                      │  sidebar: status / chips /    │
//! │         MAP          │  context info / controls      │
//! │                      │                               │
//! ├──────────────────────┴───────────────────────────────┤
//! │ log (a few lines)                                    │
//! └──────────────────────────────────────────────────────┘
//! ```
//! No images anywhere — pure terminal cells.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::chip::CHIPS;
use crate::game::{area_name, Focus, Game};
use crate::map::{ItemKind, Tile};
use crate::virus::VIRUSES;

pub const PINK: Color = Color::Rgb(255, 105, 180);
pub const DIM: Color = Color::Rgb(120, 120, 140);
pub const CYAN: Color = Color::Rgb(90, 220, 220);
pub const GOLD: Color = Color::Rgb(255, 205, 80);

fn hp_color(g: &Game) -> Color {
    let pct = if g.max_hp > 0 { g.hp * 100 / g.max_hp } else { 0 };
    if pct > 50 {
        Color::Green
    } else if pct > 25 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn hp_bar(g: &Game, width: usize) -> String {
    let filled = if g.max_hp > 0 {
        (g.hp.max(0) as usize * width) / g.max_hp as usize
    } else {
        0
    };
    format!("[{}{}]", "█".repeat(filled), "░".repeat(width - filled))
}

pub fn draw(frame: &mut Frame, g: &Game) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top bar
            Constraint::Min(0),    // main
            Constraint::Length(7), // log: 5 lines + borders
        ])
        .split(area);

    draw_top_bar(frame, rows[0], g);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    draw_map(frame, cols[0], g);
    draw_sidebar(frame, cols[1], g);
    draw_log(frame, rows[2], g);

    if g.game_over {
        draw_game_over(frame, area, g);
    }
}

fn draw_top_bar(frame: &mut Frame, area: Rect, g: &Game) {
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left = Paragraph::new(Line::from(vec![
        Span::styled(" BUGDIVE.EXE ", Style::default().fg(Color::Black).bg(PINK).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            format!("{} — Depth {}", area_name(g.depth), g.depth),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "   turn {} · kills {} · dive > {} away",
                g.turn,
                g.kills,
                (g.sector.stairs.0 - g.player.0).abs() + (g.sector.stairs.1 - g.player.1).abs()
            ),
            Style::default().fg(DIM),
        ),
    ]));
    frame.render_widget(left, halves[0]);

    let mut right_spans = vec![
        Span::styled(format!("{} ", hp_bar(g, 12)), Style::default().fg(hp_color(g))),
        Span::styled(
            format!("HP {}/{}", g.hp, g.max_hp),
            Style::default().fg(hp_color(g)).add_modifier(Modifier::BOLD),
        ),
    ];
    if g.barrier > 0 {
        right_spans.push(Span::styled(format!("  BAR {}", g.barrier), Style::default().fg(CYAN)));
    }
    if g.invis_turns > 0 {
        right_spans.push(Span::styled(format!("  INVIS {}", g.invis_turns), Style::default().fg(Color::Magenta)));
    }
    right_spans.push(Span::styled(format!("  {}z  {} frag ", g.zenny, g.bugfrags), Style::default().fg(GOLD)));
    let right = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);
    frame.render_widget(right, halves[1]);
}

fn draw_map(frame: &mut Frame, area: Rect, g: &Game) {
    let border = if g.focus == Focus::Map { PINK } else { DIM };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" {} ", area_name(g.depth)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Camera: keep Bugchan centred, clamped to the sector.
    let vw = inner.width as i32;
    let vh = inner.height as i32;
    let cam_x = (g.player.0 - vw / 2).clamp(0, (crate::map::MAP_W - vw).max(0));
    let cam_y = (g.player.1 - vh / 2).clamp(0, (crate::map::MAP_H - vh).max(0));

    // Aiming preview: while a chip is highlighted in the folder, its
    // area of effect glows yellow on the map.
    let preview: Vec<(i32, i32)> = if g.focus == Focus::Sidebar && g.sidebar_sel < g.hand.len() {
        g.chip_tiles(g.hand[g.sidebar_sel])
    } else {
        Vec::new()
    };
    let flash_on = g.flash_active();

    let buf = frame.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (wx, wy) = (cam_x + sx, cam_y + sy);
            let cell_x = inner.x + sx as u16;
            let cell_y = inner.y + sy as u16;
            let (ch, mut style) = cell_for(g, wx, wy);
            // Attack flash wins over the preview: red for half a second.
            if flash_on && g.flash_tiles.contains(&(wx, wy)) {
                style = style.bg(Color::Red).fg(Color::White);
            } else if preview.contains(&(wx, wy)) {
                style = style.bg(Color::Yellow).fg(Color::Black);
            }
            buf.set_string(cell_x, cell_y, ch, style);
        }
    }
}

fn cell_for(g: &Game, x: i32, y: i32) -> (String, Style) {
    if (x, y) == g.player {
        let style = if g.invis_turns > 0 {
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(PINK).add_modifier(Modifier::BOLD)
        };
        return ("B".to_string(), style);
    }
    if let Some(vi) = g.virus_at((x, y)) {
        let v = &g.viruses[vi];
        let color = match VIRUSES[v.def].glyph {
            'g' => Color::Green,
            'w' => Color::Yellow,
            'M' => Color::Rgb(255, 140, 60),
            'C' => Color::Cyan,
            'S' => Color::Rgb(200, 120, 255),
            _ => Color::Red,
        };
        return (v.glyph().to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD));
    }
    if let Some(item) = g.sector.items.iter().find(|it| it.pos == (x, y)) {
        return match item.kind {
            ItemKind::Zenny(_) => ("$".to_string(), Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
            ItemKind::Recovery(_) => ("+".to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        };
    }
    match g.sector.tile(x, y) {
        Tile::Wall => ("#".to_string(), Style::default().fg(Color::Rgb(70, 70, 95))),
        Tile::Floor => ("·".to_string(), Style::default().fg(Color::Rgb(55, 60, 80))),
        Tile::Stairs => (">".to_string(), Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
    }
}

fn draw_sidebar(frame: &mut Frame, area: Rect, g: &Game) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // navi status
            Constraint::Length(10), // battle chips
            Constraint::Min(5),     // context panel
            Constraint::Length(7),  // controls
        ])
        .split(area);

    draw_status(frame, rows[0], g);
    draw_chips(frame, rows[1], g);
    draw_context(frame, rows[2], g);
    draw_controls(frame, rows[3], g);
}

fn draw_status(frame: &mut Frame, area: Rect, g: &Game) {
    let lines = vec![
        Line::from(vec![
            Span::styled(" BUGCHAN ", Style::default().fg(PINK).add_modifier(Modifier::BOLD)),
            Span::styled(if g.invis_turns > 0 { "· ghost data" } else { "· net navi" }, Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::raw(" HP "),
            Span::styled(hp_bar(g, 16), Style::default().fg(hp_color(g))),
            Span::styled(format!(" {}/{}", g.hp, g.max_hp), Style::default().fg(hp_color(g))),
        ]),
        Line::from(format!(" Barrier {}   Invis {}", g.barrier, g.invis_turns)),
        Line::from(format!(" Buster {} DMG   Facing {}", g.buster, facing_arrow(g.facing))),
        Line::from(format!(" Depth {}   Zenny {}   BugFrags {}", g.depth, g.zenny, g.bugfrags)),
        Line::from(format!(
            " Folder {} left · discard {} · viruses here: {}",
            g.deck.len(),
            g.discard_pile.len(),
            g.viruses.len()
        )),
    ];
    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" NAVI "));
    frame.render_widget(p, area);
}

fn facing_arrow(f: (i32, i32)) -> &'static str {
    match f {
        (1, 0) => "▶",
        (-1, 0) => "◀",
        (0, 1) => "▼",
        (0, -1) => "▲",
        _ => "·",
    }
}

fn draw_chips(frame: &mut Frame, area: Rect, g: &Game) {
    let border = if g.focus == Focus::Sidebar { PINK } else { DIM };
    let mut lines: Vec<Line> = Vec::new();
    for (i, &chip_idx) in g.hand.iter().enumerate() {
        let def = &CHIPS[chip_idx];
        let selected = g.focus == Focus::Sidebar && g.sidebar_sel == i;
        let marked = g.marked.get(i).copied().unwrap_or(false);
        let cursor = if selected { "▶" } else { " " };
        let box_str = if marked { "[x]" } else { "[ ]" };
        let style = if selected {
            Style::default().fg(Color::Black).bg(PINK).add_modifier(Modifier::BOLD)
        } else if marked {
            Style::default().fg(GOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{cursor}{} {}. {} [{}] — {}", box_str, i + 1, def.name, def.code, def.summary()),
            style,
        )));
    }
    // The DISCARD row: the whole point of marking chips.
    let discard_sel = g.focus == Focus::Sidebar && g.sidebar_sel == g.hand.len();
    let n = g.marked_count();
    let dstyle = if discard_sel {
        Style::default().fg(Color::Black).bg(GOLD).add_modifier(Modifier::BOLD)
    } else if n > 0 {
        Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {}▶ DISCARD MARKED ({n}) — draw {n} fresh", if discard_sel { "" } else { " " }),
        dstyle,
    )));
    lines.push(Line::from(Span::styled(
        if g.focus == Focus::Sidebar {
            " ↑↓ select · Space mark · Enter use/discard"
        } else {
            " Tab: open folder · 1-5: quick-use a chip"
        },
        Style::default().fg(DIM),
    )));
    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .title(" BATTLE CHIPS — hand of 5 "),
    );
    frame.render_widget(p, area);
}

fn draw_context(frame: &mut Frame, area: Rect, g: &Game) {
    if g.focus == Focus::Sidebar {
        // Chip data for the current sidebar selection.
        let mut lines = Vec::new();
        if g.sidebar_sel < g.hand.len() {
            let def = &CHIPS[g.hand[g.sidebar_sel]];
            lines.push(Line::from(Span::styled(
                format!("{} [{}]", def.name, def.code),
                Style::default().fg(PINK).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!(" {}", def.summary())));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(def.desc, Style::default().fg(Color::Gray))));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if g.marked.get(g.sidebar_sel).copied().unwrap_or(false) {
                    " Marked for discard. Space unmarks, Enter uses it anyway."
                } else {
                    " Enter: use it (costs a turn). Space: mark for discard."
                },
                Style::default().fg(DIM),
            )));
        } else {
            lines.push(Line::from(Span::styled(" DISCARD ", Style::default().fg(GOLD).add_modifier(Modifier::BOLD))));
            lines.push(Line::from(format!(" {} chip(s) marked.", g.marked_count())));
            lines.push(Line::from(Span::styled(
                " Enter throws them back and draws fresh ones.",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                " Costs your turn — the viruses won't wait politely.",
                Style::default().fg(DIM),
            )));
        }
        let p = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" CHIP DATA "));
        frame.render_widget(p, area);
    } else {
        // Scan: viruses near Bugchan, nearest first.
        let mut near: Vec<(i32, usize)> = g
            .viruses
            .iter()
            .enumerate()
            .map(|(i, v)| ((v.pos.0 - g.player.0).abs() + (v.pos.1 - g.player.1).abs(), i))
            .collect();
        near.sort();
        let mut lines = vec![Line::from(Span::styled(
            format!(" {} virus(es) on this floor", g.viruses.len()),
            Style::default().fg(DIM),
        ))];
        if near.is_empty() {
            lines.push(Line::from(" All clear. Find the > and dive."));
        }
        for (dist, i) in near.iter().take(5) {
            let v = &g.viruses[*i];
            let hpw = 8usize;
            let filled = ((v.hp.max(0) as usize) * hpw) / (v.max_hp.max(1) as usize);
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", v.glyph()), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:<11}", v.name())),
                Span::styled(
                    format!("[{}{}]", "█".repeat(filled), "░".repeat(hpw - filled)),
                    Style::default().fg(Color::Red),
                ),
                Span::raw(format!(" {}/{} · {} away", v.hp, v.max_hp, dist)),
            ]));
        }
        if g.on_stairs() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " On the dive point — press > to descend!",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            )));
        }
        let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" SCAN "));
        frame.render_widget(p, area);
    }
}

fn draw_controls(frame: &mut Frame, area: Rect, g: &Game) {
    let text = if g.focus == Focus::Map {
        vec![
            Line::from(" move: arrows / hjkl (+ yubn diagonals)"),
            Line::from(" face: WASD / Shift+arrows (free, no turn)"),
            Line::from(" bump a virus: BugBuster attack"),
            Line::from(" 1-5 use chip · Tab folder · . wait"),
            Line::from(" > descend (on dive point) · q quit"),
        ]
    } else {
        vec![
            Line::from(" ↑↓ / jk: select chip"),
            Line::from(" WASD / Shift+arrows: aim — preview is yellow"),
            Line::from(" Space / x: mark for discard"),
            Line::from(" Enter: use chip / run DISCARD"),
            Line::from(" Tab / Esc: back to the map"),
        ]
    };
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" CONTROLS "));
    frame.render_widget(p, area);
}

fn draw_log(frame: &mut Frame, area: Rect, g: &Game) {
    let inner_h = area.height.saturating_sub(2) as usize;
    let visible: Vec<Line> = g
        .log
        .iter()
        .rev()
        .take(inner_h.max(1))
        .rev()
        .map(|s| Line::from(Span::raw(format!(" {s}"))))
        .collect();
    let p = Paragraph::new(visible).block(Block::default().borders(Borders::ALL).title(" LOG "));
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::map::Tile;
    use crate::virus::Virus;
    use ratatui::{backend::TestBackend, Terminal};

    fn count_bg(g: &Game, color: Color) -> usize {
        let backend = TestBackend::new(120, 38);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| draw(f, g)).unwrap();
        let buf = term.backend().buffer().clone();
        buf.content.iter().filter(|c| c.bg == color).count()
    }

    #[test]
    fn highlighted_chip_previews_yellow_and_attack_flashes_red() {
        let mut g = Game::new(11);
        // Find an open eastward run and stage a cannon shot.
        let mut spot = None;
        'outer: for y in 1..crate::map::MAP_H - 1 {
            for x in 1..crate::map::MAP_W - 8 {
                if (0..7).all(|i| g.sector.tile(x + i, y) == Tile::Floor) {
                    spot = Some((x, y));
                    break 'outer;
                }
            }
        }
        let Some((x, y)) = spot else { return };
        g.viruses.clear();
        g.player = (x + 3, y);
        g.face(1, 0);
        g.viruses.push(Virus::new(2, (x + 6, y), 1));
        g.hand[0] = 0; // Cannon
        g.focus = Focus::Sidebar;
        g.sidebar_sel = 0;
        assert!(count_bg(&g, Color::Yellow) >= 6, "firing line previews in yellow");
        assert!(count_bg(&g, Color::Red) == 0, "no flash before any attack");
        assert!(g.use_chip(0));
        assert!(count_bg(&g, Color::Red) >= 1, "the hit tile flashes red");
    }
}

fn draw_game_over(frame: &mut Frame, area: Rect, g: &Game) {
    let w = 46u16.min(area.width);
    let h = 11u16.min(area.height);
    let x = area.x + (area.width - w) / 2;
    let y = area.y + (area.height - h) / 2;
    let rect = Rect::new(x, y, w, h);
    frame.render_widget(Clear, rect);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  B U G C H A N   D E L E T E D  ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  Reached depth {} of {}", g.depth, area_name(g.depth))),
        Line::from(format!("  Viruses deleted: {}   Turns: {}", g.kills, g.turn)),
        Line::from(format!("  Zenny banked: {}   BugFrags: {}", g.zenny, g.bugfrags)),
        Line::from(""),
        Line::from(Span::styled("  R: log in again    Q: log out", Style::default().fg(CYAN))),
    ];
    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" CONNECTION LOST "),
    );
    frame.render_widget(p, rect);
}
