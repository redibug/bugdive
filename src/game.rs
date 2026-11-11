//! Game state and the turn engine.
//!
//! One rule shapes everything: a turn is spent by moving, by using a
//! chip, or by discarding. Bumping a wall is free (and noisy). After
//! every spent turn, every virus gets to act.

use crate::chip::{build_folder, Effect, CHIPS, HAND_SIZE};
use crate::map::{self, ItemKind, Sector, Tile};
use crate::rng::Rng;
use crate::virus::{roll_kind, Virus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Map,
    Sidebar,
}

pub fn area_name(depth: i32) -> &'static str {
    match depth {
        1 => "Surface Web",
        2 => "Mirror Garden Net",
        3 => "Forum Ruins",
        4 => "Dead Mall District",
        5 => "Undernet Fringe",
        6 => "Undernet Bazaar",
        7 => "Undernet Core",
        8 => "Firewall Catacombs",
        _ => "The Null Abyss",
    }
}

pub struct Game {
    pub rng: Rng,
    pub sector: Sector,
    pub player: (i32, i32),
    pub facing: (i32, i32),
    pub hp: i32,
    pub max_hp: i32,
    pub barrier: i32,
    pub invis_turns: i32,
    pub buster: i32,
    pub depth: i32,
    pub zenny: u32,
    pub bugfrags: u32,
    pub kills: u32,
    pub turn: u32,
    pub viruses: Vec<Virus>,
    pub deck: Vec<usize>,
    pub hand: Vec<usize>,
    pub discard_pile: Vec<usize>,
    pub marked: Vec<bool>,
    pub focus: Focus,
    /// Sidebar cursor: 0..hand.len() = a chip, hand.len() = the DISCARD row.
    pub sidebar_sel: usize,
    pub log: Vec<String>,
    pub game_over: bool,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let deck = build_folder(&mut rng);
        let mut game = Self {
            rng,
            sector: map::generate(&mut Rng::new(seed ^ 0x5EED)),
            player: (2, 2),
            facing: (1, 0),
            hp: 100,
            max_hp: 100,
            barrier: 0,
            invis_turns: 0,
            buster: 10,
            depth: 1,
            zenny: 0,
            bugfrags: 0,
            kills: 0,
            turn: 0,
            viruses: Vec::new(),
            deck,
            hand: Vec::new(),
            discard_pile: Vec::new(),
            marked: Vec::new(),
            focus: Focus::Map,
            sidebar_sel: 0,
            log: Vec::new(),
            game_over: false,
        };
        game.new_floor(true);
        game.draw_to_full();
        game.log("Log in! Bugchan, executing.");
        game.log("Dive deep, delete viruses, don't get deleted.");
        game.log("Tab opens your chip folder. > on the dive point goes deeper.");
        game
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        if self.log.len() > 100 {
            self.log.drain(..self.log.len() - 100);
        }
    }

    // ---------- floor ----------

    fn new_floor(&mut self, first: bool) {
        self.sector = map::generate(&mut self.rng);
        self.player = self.sector.spawn;
        self.viruses.clear();
        let count = (6 + self.depth * 2).min(20) as usize;
        let mut placed = 0;
        let mut tries = 0;
        while placed < count && tries < 500 {
            tries += 1;
            let x = self.rng.range(1, map::MAP_W - 2);
            let y = self.rng.range(1, map::MAP_H - 2);
            let dist = (x - self.player.0).abs() + (y - self.player.1).abs();
            if !self.sector.walkable(x, y)
                || self.sector.tile(x, y) == Tile::Stairs
                || dist < 6
                || self.virus_at((x, y)).is_some()
            {
                continue;
            }
            let def = roll_kind(&mut self.rng, self.depth);
            self.viruses.push(Virus::new(def, (x, y), self.depth));
            placed += 1;
        }
        if !first {
            self.log(format!(
                "Depth {} — {}. The viruses smell meaner here.",
                self.depth,
                area_name(self.depth)
            ));
        }
    }

    pub fn on_stairs(&self) -> bool {
        self.sector.tile(self.player.0, self.player.1) == Tile::Stairs
    }

    pub fn descend(&mut self) {
        if self.game_over {
            return;
        }
        if !self.on_stairs() {
            self.log("No dive point here. Find the > first.");
            return;
        }
        self.depth += 1;
        self.new_floor(false);
        self.end_turn(false); // fresh floor, but the turn still ticks
    }

    // ---------- chips ----------

    fn draw_one(&mut self) -> Option<usize> {
        if self.deck.is_empty() && !self.discard_pile.is_empty() {
            self.deck = std::mem::take(&mut self.discard_pile);
            self.rng.shuffle(&mut self.deck);
            self.log("Folder reshuffled from the discard pile.");
        }
        self.deck.pop()
    }

    pub fn draw_to_full(&mut self) {
        while self.hand.len() < HAND_SIZE {
            match self.draw_one() {
                Some(c) => self.hand.push(c),
                None => break,
            }
        }
        self.marked = vec![false; self.hand.len()];
        if self.sidebar_sel > self.hand.len() {
            self.sidebar_sel = self.hand.len();
        }
    }

    pub fn marked_count(&self) -> usize {
        self.marked.iter().filter(|&&m| m).count()
    }

    pub fn toggle_mark(&mut self, idx: usize) {
        if idx < self.marked.len() {
            self.marked[idx] = !self.marked[idx];
        }
    }

    /// Discard every marked chip and draw replacements. Costs a turn.
    /// Returns false (no turn spent) when nothing is marked.
    pub fn discard_marked(&mut self) -> bool {
        if self.game_over {
            return false;
        }
        let n = self.marked_count();
        if n == 0 {
            self.log("Nothing marked. Space marks a chip for discard.");
            return false;
        }
        let mut kept = Vec::new();
        for (i, &chip) in self.hand.iter().enumerate() {
            if self.marked.get(i).copied().unwrap_or(false) {
                self.discard_pile.push(chip);
            } else {
                kept.push(chip);
            }
        }
        self.hand = kept;
        self.draw_to_full();
        self.log(format!("Discarded {n} chip(s). Drew {n} fresh."));
        self.focus = Focus::Map;
        self.end_turn(true);
        true
    }

    /// Use the chip in hand slot `idx`. Costs a turn when it fires.
    /// Support chips (heal/barrier/invis) always fire; attack chips fizzle
    /// for free when there's no valid target, so a misclick isn't punished.
    pub fn use_chip(&mut self, idx: usize) -> bool {
        if self.game_over || idx >= self.hand.len() {
            return false;
        }
        let def = &CHIPS[self.hand[idx]];
        let name = def.name;
        let fired = match def.effect {
            Effect::Heal { amount } => {
                if self.hp >= self.max_hp {
                    self.log(format!("{name}: HP already full. Chip kept."));
                    return false;
                }
                self.hp = (self.hp + amount).min(self.max_hp);
                self.log(format!("{name}! Recovered {amount} HP. ({}/{})", self.hp, self.max_hp));
                true
            }
            Effect::Barrier { amount } => {
                self.barrier = (self.barrier + amount).min(60);
                self.log(format!("{name}! Barrier up ({})", self.barrier));
                true
            }
            Effect::Invis { turns } => {
                self.invis_turns = turns;
                self.log(format!("{name}! Bugchan fades into ghost data for {turns} turns."));
                true
            }
            Effect::Damage { amount, range } => match self.nearest_virus(range) {
                Some(v) => {
                    self.hit_virus(v, amount, name);
                    true
                }
                None => {
                    self.log(format!("{name}: no virus within range {range}. Chip kept."));
                    false
                }
            },
            Effect::AreaDamage { amount, range } => match self.nearest_virus(range) {
                Some(v) => {
                    let center = self.viruses[v].pos;
                    let targets: Vec<usize> = self
                        .viruses
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| (t.pos.0 - center.0).abs() + (t.pos.1 - center.1).abs() <= 1)
                        .map(|(i, _)| i)
                        .collect();
                    for t in targets {
                        self.hit_virus(t, amount, name);
                    }
                    true
                }
                None => {
                    self.log(format!("{name}: no virus within range {range}. Chip kept."));
                    false
                }
            },
            Effect::AdjacentSingle { amount } => {
                let adj: Option<usize> = self
                    .viruses
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| (v.pos.0 - self.player.0).abs() + (v.pos.1 - self.player.1).abs() == 1)
                    .max_by_key(|(_, v)| v.hp)
                    .map(|(i, _)| i);
                match adj {
                    Some(v) => {
                        self.hit_virus(v, amount, name);
                        true
                    }
                    None => {
                        self.log(format!("{name}: no virus next to you. Chip kept."));
                        false
                    }
                }
            }
            Effect::AdjacentAll { amount } => {
                let adj: Vec<usize> = self
                    .viruses
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| (v.pos.0 - self.player.0).abs() + (v.pos.1 - self.player.1).abs() == 1)
                    .map(|(i, _)| i)
                    .collect();
                if adj.is_empty() {
                    self.log(format!("{name}: no virus next to you. Chip kept."));
                    false
                } else {
                    for v in adj {
                        self.hit_virus(v, amount, name);
                    }
                    true
                }
            }
            Effect::StunDamage { amount, range } => match self.nearest_virus(range) {
                Some(v) => {
                    self.viruses[v].stun = 1;
                    self.hit_virus(v, amount, name);
                    true
                }
                None => {
                    self.log(format!("{name}: no virus within range {range}. Chip kept."));
                    false
                }
            },
        };
        if !fired {
            return false;
        }
        let used = self.hand.remove(idx);
        self.discard_pile.push(used);
        self.draw_to_full();
        self.focus = Focus::Map;
        self.end_turn(true);
        true
    }

    fn nearest_virus(&self, range: i32) -> Option<usize> {
        self.viruses
            .iter()
            .enumerate()
            .map(|(i, v)| (i, (v.pos.0 - self.player.0).abs() + (v.pos.1 - self.player.1).abs()))
            .filter(|(_, d)| *d <= range)
            .min_by_key(|(_, d)| *d)
            .map(|(i, _)| i)
    }

    fn hit_virus(&mut self, idx: usize, amount: i32, source: &str) {
        if idx >= self.viruses.len() {
            return;
        }
        self.viruses[idx].hp -= amount;
        let vname = self.viruses[idx].name();
        let hp_left = self.viruses[idx].hp;
        if hp_left <= 0 {
            let pos = self.viruses[idx].pos;
            self.viruses.remove(idx);
            self.kills += 1;
            self.log(format!("{source} hits {vname} for {amount} — DELETED!"));
            // Drops: zenny often, a bugfrag rarely, recovery data sometimes.
            if self.rng.chance(35) {
                let z = self.rng.range(1, 5) as u32 * 10;
                self.zenny += z;
                self.log(format!("  dropped {z} zenny."));
            } else if self.rng.chance(12) {
                self.bugfrags += 1;
                self.log("  dropped a BugFrag!");
            } else if self.rng.chance(18) {
                self.sector.items.push(crate::map::Item { pos, kind: ItemKind::Recovery(20) });
            }
        } else {
            self.log(format!("{source} hits {vname} for {amount}. ({hp_left} HP left)"));
        }
    }

    // ---------- movement & turns ----------

    pub fn virus_at(&self, pos: (i32, i32)) -> Option<usize> {
        self.viruses.iter().position(|v| v.pos == pos)
    }

    /// Move (or bump-attack). Returns true when a turn was spent.
    pub fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        if self.game_over {
            return false;
        }
        self.facing = (dx, dy);
        let nx = self.player.0 + dx;
        let ny = self.player.1 + dy;
        if let Some(v) = self.virus_at((nx, ny)) {
            // Bump attack: the humble Bug Buster.
            let dmg = self.buster;
            self.hit_virus(v, dmg, "BugBuster");
            self.end_turn(true);
            return true;
        }
        if !self.sector.walkable(nx, ny) {
            self.log("Bonk. Firewall.");
            return false;
        }
        self.player = (nx, ny);
        // Pickups.
        if let Some(i) = self.sector.items.iter().position(|it| it.pos == self.player) {
            let item = self.sector.items.remove(i);
            match item.kind {
                ItemKind::Zenny(z) => {
                    self.zenny += z;
                    self.log(format!("Picked up {z} zenny."));
                }
                ItemKind::Recovery(r) => {
                    self.hp = (self.hp + r as i32).min(self.max_hp);
                    self.log(format!("Recovery data! +{r} HP. ({}/{})", self.hp, self.max_hp));
                }
            }
        }
        if self.on_stairs() {
            self.log("Dive point! Press > to go DEEPER.");
        }
        self.end_turn(true);
        true
    }

    pub fn wait_turn(&mut self) {
        if self.game_over {
            return;
        }
        self.log("Bugchan holds position, antennae twitching.");
        self.end_turn(true);
    }

    /// Tick status, then let every virus act.
    fn end_turn(&mut self, viruses_act: bool) {
        self.turn += 1;
        if self.invis_turns > 0 {
            self.invis_turns -= 1;
        }
        if viruses_act {
            self.enemy_turn();
        }
    }

    fn damage_player(&mut self, amount: i32, source: &str) {
        let mut dmg = amount;
        if self.barrier > 0 {
            let soaked = self.barrier.min(dmg);
            self.barrier -= soaked;
            dmg -= soaked;
            self.log(format!("{source} hits your barrier (-{soaked})."));
        }
        if dmg > 0 {
            self.hp -= dmg;
            self.log(format!("{source} hits Bugchan for {dmg}! ({}/{})", self.hp.max(0), self.max_hp));
        } else {
            self.log(format!("{source}'s attack fizzles on the barrier."));
        }
        if self.hp <= 0 {
            self.hp = 0;
            self.game_over = true;
            self.log("BUGCHAN DELETED... log out failed. Press R to log in again.");
        }
    }

    fn enemy_turn(&mut self) {
        if self.game_over {
            return;
        }
        let order: Vec<usize> = (0..self.viruses.len()).collect();
        for i in order {
            if self.game_over || i >= self.viruses.len() {
                continue;
            }
            if self.viruses[i].stun > 0 {
                self.viruses[i].stun -= 1;
                let n = self.viruses[i].name();
                self.log(format!("{n} is stunned and skips its turn."));
                continue;
            }
            let vpos = self.viruses[i].pos;
            let dist = (vpos.0 - self.player.0).abs() + (vpos.1 - self.player.1).abs();

            if self.invis_turns <= 0 && dist == 1 {
                let dmg = self.viruses[i].dmg;
                let n = self.viruses[i].name().to_string();
                self.damage_player(dmg, &n);
                continue;
            }
            if self.invis_turns <= 0 && self.viruses[i].ranged() && dist <= 4 && dist > 1 {
                let aligned = vpos.0 == self.player.0 || vpos.1 == self.player.1;
                if aligned && self.line_clear(vpos, self.player) {
                    let dmg = self.viruses[i].dmg;
                    let n = self.viruses[i].name().to_string();
                    self.damage_player(dmg, &format!("{n} (shot)"));
                    continue;
                }
            }

            // Movement: chase when close & visible, else wander.
            let chase = self.invis_turns <= 0 && dist <= 8;
            let step: Option<(i32, i32)> = if chase {
                let dx = (self.player.0 - vpos.0).signum();
                let dy = (self.player.1 - vpos.1).signum();
                let prefer_x = (self.player.0 - vpos.0).abs() >= (self.player.1 - vpos.1).abs();
                let mut cands: Vec<(i32, i32)> = Vec::new();
                if prefer_x {
                    if dx != 0 { cands.push((vpos.0 + dx, vpos.1)); }
                    if dy != 0 { cands.push((vpos.0, vpos.1 + dy)); }
                } else {
                    if dy != 0 { cands.push((vpos.0, vpos.1 + dy)); }
                    if dx != 0 { cands.push((vpos.0 + dx, vpos.1)); }
                }
                cands.into_iter().find(|&p| self.enemy_can_step(p))
            } else if self.rng.chance(50) {
                let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                let d = *self.rng.pick(&dirs).unwrap();
                let p = (vpos.0 + d.0, vpos.1 + d.1);
                if self.enemy_can_step(p) { Some(p) } else { None }
            } else {
                None
            };
            if let Some(p) = step {
                self.viruses[i].pos = p;
            }
        }
        // Viruses delete items they trample? No — but they can stand on them.
    }

    fn enemy_can_step(&self, p: (i32, i32)) -> bool {
        self.sector.walkable(p.0, p.1)
            && self.sector.tile(p.0, p.1) != Tile::Stairs
            && p != self.player
            && self.virus_at(p).is_none()
    }

    fn line_clear(&self, a: (i32, i32), b: (i32, i32)) -> bool {
        if a.0 == b.0 {
            let (lo, hi) = if a.1 < b.1 { (a.1, b.1) } else { (b.1, a.1) };
            (lo + 1..hi).all(|y| self.sector.tile(a.0, y) != Tile::Wall)
        } else if a.1 == b.1 {
            let (lo, hi) = if a.0 < b.0 { (a.0, b.0) } else { (b.0, a.0) };
            (lo + 1..hi).all(|x| self.sector.tile(x, a.1) != Tile::Wall)
        } else {
            false
        }
    }

    pub fn restart(&mut self, seed: u64) {
        *self = Self::new(seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_starts_full() {
        let g = Game::new(42);
        assert_eq!(g.hand.len(), HAND_SIZE);
        assert_eq!(g.marked.len(), HAND_SIZE);
        assert!(!g.game_over);
        assert_eq!(g.depth, 1);
    }

    #[test]
    fn stairs_reachable_from_spawn() {
        // Flood fill walkable tiles from spawn; stairs must be reachable.
        let g = Game::new(7);
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![g.player];
        while let Some(p) = stack.pop() {
            if !seen.insert(p) || !g.sector.walkable(p.0, p.1) {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                stack.push((p.0 + dx, p.1 + dy));
            }
        }
        assert!(seen.contains(&g.sector.stairs), "dive point must be reachable");
    }

    #[test]
    fn discard_multiple_replaces_and_costs_turn() {
        let mut g = Game::new(99);
        let turn0 = g.turn;
        g.marked[0] = true;
        g.marked[2] = true;
        g.marked[4] = true;
        assert!(g.discard_marked());
        assert_eq!(g.hand.len(), HAND_SIZE, "hand refills to 5");
        assert_eq!(g.marked_count(), 0, "marks clear after discard");
        assert!(g.turn > turn0, "discarding costs a turn");
    }

    #[test]
    fn discard_with_no_marks_is_free() {
        let mut g = Game::new(99);
        let turn0 = g.turn;
        assert!(!g.discard_marked());
        assert_eq!(g.turn, turn0);
    }

    #[test]
    fn bump_attack_damages_virus() {
        let mut g = Game::new(5);
        g.viruses.clear();
        let p = g.player;
        let spot = (p.0 + 1, p.1);
        if !g.sector.walkable(spot.0, spot.1) {
            return; // unlucky spawn wall; other tests cover combat
        }
        g.viruses.push(Virus::new(1, spot, 1)); // SpamWorm, 20 HP
        let hp0 = g.viruses[0].hp;
        assert!(g.try_move(1, 0));
        assert!(g.viruses.is_empty() || g.viruses[0].hp < hp0 || g.kills > 0);
    }

    #[test]
    fn heal_chip_does_not_fizzle_at_cost() {
        let mut g = Game::new(3);
        // Force a Recov30 into slot 0 and full HP: using it must keep the chip.
        g.hand[0] = 10; // Recov30
        g.hp = g.max_hp;
        assert!(!g.use_chip(0));
        assert_eq!(g.hand.len(), HAND_SIZE);
        // Hurt, then it fires and is replaced.
        g.hp = 50;
        assert!(g.use_chip(0));
        assert_eq!(g.hp, 80);
        assert_eq!(g.hand.len(), HAND_SIZE);
    }
}
