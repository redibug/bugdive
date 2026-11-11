//! Battle chips — the Battle Network heart of the game.
//!
//! A chip is a single-use effect. You always hold a hand of 5, drawn from
//! a shuffled folder (deck). Using a chip draws a replacement. Discarding
//! is how you cycle a bad hand: mark any number of chips in the sidebar,
//! hit DISCARD, draw that many fresh ones — it costs your turn.

use crate::rng::Rng;

/// Chip shapes. Since the aiming update, every attack shape is built
/// from Bugchan's *facing* — face with WASD / Shift+arrows (free, no
/// turn), highlight a chip in the folder, and the map previews its
/// area of effect in yellow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Fire in a straight line along facing, up to `range` tiles,
    /// stopped by walls. Hits the FIRST virus in the line.
    Line { amount: i32, range: i32 },
    /// Like Line, plus the victim skips its next turn.
    LineStun { amount: i32, range: i32 },
    /// The single tile directly in front of Bugchan.
    FrontSingle { amount: i32 },
    /// The three tiles in front: front-left, front, front-right.
    FrontArc { amount: i32 },
    /// `len` tiles straight ahead; hits viruses on ALL of them (pierce).
    FrontLine { amount: i32, len: i32 },
    /// Lobbed `dist` tiles ahead (stops before a wall); explodes 3x3.
    Bomb { amount: i32, dist: i32 },
    /// A 3x3 blast centered `dist` tiles ahead.
    Blast { amount: i32, dist: i32 },
    Heal { amount: i32 },
    /// Absorbing shield, in HP.
    Barrier { amount: i32 },
    /// Viruses can't track you for `turns` turns.
    Invis { turns: i32 },
}

#[derive(Clone, Copy, Debug)]
pub struct ChipDef {
    pub name: &'static str,
    pub code: char,
    pub desc: &'static str,
    pub effect: Effect,
}

impl ChipDef {
    /// One-line effect summary for the sidebar list, e.g. "40 DMG line6".
    pub fn summary(&self) -> String {
        match self.effect {
            Effect::Line { amount, range } => format!("{amount} DMG line{range}"),
            Effect::LineStun { amount, range } => format!("{amount} DMG+stun line{range}"),
            Effect::FrontSingle { amount } => format!("{amount} DMG front"),
            Effect::FrontArc { amount } => format!("{amount} DMG arc"),
            Effect::FrontLine { amount, len } => format!("{amount} DMG front x{len}"),
            Effect::Bomb { amount, dist } => format!("{amount} DMG bomb@{dist}"),
            Effect::Blast { amount, dist } => format!("{amount} DMG blast@{dist}"),
            Effect::Heal { amount } => format!("heal {amount}"),
            Effect::Barrier { amount } => format!("barrier {amount}"),
            Effect::Invis { turns } => format!("invis {turns}t"),
        }
    }
}

pub const CHIPS: &[ChipDef] = &[
    ChipDef { name: "Cannon",   code: 'C', desc: "A trusty cannon blast. Fires in a straight line the way you're facing — first virus it meets eats it.", effect: Effect::Line { amount: 40, range: 6 } },
    ChipDef { name: "HiCannon", code: 'C', desc: "The cannon's bigger sister. Same line, twice the ouch.", effect: Effect::Line { amount: 80, range: 6 } },
    ChipDef { name: "M-Cannon", code: 'M', desc: "Military-grade cannon. Rare, rude, delightful.", effect: Effect::Line { amount: 120, range: 6 } },
    ChipDef { name: "Sword",    code: 'S', desc: "A clean energy blade. Hits the tile right in front of you.", effect: Effect::FrontSingle { amount: 80 } },
    ChipDef { name: "WideSwd",  code: 'S', desc: "A sweeping blade arc — the three tiles in front of you.", effect: Effect::FrontArc { amount: 70 } },
    ChipDef { name: "LongSwd",  code: 'L', desc: "An extra-long blade. Stabs through two tiles in a line.", effect: Effect::FrontLine { amount: 110, len: 2 } },
    ChipDef { name: "MiniBomb", code: 'B', desc: "Lobs a little bomb three tiles ahead. Explodes in a 3x3 — mind the splash (it only hurts viruses).", effect: Effect::Bomb { amount: 60, dist: 3 } },
    ChipDef { name: "Spreadr",  code: 'P', desc: "Scattershot. A 3x3 blast centered two tiles ahead.", effect: Effect::Blast { amount: 40, dist: 2 } },
    ChipDef { name: "ShockWav", code: 'W', desc: "A shockwave that travels far down your facing line.", effect: Effect::Line { amount: 60, range: 9 } },
    ChipDef { name: "ZapRing",  code: 'Z', desc: "A crackling ring down your facing line. Damage now, and the virus loses its next turn.", effect: Effect::LineStun { amount: 50, range: 5 } },
    ChipDef { name: "Recov30",  code: 'R', desc: "Patch data. Restores 30 HP.", effect: Effect::Heal { amount: 30 } },
    ChipDef { name: "Recov80",  code: 'R', desc: "Serious patch data. Restores 80 HP.", effect: Effect::Heal { amount: 80 } },
    ChipDef { name: "Recov120", code: '*', desc: "A full hot-fix, couriered from the surface. Restores 120 HP.", effect: Effect::Heal { amount: 120 } },
    ChipDef { name: "Barrier",  code: 'B', desc: "A soap-bubble shield that soaks 40 HP of damage first.", effect: Effect::Barrier { amount: 40 } },
    ChipDef { name: "Invis",    code: 'I', desc: "Ghost data. For 3 turns, viruses lose track of you and wander.", effect: Effect::Invis { turns: 3 } },
];

/// (index into CHIPS, copies in the folder)
const FOLDER_RECIPE: &[(usize, usize)] = &[
    (0, 4),  // Cannon
    (1, 2),  // HiCannon
    (2, 1),  // M-Cannon
    (3, 3),  // Sword
    (4, 2),  // WideSwd
    (5, 1),  // LongSwd
    (6, 3),  // MiniBomb
    (7, 2),  // Spreadr
    (8, 2),  // ShockWav
    (9, 2),  // ZapRing
    (10, 3), // Recov30
    (11, 2), // Recov80
    (12, 1), // Recov120
    (13, 2), // Barrier
    (14, 1), // Invis
];

pub const HAND_SIZE: usize = 5;

/// A fresh shuffled folder.
pub fn build_folder(rng: &mut Rng) -> Vec<usize> {
    let mut deck: Vec<usize> = Vec::new();
    for &(idx, copies) in FOLDER_RECIPE {
        for _ in 0..copies {
            deck.push(idx);
        }
    }
    rng.shuffle(&mut deck);
    deck
}
