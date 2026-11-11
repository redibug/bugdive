//! Battle chips — the Battle Network heart of the game.
//!
//! A chip is a single-use effect. You always hold a hand of 5, drawn from
//! a shuffled folder (deck). Using a chip draws a replacement. Discarding
//! is how you cycle a bad hand: mark any number of chips in the sidebar,
//! hit DISCARD, draw that many fresh ones — it costs your turn.

use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Hit the nearest virus within `range` (Manhattan).
    Damage { amount: i32, range: i32 },
    /// Hit the nearest virus within `range`, plus every virus next to it.
    AreaDamage { amount: i32, range: i32 },
    /// Hit the single strongest virus standing next to Bugchan.
    AdjacentSingle { amount: i32 },
    /// Hit every virus standing next to Bugchan.
    AdjacentAll { amount: i32 },
    /// Damage + the target skips its next turn.
    StunDamage { amount: i32, range: i32 },
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
    /// One-line effect summary for the sidebar list, e.g. "40 DMG rng6".
    pub fn summary(&self) -> String {
        match self.effect {
            Effect::Damage { amount, range } => format!("{amount} DMG rng{range}"),
            Effect::AreaDamage { amount, range } => format!("{amount} DMG area rng{range}"),
            Effect::AdjacentSingle { amount } => format!("{amount} DMG adj"),
            Effect::AdjacentAll { amount } => format!("{amount} DMG all-adj"),
            Effect::StunDamage { amount, range } => format!("{amount} DMG+stun rng{range}"),
            Effect::Heal { amount } => format!("heal {amount}"),
            Effect::Barrier { amount } => format!("barrier {amount}"),
            Effect::Invis { turns } => format!("invis {turns}t"),
        }
    }
}

pub const CHIPS: &[ChipDef] = &[
    ChipDef { name: "Cannon",   code: 'C', desc: "A trusty cannon blast. Hits the nearest virus in range.", effect: Effect::Damage { amount: 40, range: 6 } },
    ChipDef { name: "HiCannon", code: 'C', desc: "The cannon's bigger sister. Same range, twice the ouch.", effect: Effect::Damage { amount: 80, range: 6 } },
    ChipDef { name: "M-Cannon", code: 'M', desc: "Military-grade cannon. Rare, rude, delightful.", effect: Effect::Damage { amount: 120, range: 6 } },
    ChipDef { name: "Sword",    code: 'S', desc: "A clean energy blade. Hits the toughest virus next to you.", effect: Effect::AdjacentSingle { amount: 80 } },
    ChipDef { name: "WideSwd",  code: 'S', desc: "A sweeping blade arc. Hits every virus next to you.", effect: Effect::AdjacentAll { amount: 70 } },
    ChipDef { name: "LongSwd",  code: 'L', desc: "An extra-long blade for an extra-bad neighbour.", effect: Effect::AdjacentSingle { amount: 110 } },
    ChipDef { name: "MiniBomb", code: 'B', desc: "Lobs a little bomb at the nearest virus — splash hits its neighbours.", effect: Effect::AreaDamage { amount: 60, range: 5 } },
    ChipDef { name: "Spreadr",  code: 'P', desc: "Scattershot. Modest damage, but it splashes.", effect: Effect::AreaDamage { amount: 40, range: 6 } },
    ChipDef { name: "ShockWav", code: 'W', desc: "A shockwave that travels far across the data plane.", effect: Effect::Damage { amount: 60, range: 9 } },
    ChipDef { name: "ZapRing",  code: 'Z', desc: "A crackling ring. Damage now, and the virus loses its next turn.", effect: Effect::StunDamage { amount: 50, range: 5 } },
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
