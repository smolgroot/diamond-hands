# Working on Sample Metroidvania with a coding agent

This game's rooms, world map, tilesets, sprites and audio are edited in Pecten-Studio and
kept as JSON under `studio/`. The game's behaviour is Rust code in this crate, built
on [agb](https://agbrs.dev) with the `pecten_studio_rt` runtime.

## Who owns what

| Files | Made by | Rule |
| --- | --- | --- |
| `assets/` (PNG, MIDI, SF2, XM, MOD, WAV) | the author, by hand | Read only. Never regenerate, replace, resize or delete. |
| `generated/` (PNG) | tools, for the agent | Made by `tileset_generate`, `pixel_art_convert`. Tools never replace a file here; the author may. |
| `studio/rooms/`, `studio/world.json` | the author, in the studio | Change only what was asked for; leave the rest of a room as it is. |
| `studio/project.json`, `tilesets/`, `sprites/`, `audio.json` | the author, in the studio | Same. |
| `src/` | you and the author | The game's rules. |
| Generated data in `OUT_DIR` (`studio_data.rs`, `studio_gfx.rs`, `studio_audio.rs`) | `build.rs` | Never edit; change the project and rebuild. |

Do not draw art or write music in code to stand in for an asset. If something is
missing, say what is needed and use a placeholder the author can see is one.

## Tools

The studio has an MCP server for this folder. Start it with:

```sh
pecten-studio mcp .
```

(`node <path to pecten-studio>/server/src/cli.ts mcp .` from a checkout.) When the studio
is open on this folder it serves the same tools at `http://127.0.0.1:4517/mcp`;
through that endpoint `studio_view` tells you what the author has on screen and
`studio_open` shows them an item. `pecten-studio agents . --claude` sets Claude Code
up with a skill and the server entry. Begin with
`project_overview`. `render_room` and `render_tileset` return pictures, and
`room_read_cells` returns cells as text. Change rooms with `room_fill`,
`room_paint` and the `entity_` tools rather than by editing JSON: tile layers are
run-length encoded and easy to corrupt. Tilesets, sprites, animations, entity types
and audio have tools of their own (`tileset_set_collision`, `sprite_set_animation`,
`entity_type_set`, `audio_add` and others). `build` and `run_rom` build the game and
return pictures of it running. After a change to level data only (tiles, collision,
entities, properties, room placement, physics), `patch_rom` puts it into the built
ROM in under a second instead of a build; it says when a build is needed. If the
studio is open, it shows your changes as you make them.

Without MCP, these commands print JSON:

```sh
pecten-studio validate . --json     # every problem in the project
pecten-studio build . --json        # run the dev build; add --release for the release one
pecten-studio patch . --json        # put changed level data into the built ROM; exit 3 if it needs a build
pecten-studio schema                # JSON Schema of every document kind
```

Run `validate` after changing anything under `studio/`.

## The format in short

- A screen is 240x160 px: 30x20 tiles of 8 px. A room is a whole number of screens.
- `studio/world.json` places rooms on a grid of screens. Rooms that touch are
  connected: the player walks from one into the other and the camera scrolls along.
- A room has 1 to 3 tile layers, back to front. A cell holds a tile number of the
  room's tileset. In the files, layers are run-length pairs `[count, cell, ...]`
  where cell 0 is empty and cell n is tile n-1.
- A room can have up to 2 backgrounds behind its tile layers, back to front: a PNG
  that repeats in both directions and a parallax in percent of the camera's speed
  (0 stands still, 100 moves with the room). Tile layers and backgrounds together
  are at most 4, the layers the console draws. Set them with `room_update`.
- A room can darken some of its layers so what is in front stands out: `dim` on a
  background or tile layer, and one `dimLevel` for the room, 0 (no change) to 16
  (black). The console has one level for all dimmed layers, and sprites are never
  dimmed. Set it with `room_update` (`dimLevel`, `dimLayers`, and `dim` on a
  background); the pictures themselves are never changed.
- Collision comes from the tileset: each tile has a collision class. A room can
  override single cells.
- An entity is a type, a position in pixels (its feet: the middle of its bottom
  edge), and the properties its type defines. What an entity does is code in `src/`:
  match on the generated `data::EntityKind`.
- Ids are lowercase letters, digits, `-` and `_`, and are the file names.

## In the game code

```rust
pecten_studio_rt::include_data!();   // data::world(), EntityKind, ENTITY_SLOTS, room and class constants
pecten_studio_rt::include_gfx!();    // gfx::ART, TILESETS, BACKGROUNDS, SPRITE_<NAME>, ENTITY_SPRITES
pecten_studio_rt::include_audio!();  // audio::TRACKS, SFX (needs agb_tracker)
```

Rooms, collision, entities, world placement and physics are read from a data
section in the ROM: call `let world = data::world();` once at the start and pass
`&world` around. `pecten_studio_rt::physics::Body` moves a box through the collision
cells of the whole world (`body.step(&world, drop_through)`), and
`WorldView::new(&world, gfx::ART, room, &mut graphics)`
draws the rooms around a camera, with the backgrounds of one room behind them: call
`view.set_backgrounds(room, &graphics)` when the player enters a room. The view
darkens the room's dimmed layers: `view.set_dim(level)` or
`view.fade_dim(level, frames)` change the level while the game runs (to dim the scene
behind a dialogue), and entering a room puts that room's own level back. The data
and physics half of the runtime has no dependencies and runs on the host, so rules
can be unit tested with `cargo test`.

The Physics tab of the studio saves `physics` in `studio/project.json`: the game type
(`platformer` or `topdown`) and how it moves. The game gets it from `world.physics()`.
`physics::PlatformerController` and `physics::TopDownController` apply it, so use
them rather than hard-coding gravity or speeds.
