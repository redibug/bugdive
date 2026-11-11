# BUGDIVE.EXE 🐞

A cyberspace roguelike in the terminal. No images — pure ratatui cells.

You are **Bugchan**, a living computer bug, jacked into the net.
Every dive takes you one layer deeper, and the deeper you go,
the nastier the viruses get. Your weapons: a humble BugBuster
(bump into things) and a folder of **battle chips**.

```
┌────────────────────────────────────────────────────────┐
│ BUGDIVE.EXE  Mirror Garden Net — Depth 2      HP 84/100 │
├─────────────────────────┬──────────────────────────────┤
│  · · # # · · · g ·      │  NAVI / BATTLE CHIPS / SCAN  │
│  · B · · · M · · >      │  (sidebar changes with       │
│  · · · $ · · · · ·      │   what you're doing)         │
├─────────────────────────┴──────────────────────────────┤
│ LOG: Cannon hits Mett for 40. (15 HP left)             │
└────────────────────────────────────────────────────────┘
```

## Run it

You need Rust (rustup is the easy way — https://rustup.rs).

**Windows**

```powershell
git clone https://github.com/redibug/bugdive.git
cd bugdive
cargo run --release
```

**Linux (Arch and friends)**

```sh
git clone https://github.com/redibug/bugdive.git
cd bugdive
cargo run --release
```

Use a decent-sized terminal (100×30 or more feels good).
Works fine smaller — the map camera follows you.

## How to play

One turn = **one** of these: move, use a chip, or discard.
Viruses move after every turn you spend. Bumping a wall is free.

| Key | What it does |
|---|---|
| Arrows / `hjkl` / WASD | Move (diagonals: `y` `u` `b` `n`) |
| Bump a virus | BugBuster attack (10 DMG) |
| `1`–`5` | Quick-use the chip in that slot |
| `Tab` or `C` | Open the chip folder (sidebar focus) |
| `.` or `Space` (map) | Wait a turn |
| `>` (on a dive point) | Descend to the next depth |
| `Q` / `Ctrl-C` | Jack out (quit) |
| `R` (when deleted) | Re-jack in — new run |

### Battle chips & the discard rule

- Your hand is always **5 chips**, drawn from a 31-chip folder.
  Chips are single-use; a used chip goes to the discard pile and
  you draw a replacement. Empty folder? The discard pile reshuffles.
- Attack chips need a valid target in range — if there isn't one,
  the chip **fizzles for free** (kept, no turn spent).
- **Discarding:** press `Tab`, move with `↑`/`↓`, press `Space`
  to mark any number of chips (`[x]`), select **DISCARD MARKED**
  and press `Enter`. You draw that many fresh chips —
  and it costs your turn, so don't do it with a Swordy in your face.
  (Probably.)

### The dive

- `>` is the dive point. The `SCAN` panel lists nearby viruses.
- Floors are generated server-rooms + corridors; every floor's
  dive point is reachable.
- Viruses scale with depth: more of them, tougher kinds
  (CanoDrones shoot from depth 2, Swordies from 3,
  NullWraiths from 6), and everything gains +8 HP / +2 DMG
  per depth past the first.
- Viruses drop zenny, BugFrags, and recovery data. `$` on the
  floor is a zenny cache, `+` is +20 HP recovery data.
- Death is deletion. How deep did you get?

Areas, in order: Surface Web → Mirror Garden Net → Forum Ruins →
Dead Mall District → Undernet Fringe → Undernet Bazaar →
Undernet Core → Firewall Catacombs → **The Null Abyss**.

## Layout of the code

| File | Job |
|---|---|
| `src/main.rs` | Entry, event loop, key handling (map focus vs sidebar focus) |
| `src/game.rs` | Game state + turn engine (move / chip / discard, virus turns) |
| `src/map.rs` | Sector generation: rooms + corridors, loot, dive point |
| `src/chip.rs` | Chip definitions, effects, and the folder recipe |
| `src/virus.rs` | Virus bestiary and depth scaling |
| `src/ui.rs` | The ratatui layout: top bar / map+sidebar / log |
| `src/rng.rs` | Tiny deterministic RNG (splitmix64) — runs are seedable for tests |

`cargo test` covers the important promises: hand starts at 5,
the dive point is always reachable, multi-discard refills and
costs a turn, and a heal chip at full HP doesn't eat your turn.

## Roadmap ideas

- Chip codes & Program Advances (three Cannons → Zeta Cannon?)
- Boss viruses on milestone depths
- Mystery Data that adds a chip to your folder permanently
- Save / continue a dive
- More virus behaviours (splitting, hiding, area denial)

MIT licensed. A Cutievirus-adjacent toy by Mel & Bugchan.
