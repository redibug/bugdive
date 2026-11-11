//! The natives. Virus kinds get meaner the deeper you dive: the spawn
//! table shifts, and every kind gets +HP / +damage per extra depth.

#[derive(Clone, Copy, Debug)]
pub struct VirusDef {
    pub name: &'static str,
    pub glyph: char,
    pub hp: i32,
    pub dmg: i32,
    pub ranged: bool,
    pub min_depth: i32,
    pub weight: u32,
}

pub const VIRUSES: &[VirusDef] = &[
    VirusDef { name: "Glitchlet",  glyph: 'g', hp: 30,  dmg: 6,  ranged: false, min_depth: 1, weight: 30 },
    VirusDef { name: "SpamWorm",   glyph: 'w', hp: 20,  dmg: 5,  ranged: false, min_depth: 1, weight: 25 },
    VirusDef { name: "Mett",       glyph: 'M', hp: 55,  dmg: 10, ranged: false, min_depth: 1, weight: 20 },
    VirusDef { name: "CanoDrone",  glyph: 'C', hp: 40,  dmg: 12, ranged: true,  min_depth: 2, weight: 18 },
    VirusDef { name: "Swordy",     glyph: 'S', hp: 65,  dmg: 16, ranged: false, min_depth: 3, weight: 14 },
    VirusDef { name: "NullWraith", glyph: 'N', hp: 100, dmg: 20, ranged: false, min_depth: 6, weight: 8 },
];

#[derive(Clone, Debug)]
pub struct Virus {
    pub def: usize, // index into VIRUSES
    pub pos: (i32, i32),
    pub hp: i32,
    pub max_hp: i32,
    pub dmg: i32,
    pub stun: i32,
}

impl Virus {
    pub fn new(def: usize, pos: (i32, i32), depth: i32) -> Self {
        let d = &VIRUSES[def];
        let bonus = (depth - 1).max(0);
        let hp = d.hp + bonus * 8;
        Self {
            def,
            pos,
            hp,
            max_hp: hp,
            dmg: d.dmg + bonus * 2,
            stun: 0,
        }
    }
    pub fn name(&self) -> &'static str {
        VIRUSES[self.def].name
    }
    pub fn glyph(&self) -> char {
        VIRUSES[self.def].glyph
    }
    pub fn ranged(&self) -> bool {
        VIRUSES[self.def].ranged
    }
}

/// Pick a virus kind for this depth, weighted, respecting min_depth.
pub fn roll_kind(rng: &mut crate::rng::Rng, depth: i32) -> usize {
    let total: u32 = VIRUSES
        .iter()
        .filter(|v| depth >= v.min_depth)
        .map(|v| v.weight)
        .sum();
    let mut roll = rng.below(total.max(1) as usize) as u32;
    for (i, v) in VIRUSES.iter().enumerate() {
        if depth < v.min_depth {
            continue;
        }
        if roll < v.weight {
            return i;
        }
        roll -= v.weight;
    }
    0
}
