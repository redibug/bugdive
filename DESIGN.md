# BUGDIVE.EXE — design notes (v0.1)

## The fantasy
Battle Network's jack-in, as a roguelike. You are Bugchan, diving.
Depth is danger: same verb set, meaner rooms. The run ends in
deletion; the score is how deep + how much you deleted.

## Core loop
1. Explore a generated sector (rooms + corridors).
2. Fight viruses with positioning (bump buster), timing (chips),
   and hand management (discard cycling).
3. Find the dive point `>`, go deeper, repeat until deleted.

## Turn economy (from Mel's spec)
- A turn is spent by exactly one of: **move**, **use a chip**,
  **discard** (any number of marked chips at once).
- Discarding is a sidebar action: Tab in, Space to mark, Enter
  on the DISCARD MARKED row. It refills the hand and ends the turn.
- Free actions: opening/closing the folder, marking chips,
  bumping walls. Chip fizzles (no valid target / heal at full HP)
  are free too — misclicks shouldn't kill you; viruses should.

## Screen (from Mel's spec)
- **Top bar (1 line):** area name left, HP right (+ barrier,
  invis, zenny/frags, turn/kills on the left side).
- **Main:** left half map (camera follows), right half sidebar.
  Sidebar is contextual: NAVI status always; BATTLE CHIPS list;
  below that, CHIP DATA when the folder is focused, SCAN
  (nearby viruses) when it isn't; CONTROLS reflect the focus.
- **Bottom:** log, 5 visible lines.

## Chips (v0.1)
15 kinds, 31-chip folder. Targeting is automatic-nearest so a
turn is one keypress — the tactics live in range, positioning
and hand cycling, not in a targeting cursor. (A targeting
cursor is a v0.2 candidate for cannon-line chips.)

## Viruses (v0.1)
Greedy chase within 8 tiles, wander otherwise. CanoDrone adds
line-of-sight ranged fire. ZapRing's stun + Invis' tracking
loss are the two status effects. Depth scales count, kind
table, and stats.

## Not in v0.1 (deliberately)
Images (spec: none), FOV/shadowcasting (Bugchan has the sector
map — she's a bug, she lives here), saves, bosses, chip codes,
sound. See README roadmap.
