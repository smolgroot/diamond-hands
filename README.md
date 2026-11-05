# Diamond Hands


![Diamond Hands](./cover.png)

A side-scrolling action platformer for the Game Boy Advance, written in Rust with [agb](https://agbrs.dev).
In a dystopian future the bears have built machines. A wassie in a red poncho, hauling an
oversized laser cannon, blasts its way left to right through their cyber bots. Each level ends
with a boss.

1. **The Badlands** (playable): mossy cliffs below a ruined city, from a violet dusk to a burning
   night. Sentry walkers, Crawler spider-bots, Drones and rolling Mines, then **BERA 3000**, a
   bear-shaped war mech.
2. **Solana Trenches** (playable): a neon casino strip sunk into the rock, lit magenta and teal.
   It reuses the Badlands' tiles, bots and music under its own palettes, and ends with
   **BERA 3000 MK2**, the mech rebuilt after the Badlands.
3. **Ethereum City** (planned): a futuristic city of gas drones and MEV bots.

| Button      | Action                                                          |
| ----------- | --------------------------------------------------------------- |
| D-pad       | Move and aim: Up, Up+forward; in the air also Down, Down+forward |
| A           | Jump (hold for higher); keep holding to hover on the jetpack    |
| B           | Fire; hold and release for a charged beam                       |
| L or R      | Hold to plant your feet, so the d-pad only aims                 |
| Start       | Leaves the splash; opens and closes the pause menu; skips dialogue |

- **Splash:** the game boots to the cover art. After 1.5 s "PRESS START TO PLAY" blinks in,
  and Start (or A) opens the save files.
- **Save files:** there are three. An empty file goes straight to the title. A file with a run in
  it asks CONTINUE or NEW GAME. A new game leaves the old run in place until the new one saves,
  so a wrong pick loses nothing. Lighting a beacon saves the run, and so does clearing a level.
  CONTINUE (on the title, B goes back to the files) starts at the last beacon with full health;
  the diamonds and bots behind it stay collected or beaten. Between levels, CONTINUE opens the
  world map on the next level instead. Each file also keeps which levels it has cleared. Saves
  from before that was added are read and carried over: a run that had reached level N has
  the levels before it cleared.
- **Combat:** you can stomp bots by landing on them. A "!" over a bot means it is about to
  attack. The blaster fires in eight directions; diagonal shots fly at the same speed.
- **Jetpack:** after a jump's rise, keeping A held lights a jetpack that climbs slowly for
  48 frames. This reaches about 62 px, against 38 px for a plain jump, so the diamonds on
  the floating rocks are reachable. Letting go saves the rest of the fuel, landing or
  stomping a bot refuels it, and a quick tap in the air never hovers (it stays a buffered
  jump).
- **Arrival:** starting the Badlands fresh plays its arrival on CryptoTerra. A cargo ship drops the
  wassie off, rekt and owning nothing but its scarf. CL207, a white cat in a yellow cat-eared hazmat suit,
  explains the rules, then hands over the blaster and the jetpack. A reads the lines and Start
  skips the scene. Continuing a saved run never replays it.
- **Boss dialogue:** when the arena closes and the camera settles, the boss and the wassie
  trade a few lines. A finishes a line and then turns the page; Start skips the rest. It
  plays once per visit, so retries go straight to the fight.
- **Level clear:** the screen fades down, a ring of diamonds bursts off the wassie, the tally
  counts up row by row, and the place the clear unlocked is announced. A (after a second)
  or a 7-second wait opens the world map.
- **World map:** the hub between levels, and MAP on the title opens it too. Levels unlock in
  order: the Badlands is open on a new file, and clearing a level opens the next. Each place
  shows how far the file has got there: a diamond with a teal tick (cleared), a glinting
  diamond (open, with a "!" when newly unlocked), a padlock (locked: the line under it says
  which level to clear), or a steel "?" beacon (Ethereum City, coming soon). After a clear
  the wassie walks the road to the newly unlocked place. Left and Right move, A plays an
  open or cleared place, B goes back to the title. A cleared level can be replayed; replays
  bank no score and can't set a best. Every run saves to the file, at beacons and on clears.
- **Pause menu:** Start in a level opens it and the level waits; Start (or B) closes it and
  play carries on exactly where it stopped. Left/Right (or L/R) switch tabs:
  - CONTROLS lists every button with what it does in play and in menus. The rows come from
    the game's own input mapping (`logic/src/controls.rs`), so they can't drift from it.
  - BAG shows the wassie's gear: the blaster, the jetpack and the red scarf, each with an
    icon, its slot and an "E" when equipped. Up/Down pick an item and the box below
    describes it; A opens a card with a big icon, more detail and, for the jetpack, a live
    fuel gauge. B closes the card.
  - WALLET shows the diamonds in hand (the same count as the HUD), how many the level
    holds, the count saved at the last beacon, and the score.

  Buttons still held when the menu closes are ignored until let go, so the B that closes it
  doesn't fire and a charge held into the menu is dropped.
- **Health and respawns:** you have 8 HP. Touching a beacon makes it your respawn point, and
  retries are unlimited.
- **Diamonds:** they are scattered along the route, and the upper ledges hold the most.
- **Options:** sets the difficulty (Chill, Normal, Degen), which scales the damage you take and
  how often bots fire. It is saved to SRAM with the best score.

Art direction follows `cover.png` and the project skill in `.claude/skills/diamond-hands-gba/`.
The wassie's poses and effects follow the player sprite sheet there (`sprite-sheet.png`):
- the wassie has a gold rim light, a broad bill and a red neck scarf
- it squashes on a hard landing and leans back when it skids
- it sees stars when hit, and when knocked out it slumps, then falls flat
- dust puffs behind its heels when it runs, sprays ahead when it skids, and kicks up both
  ways on take-off and landing
- wind streaks show over a hard fall
- a forward shot throws a ring of light at the muzzle, and the laser bolt has a ringed head

## Layout

- `logic/`: all game rules. Tile physics, the player, villains, the boss, scenes, levels and the
  test probe. `no_std` and integer-only, tested on the host.
  - Levels are ASCII maps in `logic/src/levels/`. The legend is in `logic/src/level.rs`. Each
    level also carries its boss dialogue; places on the world map and the unlock rules are in
    `logic/src/levels.rs`.
  - The pause menu's state is in `logic/src/game.rs`; what it lists comes from
    `logic/src/controls.rs` (button bindings) and `logic/src/inventory.rs` (the bag's items).
    A new item needs an entry there and an icon in `game/src/art/menu.rs`.
- `game/`: the agb ROM. It reads buttons, steps `logic`, draws, and saves to SRAM.
  - All art is drawn in code at boot, in `game/src/art/`, except the splash. That is
    `cover.png` cropped and fitted to 16 palettes by `scripts/make-splash.py` (needs Pillow),
    which writes the tiles, palettes and a preview to `game/gfx/`. Rerun it when the cover
    changes; the ROM includes those files as they are.
  - Music is tracker modules in `game/music/*.xm`, played by agb-tracker on the mixer
    (`game/src/music.rs` picks the cue from the scene); the blaster effects stay on the DMG
    channels (`game/src/sound.rs`). The songs are written in code by
    `game/music/src/soundtrack.py` with synthesized instruments: run
    `python3 game/music/src/soundtrack.py` after changing it. Cues: title, Badlands, boss,
    level clear, knocked out.
  - It builds with its own nightly toolchain and target (`game/rust-toolchain.toml`,
    `game/.cargo/config.toml`), so it is excluded from the workspace.
- `harness/`: headless ROM tests on the [gba-wasm](https://github.com/smolgroot/gba-wasm) core,
  which must be checked out next to this repo.

## Build and run

Install agb-gbafix once:

```sh
cargo install agb-gbafix
```

Then:

```sh
./scripts/build-rom.sh            # → dist/diamond-hands.gba
(cd game && cargo run --release)  # play in mgba-qt
```

To play in gba-wasm, load `dist/diamond-hands.gba` in its browser UI (`pnpm dev` in `../gba-wasm`).

## Test

```sh
cargo test -p diamond_hands_logic    # rules, boss phases, map checks, route-bot clears
cargo test -p diamond_hands_harness  # boots the ROM on gba-wasm (needs dist/diamond-hands.gba)
```

The logic tests also check every map for authoring mistakes: unreachable steps, spawns inside
rock, the arena size, and diamonds on floating rock that the jetpack can't reach. They also
check that the dialogue fits the box and the font. A route bot then plays each level in god
mode and must beat the boss.

Each frame, the ROM mirrors a `Probe` struct into EWRAM. The harness finds it by its `DHPR` magic, then checks:
- scene flow
- running, jumping, firing
- pausing
- SRAM persistence
- hovering, and diagonal and downward aim
- the world map: locked and coming-soon places, and unlocking the Trenches by clearing the
  Badlands, which survives a power cycle
- the pause menu's tabs, items and inspect card, and that closing it neither fires nor
  moves the level on
- that the ROM matches the host-side model frame by frame while the route bot clears the
  Badlands, including the boss dialogue

Set `DIAMOND_HANDS_SHOTS=<dir>` to have the harness save screenshots (PPM) of the title, map,
hovering, aiming, each line of dialogue, the level-clear sequence, each pause-menu page and
the Trenches.

## Known limitations

- There is no sound yet.
- Two levels are playable. The Trenches reuse the Badlands' tile shapes, bots, boss and
  music under new palettes; Ethereum City is a placeholder.
- Sprite VRAM is nearly full: new sprites for later levels will need the level's art loaded
  per level rather than all at boot. Splitting the blaster from the body poses and shrinking
  the big font to 8x16 bought back room for the aims, jetpack, portraits and map markers.
- The world map is painted at boot, which adds about half a second to start-up.
- Background VRAM is nearly full too. All eight screenblocks are taken, so the pause menu
  borrows the dialogue box's unused rows, and while it is open the dialogue text lends it
  its tiles. The menu is drawn on an 8x8 grid so each glyph is one shared tile, and a page
  needs about 170-230 tiles. Switching tabs takes about a tenth of a second, and the new
  page wipes in over a few frames.

If a harness test fails, try the ROM in mGBA. If it behaves correctly there, the bug is in gba-wasm:
1. Record it in gba-wasm as a failing test or a `TASKS.md` entry.
2. Mark the harness test `#[ignore = "gba-wasm: …"]`.

## Website

The showcase site is a [VitePress](https://vitepress.dev) project in `site/` (custom theme matching wassie.wtf:
its MUI palette and Londrina Solid). `cd site && npm ci && npm run dev` to preview it; pushes to `master`
deploy it to GitHub Pages. Set `SITE_BASE=/` to build for a root domain.
