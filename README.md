# Diamond Hands


![Diamond Hands](./cover.png)

A side-scrolling action platformer for the Game Boy Advance, written in Rust with [agb](https://agbrs.dev).
In a dystopian future the bears have built machines. A wassie in a red poncho, hauling an
oversized laser cannon, blasts its way through their cyber bots. Each level ends with a boss.

1. **The Badlands** (playable): an explorable map, Metroidvania style, three screens tall.
   Mossy cliffs below a ruined city, from a violet dusk to a burning night. Sentry walkers,
   Crawler spider-bots, Drones and rolling Mines, then **BERA 3000**, a bear-shaped war mech.
   The way to her runs over a cliff face too high to jump. The only other way on is down a
   pit, which can't be climbed back up without the jetpack. Below it are the caves: a long
   cavern, a false wall hiding a secret room at its west end, and a shaft. The shaft drops to
   the deep, a crystal cave where the **jetpack** lies at the far west end, past electric
   fences. With the jetpack, the girders up the shaft climb all the way back to the plateau
   over the cliff, and the high ledges passed on the way down are in reach.
2. **Solana Trenches** (playable): not a battlefield but Crypto Twitter's memecoin casino,
   where a coin launches every minute and most of them rug. An explorable map, Metroidvania
   style, four storeys tall. CL207 calls as it starts. SAM runs the casino from his rooftop
   and has cut the power to his elevator. CL207 is also building a **Plasma Shield** and
   needs the three plasma cells hidden below ground.
   - **The strip and the casino floor:** a neon Vegas strip under a giant gold coin, with
     a plaza up a ramp. Then the casino floor: felt tables to hop, slot banks, roulette,
     trading screens, croupier bots dealing cards, rolling chips, a sunken bar and a false
     wall hiding a secret tunnel. The elevator lobby at its end is dead ("NO POWER").
   - **The back room:** down a hole in the casino floor, the underground casino. A grand
     ramp leads down to banks of slot machines and crypto ATMs, a high rollers' stage and,
     behind a false wall, the VIP vault with the first cell.
   - **The sewers:** down the back room's drain (or a manhole on the strip), a long brick
     corridor full of bots. Sentries, crawlers, guards and drones stand between two fences,
     a sunken channel and a raised walkway. A second cell waits in the bots' nest at the
     west end, and a third on a pipe above the walkway, up on the jetpack. The **breaker**
     at the east end brings the elevator's power back. The shaft beside it climbs on
     girders back up to a hatch in the lobby floor, which only opens from below.
   - **The tower:** the elevator rides up to the exchange suites (server racks, the vault
     up a ramp, armoured guard bots) and out onto the rooftop, where **SAM**, a cyborg
     exchange founder with a cucumber, waits under the open sky.

   The floors aren't all flat: ramps and slopes, steep and gentle, run through all four
   storeys. The casino pays in **gold coins**, which the penguin cashier changes into
   diamonds after the fight.
3. **Ethereum City** (unlocked by clearing the Trenches, not built yet): where CL207's
   Plasma Shield arrives.

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
- **Jetpack:** in the Badlands it has to be found first (in the deep); the Trenches start with
  it. After a jump's rise, keeping A held lights a jetpack that climbs slowly for
  48 frames. This reaches about 62 px, against 38 px for a plain jump, so the diamonds on
  the floating rocks are reachable. Letting go saves the rest of the fuel, landing or
  stomping a bot refuels it, and a quick tap in the air never hovers (it stays a buffered
  jump).
- **Arrival:** starting the Badlands fresh plays its arrival on CryptoTerra. A cargo ship drops the
  wassie off, rekt and owning nothing but its scarf. CL207, a white cat in a yellow cat-eared hazmat suit,
  explains the rules, then hands over the blaster. A bear bot dragged the jetpack he meant
  to give it down into the caves. A reads the lines and Start
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
  - BAG shows the wassie's gear: the blaster, the jetpack, CL207's plasma shield (not held
    until Ethereum City hands it over), the plasma cells and the red scarf, each with an
    icon, its slot and an "E" when equipped. The icons follow the inventory mockup in the
    project skill (`inventory-mockup.png`). Up/Down pick an item and the box below
    describes it; A opens a card with a big icon, more detail and, for the jetpack and the
    cells, a live gauge. B closes the card.
  - WALLET shows the diamond balance (banked plus this level's), the banked count, the
    level's diamonds found out of its total, gold coins (this level's and the purse), and
    the score.

  Buttons still held when the menu closes are ignored until let go, so the B that closes it
  doesn't fire and a charge held into the menu is dropped.
- **Two currencies:** diamonds are banked into the file's wallet when a level is cleared;
  the HUD shows the level's count. The Trenches pay **gold coins** instead: pickups, a
  bounty for each bot (1-3 coins) and SAM's stash (10). After SAM, the penguin cashier
  offers **1 coin = 8 diamonds**: the counter shows the coins, the rate and the diamonds
  they buy. EXCHANGE pays the wallet; KEEP MY COINS puts them in the file's purse, offered
  again at the next visit. The deal is struck once and saved with the level's clear.
  Replays bank no diamonds or coins, as they bank no score, but the purse can still be
  changed.
- **Plasma cells:** three in the Trenches, one in the back room and two in the sewers.
  Each is found once per file (the bag shows how many), and they stay found on replays.
- **Elevator:** its power is cut. Press Up at the breaker in the sewers and its lights come
  back on. Then, in the lobby, step into the open car and press Up again. A "^" bubble shows
  where Up does something. A terminal standing by a car (none does in the Trenches) can also
  be worked from the car's door.
- **Slopes:** walk up and down them like any floor. Standing on one, the wassie stands where
  the floor crosses its middle. Bots walk them too, and shots stop at the rock under them.
- **SAM:** opens with his **Cucumber Hit** (crackling aura, then a lunge and a giant
  cucumber along the floor that knocks you flying: jump it; afterwards he's winded and takes
  double damage), lobs arcing cucumbers, and calls a **Market Crash** (the screens go red,
  candles rain, magenta energy flies). At half health his implants overload: faster, three
  cucumbers a volley, and the Cucumber Hit ends in shockwaves.
- **Electric fences:** pylons on the ground throw up a curtain of lightning that reaches the
  rock above (or the sky), so it can't be jumped. They cycle: dead, a blinking warning with
  sparks, then live for about 2.5 seconds. Touching a live curtain shocks you and throws you
  back; wait for it to die, then run through. Fences in view crackle as they come on, and
  a shock buzzes loudly.
- **Health and respawns:** you have 8 HP. Touching a beacon makes it your respawn point, and
  retries are unlimited.
- **Diamonds:** they are scattered along the route, and the upper ledges hold the most.
- **Options:** sets the difficulty (Chill, Normal, Degen), which scales the damage you take and
  how often bots fire. It is saved to SRAM with the best score.

Art direction follows `cover.png` and the project skill in `.claude/skills/diamond-hands-gba/`.
The backdrops (sky and city) are muted, and the scenery pictures on the terrain less so
(`BACKDROP_MUTE` and `SCENERY_MUTE` in `game/src/art/mod.rs`). The wassie, the bots and
the ground they stand on keep their full colour, so they stand out.
The Badlands' ground follows the tiles mockup in `.claude/mockup/` (`level1-tiles.png`):
lime moss hanging over the edges in leafy lobes, two courses of rounded purple bricks
under it, then big slanted flagstones with blue crystals glinting in them, pillars of
stacked blocks up the cliff faces, a brick trim under ceilings and grey steel girders.
Below ground, open cells show a dim back wall instead of black, after
`level1-tiles-sprites-sheet-with-background.png`: an arcade of round arches on pillars
standing on the cavern's and the deep's floors, with brick above it. It is drawn in the
terrain layer (no background layer is free) in its own muted palette, BG slot 4, and
girders, fences and scenery underground are drawn over it in the same tile.
The Trenches' sewers follow `level2-sewers-tiles-sprites-sheet.png` and
`level2-sewers-background-tiles-sprites-sheet.png`: mossy green-grey brick under a steel
rail with hazard orange dashes (its lit top is the walking line), the same rail up the
ramps and along the catwalks, and behind the corridor a dark teal back wall of vaults on
pillars with a round grate in each bay, a ledge and water at the foot. The corridor has
outflow pipes with slime glowing in them, vines, pipework, barriers, a ladder up to the
breaker's catwalk and graffiti on the brick under the ledge. Every copy of a
picture sits at the same place in the back wall's 64 px bay, so the copies share tiles.
The wassie's poses and effects follow the player sprite sheet there (`sprite-sheet.png`):
- the wassie has a gold rim light, a broad bill and a red neck scarf
- it squashes on a hard landing and leans back when it skids
- it runs in a six-step cycle, pitched forward and bobbing, one foot reaching ahead while
  the other kicks up behind, with the cannon carried along
- it sees stars when hit. Knocked out in the air, it tumbles down to the floor first; the
  knockout count only starts once it lands, where it slumps, falls flat and lets go of the
  cannon
- the cannon is a chunky gunmetal piece on its own palette: a glowing power cell, a carry
  handle with a cyan sight, the diamond insignia, magenta vents, a finned barrel and a big
  muzzle ring with a white-hot core
- dust puffs behind its heels when it runs, sprays ahead when it skids, and kicks up both
  ways on take-off and landing
- wind streaks show over a hard fall
- a forward shot throws a ring of light at the muzzle, and the laser bolt has a ringed head

## Layout

- `logic/`: all game rules. Tile physics, the player, villains, the boss, scenes, levels and the
  test probe. `no_std` and integer-only, tested on the host.
  - Levels are ASCII maps in `logic/src/levels/`. The legend is in `logic/src/level.rs`,
    including the scenery pictures (`DECOR`: slots, screens, signs, ATMs, sewer pipes,
    graffiti) drawn over a block of their letter, and the slopes: `/` and `\` rise and fall
    a tile per tile, `{` and `}` a tile over two, in pairs. A slope is a floor under a body's
    centre only, cut from rock (`Body::step` in `logic/src/physics.rs`). Each
    level also carries its boss dialogue; places on the world map and the unlock rules are in
    `logic/src/levels.rs`.
  - A map is one screen (20 rows) tall, or taller for an explorable level (`explore`): the
    Badlands are 180 x 60, the Trenches 280 x 84. There the camera follows the wassie up and
    down too, the sky and the city fade to black below `sky_rows`, a resumed run comes back
    with exactly what it had taken (`World::taken`, saved per actor), and `route` lists
    waypoints for the route bot. `J` is the jetpack for a level that hides it; owning it is
    a bit of the file's part mask (`GEAR_JETPACK`), next to the plasma cells.
  - A map can also be built in storeys (`bands`). The camera then keeps to the storey the
    wassie is in and pans when it drops or climbs to another. A test checks that everywhere
    the wassie can stand shows whole in its storey. `strata` gives the rows where the
    renderer switches tileset and palette (the Trenches' back room and sewers). `areas`
    light a block of the map as another stage (the Trenches' tower top under the night
    sky). `backdrop_y` is the camera height the sky and the city were painted for.
  - Each map's markers are listed when the game compiles (`Markers::scan`): on the GBA the
    map sits in cartridge ROM, and reading every cell of a tall one as a level starts
    overran a frame.
  - The pause menu's state is in `logic/src/game.rs`; what it lists comes from
    `logic/src/controls.rs` (button bindings) and `logic/src/inventory.rs` (the bag's items).
    A new item needs an entry there and an icon in `game/src/art/menu.rs`.
- `game/`: the agb ROM. It reads buttons, steps `logic`, draws, and saves to SRAM.
  - All art is drawn in code, in `game/src/art/` (`casino.rs` and `trenches.rs` for the
    Trenches, after `.claude/skills/diamond-hands-gba/sprite-sheet-SBF.png` and
    `level2-casino-mockup.png`), except the splash, the wassie's portrait, SAM, the Sentry bot and the Guard. The splash
    is `cover.png` cropped and fitted to 16 palettes by `scripts/make-splash.py` (needs
    Pillow), which writes the tiles, palettes and a preview to `game/gfx/`. SAM's poses and
    portrait are cut from the SBF sprite sheet at gameplay size by `scripts/make-sam.py`
    (review them in `game/gfx/sam.png`); only his current pose sits in sprite VRAM. The Sentry
    bot's and the Guard's (the casino's ninja bot) poses are cut from
    `lvl1-bot-sprite-sheet.png` and `lvl2-bot-ninja-sprite-sheet.png` by
    `scripts/make-bots.py` (review in `game/gfx/sentry.png` and `ninja.png`); a pose is
    loaded only while something on screen shows it. Rerun a
    script when its source changes; the ROM includes those files as they are.
  - Music is tracker modules in `game/music/*.xm`, played by agb-tracker on the mixer
    (`game/src/music.rs` picks the cue from the scene); the blaster effects stay on the DMG
    channels (`game/src/sound.rs`). The songs are written in code by
    `game/music/src/soundtrack.py` with synthesized instruments: run
    `python3 game/music/src/soundtrack.py` after changing it. Cues: title, Badlands, boss,
    level clear, knocked out. The Trenches still play the Badlands' theme.
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
./scripts/build-rom.sh --dev      # → dist/diamond-hands-dev.gba: every level open on the map
(cd game && cargo run --release)  # play in mgba-qt
```

The dev ROM (`DEV` in the title screen's corner) opens every level on the world map from
a new file, so the Trenches can be played without beating the Badlands. It changes only
what the map lets you start: nothing extra is saved, and a level beaten there counts as a
first clear.

To play in gba-wasm, load `dist/diamond-hands.gba` in its browser UI (`pnpm dev` in `../gba-wasm`).

## Test

```sh
cargo test -p diamond_hands_logic    # rules, boss phases, map checks, route-bot clears
cargo test -p diamond_hands_harness  # boots the ROM on gba-wasm (needs dist/diamond-hands.gba)
```

The logic tests also check every map for authoring mistakes: unreachable steps, spawns inside
rock, the arena size, and diamonds on floating rock that the jetpack can't reach. They also
check that the dialogue fits the box and the font, that every slope has rock under it, and
that in a map built in storeys everywhere the wassie stands shows whole on screen. A route
bot then plays each level in god mode and must beat the boss. In an explorable level it
follows the level's waypoints there:
- the Badlands: down the pit, through the caves, to the jetpack and back up the shaft
- the Trenches: down the casino's hole, through the back room and down its drain, along
  the sewers to the breaker, up the shaft, then the elevator

The gates are checked too. In the Badlands, a jump can't top the cliff and the jetpack
can, and a resumed run keeps the jetpack and exactly what it had taken. In the Trenches,
the elevator stays dark until the breaker is thrown, the lobby's hatch holds from above,
the cells are below ground, and a resumed run finds the elevator as it was left.

Each frame, the ROM mirrors a `Probe` struct into EWRAM. The harness finds it by its `DHPR` magic, then checks:
- scene flow
- running, jumping, firing
- pausing
- SRAM persistence
- that the Badlands start without the jetpack, and diagonal and downward aim
- the world map: locked and coming-soon places, and unlocking the Trenches by clearing the
  Badlands, which survives a power cycle
- the pause menu's tabs, items and inspect card, and that closing it neither fires nor
  moves the level on
- that the ROM matches the host-side model frame by frame while the route bot clears the
  Badlands, including the boss dialogue. A frame that runs long on the ROM is followed rather
  than flagged: the model steps as often as the ROM's frame counter did (`step_in_sync`)

Set `DIAMOND_HANDS_SHOTS=<dir>` to have the harness save screenshots (PPM) of the title, map,
aiming, each line of dialogue, each waypoint on the Badlands' route (`b1-*`), the
level-clear sequence, each pause-menu page, and the Trenches' storeys (street, back room,
sewers, the shaft, the tower), SAM's attacks and the exchange (`t2-*`).

Set `DIAMOND_HANDS_PROFILE=<file>` to profile the ROM through Level 2 in
`the_trenches_play_through_to_the_exchange_in_step_with_the_model` (profiled frames aren't
drawn, so take screenshots in a separate run). Each video frame gets an `F` line: busy
CPU cycles out of 280,896, scene, camera x and y, bots and shots on screen, objects in
OAM, fence state, game frame, and whether the wassie's sprite is in OAM (`1`, `0`, or `-`
while it shouldn't show). Then `P <pc> <cycles>` lines give cycles by program counter, to
look up with `nm -C --numeric-sort` or `addr2line` on
`game/target/thumbv4t-none-eabi/release/diamond-hands`.
`DIAMOND_HANDS_PROFILE_WATCH=<hex start>-<hex end>,...` also logs who calls the leaf
functions in those address ranges (`C <range> <lr> <cycles>`).

## Known limitations

- Two levels are playable; Ethereum City shows on the map once unlocked but can't be
  played yet. The Trenches have no music of their own yet.
- Sprite VRAM is full. Each level's own sprites load while it is on screen (`LevelArt` in
  `game/src/render.rs`): BERA 3000 and the arrival ship in the Badlands, the casino's in the
  Trenches. Even alone the casino's don't fit, so they come in two halves swapped as the
  camera goes: the floors' (bots, elevator, terminal) and the arena's (SAM, the penguin).
- The world map is painted each time it opens (about half a second) and dropped when it
  closes: its tiles don't fit in background VRAM beside a level's. Moving between the
  Badlands and the Trenches repaints the backdrop over three frames for the same reason.
- `World` keeps its actor table on the heap, and `Game` its `World`: built on the IWRAM
  stack they overflowed it at boot.
- The heap is easily fragmented: agb copies a background's tile map whenever it changes,
  and the pause menu (two pages of about 19 KB) and the world map need large blocks
  beside a level's terrain. Running out panics the ROM, and the harness then sees its
  frame counter stop. The tile cache (`TileCache` in `game/src/art/mod.rs`) keeps 8 bytes
  per tile in one sorted block to leave room. The Trenches play-through opens every
  pause page up the tower, where the terrain is fullest, then goes on to the world map.
- Saves from before the casino (version 5) are moved over with an empty wallet: version 5
  kept no diamond balance. A run saved part-way through the old Trenches starts the
  rebuilt level from the top, and one part-way through the Badlands from before they became
  explorable (version 6 and older) starts them from the top too.
- Background VRAM can't hold all of the Trenches' terrain at once. The sewers and the
  floors above them each keep their tiles only while the camera is there (`region` in
  `game/src/render.rs`): once the camera is wholly in one, the other's tiles are dropped
  and the map is cleared of them off screen, half a column a frame.
- The back room has no backdrop of its own yet: there the sky and the city fade to black
  behind the rock. The caves and the sewers draw a back wall instead.
- Slopes are drawn only in the Trenches' tilesets; the Badlands have none.
- A file that found plasma cells in the old Trenches keeps them: the cells are numbered in
  map order, so they count as the new map's first cells.
- The ROM runs close to its frame budget. Each terrain cell costs agb's VRAM manager a few
  thousand cycles, so the renderer loads terrain a little ahead of the camera, makes the
  level's tiles ahead of time on the level card and in the menus, and after a respawn
  refills the screen over about ten frames, dark behind the sprites. Opening BERA 3000's
  dialogue drops a frame on this map (the whole frame runs about three times slower, game
  step included); the cause isn't tracked down yet.
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
