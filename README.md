# Diamond Hands


![Diamond Hands](./cover.png)

A side-scrolling platform adventure for the Game Boy Advance, written in Rust with [agb](https://agbrs.dev).
A wassie in a red poncho, packing a laser blaster, fights its way left to right through the bears'
territory. Each level ends with a boss.

1. **The Canyon** (playable): a Bitcoin western from noon to nightfall. Bandit Bears, rattlesnakes,
   FUD vultures and tumbleweeds, then **Sheriff Ursa**.
2. **Solana Trenches** (planned): a neon casino strip full of hustlers.
3. **Ethereum City** (planned): a futuristic city of gas drones and MEV bots.

| Button      | Action                                       |
| ----------- | -------------------------------------------- |
| D-pad       | Move; Up aims the blaster skyward            |
| A           | Jump (hold for higher); confirm in menus     |
| B           | Fire; hold and release for a charged beam    |
| Start       | Pause                                        |

You can also stomp enemies by landing on them. You have 8 HP. Touching a ₿ flag makes it your
respawn point, and retries are unlimited. **Options** sets the difficulty (Chill, Normal, Degen),
which scales the damage you take and how often villains fire. It is saved to SRAM with the best score.

## Layout

- `logic/`: all game rules. Tile physics, the player, villains, the boss, scenes, levels and the
  test probe. `no_std` and integer-only, tested on the host.
  - Levels are ASCII maps in `logic/src/levels/`. The legend is in `logic/src/level.rs`.
- `game/`: the agb ROM. It reads buttons, steps `logic`, draws, and saves to SRAM.
  - All art is drawn in code at boot, in `game/src/art/`.
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
rock, and the arena size. A route bot then plays each level in god mode and must beat the boss.

Each frame, the ROM mirrors a `Probe` struct into EWRAM. The harness finds it by its `DHPR` magic, then checks:
- scene flow
- running, jumping, firing
- pausing
- SRAM persistence
- that the ROM matches the host-side model frame by frame while the route bot clears the Canyon

If a harness test fails, try the ROM in mGBA. If it behaves correctly there, the bug is in gba-wasm:
1. Record it in gba-wasm as a failing test or a `TASKS.md` entry.
2. Mark the harness test `#[ignore = "gba-wasm: …"]`.
