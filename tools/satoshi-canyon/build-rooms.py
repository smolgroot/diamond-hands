#!/usr/bin/env python3
"""Lays out Satoshi Canyon's four rooms and saves them through GBA-Studio's MCP server.

A first pass, made on 2026-11-26 for the author to rework in the studio: running it
again replaces the four rooms below, so once they have been edited by hand, edit them
in the studio instead. Each room is drawn here in blocks of 16 px (2x2 tiles), the
size of the blocks `tileset_generate` cuts, and autotiled with the generator's own
block layout (gba-studio core/src/tilegen.ts: LAYOUT and autotile).

- btc-canyon (5x2 screens, tileset satoshi-canyon): the far-west canyon. Mesas and
  buttes to climb, a rope bridge high over the canyon floor and a rock arch, with
  drones, sentry bots and Bad Cop bots.
- btc-town (4x2, satoshi-town): a small western town on three levels: the street,
  the bank's hill, and the jail on a rock shelf, where Bad Cop bots hold the Bitcoin
  Puppet behind adobe bricks. Freed, it opens the gate to the mine under the shelf.
- satoshi-mine (3x3, satoshi-mine): down through the Satoshi mines: the adit, a shaft,
  a gallery west and a deeper one back east, through ore blocks, under stalactites
  that drop, past runaway mine carts.
- inscriber-vault (1x1, satoshi-mine): THE INSCRIBER's vault, the boss arena.

Map characters, one a block: `#` rock, `=` a one-way plank, `/` and `\\` slopes, `t`
tuft, `f` flower, `r` rock, `L` torch or lantern, `S` stalactite (scenery), `.` air.

    python3 tools/satoshi-canyon/build-rooms.py   # with the studio open (its MCP endpoint)

STUDIO_PORT picks the studio's port (default 4519).
"""
import json
import os
import urllib.request

PORT = os.environ.get("STUDIO_PORT", "4519")
URL = f"http://127.0.0.1:{PORT}/mcp"

# The generator's sheet: 8 blocks a row, 5 rows (tilegen.ts LAYOUT).
LAYOUT = [
    ["tl", "t", "tr", "pillar-top", "inner-tl", "inner-tr", "single", "t-alt"],
    ["l", "c", "r", "pillar", "inner-bl", "inner-br", "c-alt", "c-alt2"],
    ["bl", "b", "br", "pillar-bottom", "platform-l", "platform", "platform-r", "spikes"],
    ["tuft", "flower", "rock", "torch", "back", "back-alt", "window", "back-shadow"],
    ["slope-up", "slope-down", "gentle-up-1", "gentle-up-2", "gentle-down-1", "gentle-down-2", "stalactite", "vine"],
]
BLOCK = {name: (col, row) for row, names in enumerate(LAYOUT) for col, name in enumerate(names)}
DECO = {"t": "tuft", "f": "flower", "r": "rock", "L": "torch", "S": "stalactite"}
SHEET_TILES = 16  # tiles across the tileset picture


def rpc(name, args):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}}).encode()
    req = urllib.request.Request(URL, data=body, headers={"Content-Type": "application/json", "Accept": "application/json, text/event-stream"})
    with urllib.request.urlopen(req, timeout=120) as r:
        text = r.read().decode()
    if "data:" in text:
        text = [line[5:] for line in text.splitlines() if line.startswith("data:")][-1]
    res = json.loads(text)
    if "error" in res:
        raise SystemExit(f"{name}: {res['error']}")
    out = "\n".join(c["text"] for c in res["result"].get("content", []) if c["type"] == "text")
    if res["result"].get("isError"):
        raise SystemExit(f"{name}: {out}")
    return out


def hash01(x, y, seed):
    h = (x * 374761393 + y * 668265263 + seed * 1442695041) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    h ^= h >> 16
    return h / 4294967296


class Grid:
    """A room in blocks: the front map, the back wall, and the entities."""

    def __init__(self, w, h, fill="."):
        self.w, self.h = w, h
        self.cells = [[fill] * w for _ in range(h)]
        self.back = [[None] * w for _ in range(h)]
        self.entities = []

    def fill(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.cells[y][x] = c

    def put(self, x, y, c):
        self.cells[y][x] = c

    def wall(self, x0, y0, x1, y1, windows=None):
        """Back wall over a rectangle; `windows` = (every x, every y, x offset, y offset)."""
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                name = "back-alt" if hash01(x, y, 7) < 0.3 else "back"
                if windows:
                    ex, ey, ox, oy = windows
                    if (x - x0 - ox) % ex == 0 and (y - y0 - oy) % ey == 0 and y < y1:
                        name = "window"
                self.back[y][x] = name

    def shadow(self, x0, x1, y):
        for x in range(x0, x1 + 1):
            if self.back[y][x]:
                self.back[y][x] = "back-shadow"

    def add(self, kind, bx, by, **props):
        """An entity standing in block (bx, by): feet on the bottom of that block row."""
        self.entities.append({"type": kind, "x": int(bx * 16 + 8), "y": int((by + 1) * 16), "props": props})

    def hang(self, kind, bx, by):
        """An entity hanging from the ceiling over block (bx, by): its marker in the
        block's top tile row."""
        self.entities.append({"type": kind, "x": int(bx * 16 + 8), "y": int(by * 16 + 8), "props": {}})

    def solid(self, x, y):
        if y >= self.h:
            return True
        if y < 0:
            return False
        x = max(0, min(self.w - 1, x))
        return self.cells[y][x] == "#"

    def blocks(self, variety=5):
        """Block names for the front layer, autotiled as the generator does."""
        out = [[None] * self.w for _ in range(self.h)]
        for y in range(self.h):
            for x in range(self.w):
                c = self.cells[y][x]
                if c == "#":
                    t, b = not self.solid(x, y - 1), not self.solid(x, y + 1)
                    l, r = not self.solid(x - 1, y), not self.solid(x + 1, y)
                    if l and r:
                        name = "single" if t and b else "pillar-top" if t else "pillar-bottom" if b else "pillar"
                    elif t and b:
                        name = "platform-l" if l else "platform-r" if r else "platform"
                        # A one-block-thick rock ledge stays rock: use the top edges.
                        name = {"platform-l": "tl", "platform": "t", "platform-r": "tr"}[name]
                    elif t:
                        name = "tl" if l else "tr" if r else ("t-alt" if hash01(x, y, variety) < 0.25 else "t")
                    elif b:
                        name = "bl" if l else "br" if r else "b"
                    elif l:
                        name = "l"
                    elif r:
                        name = "r"
                    elif not self.solid(x - 1, y - 1):
                        name = "inner-tl"
                    elif not self.solid(x + 1, y - 1):
                        name = "inner-tr"
                    elif not self.solid(x - 1, y + 1):
                        name = "inner-bl"
                    elif not self.solid(x + 1, y + 1):
                        name = "inner-br"
                    else:
                        roll = hash01(x, y, variety + 1)
                        name = "c-alt" if roll < 0.15 else "c-alt2" if roll < 0.3 else "c"
                elif c == "=":
                    left = x > 0 and self.cells[y][x - 1] == "="
                    right = x < self.w - 1 and self.cells[y][x + 1] == "="
                    name = "platform" if left == right else "platform-l" if right else "platform-r"
                elif c == "/":
                    name = "slope-up"
                elif c == "\\":
                    name = "slope-down"
                elif c in DECO:
                    name = DECO[c]
                else:
                    name = None
                out[y][x] = name
        return out

    def layer(self, names):
        """Tile cells (0 empty, n = tile n-1) for a grid of block names."""
        cols, rows = self.w * 2, self.h * 2
        cells = [0] * (cols * rows)
        for by in range(self.h):
            for bx in range(self.w):
                name = names[by][bx]
                if not name:
                    continue
                c, r = BLOCK[name]
                for ty in range(2):
                    for tx in range(2):
                        tile = (r * 2 + ty) * SHEET_TILES + c * 2 + tx
                        cells[(by * 2 + ty) * cols + bx * 2 + tx] = tile + 1
        return cells


def rle(cells):
    out = []
    run, value = 0, None
    for c in cells:
        if c == value:
            run += 1
        else:
            if value is not None:
                out += [run, value]
            run, value = 1, c
    out += [run, value]
    return out


def save(room_id, name, grid, tileset, backgrounds, dim, screens):
    doc = {
        "version": 1,
        "name": name,
        "width": screens[0],
        "height": screens[1],
        "tileset": tileset,
        "layers": [
            {"name": "Back wall", "dim": True, "data": rle(grid.layer(grid.back))},
            {"name": "Ground", "data": rle(grid.layer(grid.blocks()))},
        ],
        "backgrounds": backgrounds,
        "entities": grid.entities,
        "dimLevel": dim,
    }
    print(rpc("write_document", {"kind": "room", "id": room_id, "document": doc}))


SKY = [
    {"image": "generated/backgrounds/satoshi-sky.png", "parallax": {"x": 0, "y": 0}},
    {"image": "generated/backgrounds/satoshi-mesas.png", "parallax": {"x": 20, "y": 0}},
]
CAVE = [{"image": "generated/backgrounds/satoshi-mine-back.png", "parallax": {"x": 30, "y": 30}}]


def canyon():
    """5x2 screens, 75x20 blocks. The floor's top is row 17."""
    g = Grid(75, 20)
    g.fill(0, 17, 74, 19, "#")
    g.fill(0, 0, 1, 16, "#")  # the mesa face the wassie starts against
    # 1. The trailhead: a rock step and a ledge over it.
    g.fill(10, 15, 14, 16, "#")
    g.fill(6, 12, 9, 12, "=")
    # 2. Up two slopes onto mesa A, with a slab of rock floating over it.
    g.fill(15, 15, 16, 16, "#")
    g.put(15, 14, "/")
    g.fill(16, 14, 16, 14, "#")
    g.put(16, 13, "/")
    g.fill(17, 13, 24, 16, "#")
    g.fill(19, 10, 22, 10, "#")
    # Down two slopes into the wash.
    g.fill(25, 14, 25, 16, "#")
    g.put(25, 13, "\\")
    g.fill(26, 15, 26, 16, "#")
    g.put(26, 14, "\\")
    g.fill(27, 15, 31, 16, "#")
    # 3. The butte, climbed in steps of two.
    g.fill(32, 13, 33, 16, "#")
    g.fill(34, 11, 35, 16, "#")
    g.fill(36, 9, 40, 16, "#")
    # 4. The rope bridge off the butte, high over the canyon floor, to a rock arch.
    g.fill(41, 9, 52, 9, "=")
    g.fill(53, 9, 56, 12, "#")
    # Down off the arch onto mesa C, and steps back down to the floor.
    g.fill(56, 15, 56, 16, "#")
    g.fill(57, 13, 61, 16, "#")
    g.fill(62, 15, 63, 16, "#")
    # Under the arch, the canyon wall behind.
    g.wall(53, 13, 56, 16)
    g.shadow(53, 56, 13)
    # Scenery: tufts, flowering cacti and rocks along the tops.
    for x, y, c in [(3, 16, "t"), (8, 16, "r"), (12, 14, "f"), (18, 12, "t"), (23, 12, "f"), (29, 14, "t"), (37, 8, "f"),
                    (44, 16, "t"), (48, 16, "r"), (51, 16, "f"), (59, 12, "t"), (66, 16, "f"), (70, 16, "t"), (72, 16, "r")]:
        g.put(x, y, c)
    # The way through.
    g.add("player-start", 4, 16)
    g.add("checkpoint", 6, 16)
    for x in (7, 8, 9):
        g.add("coin", x, 15)
    g.add("diamond", 7, 11)
    g.add("diamond", 8, 11)
    g.add("drone", 12, 8)
    g.add("sentry", 21, 12)
    g.add("diamond", 20, 9)
    g.add("diamond", 21, 9)
    g.add("emerald", 22, 9)
    g.add("bad-cop", 29, 14)
    for x in (27, 28, 30):
        g.add("coin", x, 14)
    g.add("diamond", 33, 12)
    g.add("diamond", 35, 10)
    g.add("checkpoint", 38, 8)
    g.add("drone", 46, 5)
    for x in (43, 45, 47, 49, 51):
        g.add("diamond", x, 8)
    g.add("heal", 54, 8)
    g.add("drone", 50, 12)
    g.add("bad-cop", 46, 16)
    g.add("sentry", 52, 16)
    for x in (42, 44, 50, 54, 55):
        g.add("coin", x, 15)
    g.add("emerald", 59, 12)
    g.add("bad-cop", 68, 16)
    g.add("drone", 66, 9)
    g.add("checkpoint", 64, 16)
    for x in (69, 71, 73):
        g.add("coin", x, 15)
    g.add("diamond", 72, 14)
    return g


def town():
    """4x2 screens, 60x20 blocks: the street (top row 17), the bank's hill (13) and the
    jail's shelf (9)."""
    g = Grid(60, 20)
    g.fill(0, 17, 59, 19, "#")
    # The saloon: a step up to its balcony, and its roof.
    g.wall(1, 10, 11, 16, windows=(3, 3, 1, 1))
    g.fill(0, 15, 0, 15, "=")
    g.fill(0, 12, 0, 12, "=")  # a hitching rail, up to the roof
    g.fill(1, 13, 13, 13, "=")
    g.fill(1, 10, 12, 10, "#")
    g.shadow(1, 11, 14)
    # The general store, its awning and roof, and the water tower beside it.
    g.wall(15, 12, 23, 16, windows=(4, 2, 1, 0))
    g.fill(14, 15, 24, 15, "=")
    g.fill(15, 12, 23, 12, "#")
    g.fill(25, 10, 28, 10, "=")
    # Up the slopes onto the bank's hill.
    g.fill(30, 16, 30, 16, "#")
    g.put(30, 16, "/")
    g.fill(31, 16, 31, 16, "#")
    g.put(31, 15, "/")
    g.fill(32, 15, 32, 16, "#")
    g.put(32, 14, "/")
    g.fill(33, 14, 33, 16, "#")
    g.put(33, 13, "/")
    g.fill(34, 13, 43, 16, "#")
    g.wall(35, 7, 42, 12, windows=(3, 3, 1, 1))
    # A plank up off the hill, then the jail's shelf over the lane to the mine.
    g.fill(40, 11, 42, 11, "=")
    g.fill(44, 9, 59, 10, "#")
    g.fill(57, 0, 59, 8, "#")  # the cliff behind the jail
    # The jail: its front wall of adobe bricks, a roof and the cell behind.
    g.wall(49, 3, 56, 8, windows=(5, 3, 1, 1))
    g.fill(51, 5, 56, 5, "#")
    g.wall(53, 6, 56, 8)
    # The lane under the shelf: a plank back up to the hill, and the mine's lintel.
    g.fill(44, 15, 45, 15, "=")
    g.fill(59, 11, 59, 13, "#")
    g.wall(46, 11, 58, 16)
    g.shadow(46, 58, 11)
    for x, y, c in [(13, 16, "t"), (24, 16, "r"), (29, 16, "t"), (36, 12, "f"), (47, 8, "t"), (48, 16, "L"), (55, 16, "L")]:
        g.put(x, y, c)
    # Bricks: a column of three in front of the cell.
    for y in (6, 7, 8):
        g.add("brick-block", 52, y)
    g.add("btc-puppet", 54, 8, talk=PUPPET_TALK)
    # The Bad Cops: on the street, the hill, and two guarding the jail.
    g.add("bad-cop", 8, 16)
    g.add("bad-cop", 20, 16)
    g.add("bad-cop", 38, 12)
    g.add("bad-cop", 46, 8)
    g.add("bad-cop", 49, 8)
    g.add("drone", 26, 6)
    g.add("checkpoint", 3, 16)
    g.add("checkpoint", 35, 12)
    for x in (3, 5, 7, 9):
        g.add("coin", x, 12)
    for x in (4, 7, 10):
        g.add("diamond", x, 9)
    for x in (17, 19, 21):
        g.add("coin", x, 14)
    g.add("diamond", 26, 9)
    g.add("diamond", 27, 9)
    g.add("emerald", 18, 11)
    for x in (37, 39, 41):
        g.add("coin", x, 12)
    g.add("heal", 41, 10)
    g.add("emerald", 45, 8)
    for x in (47, 50, 53, 56):
        g.add("coin", x, 16)
    g.add("power-gate", 58, 16)
    return g


PUPPET_TALK = "\n".join(["OH MY GOD! THE BAD", "COPS ARE DOWN!"]) + "|" + "\n".join(["THEY LOCKED ME UP FOR", "HODLING. CAN YOU", "IMAGINE?"]) + "|>" + "\n".join(["WHO ARE YOU?"]) + "|" + "\n".join(["A BITCOIN PUPPET.", "THE INSCRIBER RUNS", "THE SATOSHI MINES."]) + "|" + "\n".join(["HE INSCRIBES EVERY SAT", "HE DIGS UP. I OPENED", "THE MINE GATE FOR YOU."]) + "|" + "\n".join(["TAKE THESE COINS.", "STAY HUMBLE, STACK", "SATS!"])


def mine():
    """3x3 screens, 45x30 blocks: the adit (floor row 7), the middle gallery (17) and
    the deep gallery (28), which leads into the vault."""
    g = Grid(45, 30, "#")
    # The adit, from the town's lane.
    g.fill(0, 4, 28, 6, ".")
    g.fill(16, 1, 28, 3, ".")
    g.fill(18, 4, 21, 4, "=")
    # The shaft down, with planks up its far side to climb back.
    g.fill(29, 4, 32, 16, ".")
    for y in (9, 12, 15):
        g.fill(31, y, 32, y, "=")
    # The middle gallery, west.
    g.fill(2, 13, 32, 16, ".")
    g.fill(8, 8, 16, 12, ".")
    g.fill(10, 14, 13, 14, "=")
    g.fill(12, 11, 15, 11, "=")
    # The hole down at its west end.
    g.fill(2, 17, 3, 23, ".")
    # The deep gallery, east, with a cave over its middle and a rise to cross.
    g.fill(2, 24, 43, 27, ".")
    g.fill(10, 20, 18, 23, ".")
    g.fill(11, 25, 13, 25, "=")
    g.fill(14, 22, 17, 22, "=")
    g.fill(24, 26, 29, 27, "#")
    # Into the vault: the doorway at the east edge.
    g.fill(44, 25, 44, 27, ".")
    # No back wall: the tunnels open onto the backdrop's timber frames and lanterns.
    for x, y, c in [(4, 4, "L"), (15, 4, "L"), (25, 6, "L"), (6, 16, "L"), (20, 16, "L"), (8, 27, "L"), (33, 27, "L"), (40, 27, "L"),
                    (5, 13, "S"), (28, 13, "S"), (21, 24, "S"), (36, 24, "S"), (3, 6, "r"), (17, 16, "r"), (31, 27, "r")]:
        g.put(x, y, c)
    # The adit: a beacon, ore blocks across the tunnel, stalactites, a cart.
    g.add("checkpoint", 2, 6)
    for y in (4, 5, 6):
        g.add("ore-block", 10, y)
    g.hang("stalactite", 7, 4)
    g.hang("stalactite", 14, 4)
    g.hang("stalactite", 23, 1)
    g.add("mine-cart", 27, 6)
    for x in (4, 6, 12, 16, 20, 24):
        g.add("coin", x, 5)
    for x in (18, 19, 20, 21):
        g.add("diamond", x, 3)
    g.add("diamond", 32, 8)
    g.add("diamond", 32, 11)
    # The middle gallery: a beacon at the foot of the shaft, a cart, a sentry, ore.
    g.add("checkpoint", 30, 16)
    g.add("heal", 28, 15)
    g.add("mine-cart", 6, 16)
    g.add("sentry", 12, 16)
    g.hang("stalactite", 22, 13)
    g.hang("stalactite", 25, 13)
    for y in (13, 14, 15, 16):
        g.add("ore-block", 18, y)
    g.add("drone", 24, 14)
    for x in (10, 11, 12, 13):
        g.add("diamond", x, 13)
    g.add("emerald", 14, 10)
    for x in (5, 9, 21, 25):
        g.add("coin", x, 15)
    # The deep gallery: carts, stalactites, a cop, ore, and a beacon by the vault.
    g.add("emerald", 4, 26)
    g.hang("stalactite", 8, 24)
    for y in (24, 25, 26, 27):
        g.add("ore-block", 19, y)
    for x in (14, 15, 16, 17):
        g.add("diamond", x, 21)
    g.add("diamond", 12, 24)
    g.add("coin", 11, 26)
    g.add("coin", 17, 26)
    g.add("bad-cop", 27, 24 + 1)
    g.hang("stalactite", 27, 24)
    g.add("mine-cart", 38, 27)
    g.hang("stalactite", 33, 24)
    for x in (31, 33, 35):
        g.add("coin", x, 26)
    g.add("emerald", 36, 26)
    g.add("checkpoint", 42, 27)
    return g


def vault():
    """1 screen, 15x10 blocks: THE INSCRIBER's vault, its treasure on two shelves."""
    g = Grid(15, 10, "#")
    g.fill(1, 2, 13, 7, ".")
    g.fill(0, 5, 0, 7, ".")
    g.fill(1, 1, 12, 1, ".")
    g.fill(1, 5, 3, 5, "=")
    g.fill(11, 5, 13, 5, "=")
    for x, y, c in [(5, 7, "f"), (7, 7, "r"), (9, 7, "f"), (4, 2, "S"), (10, 2, "S")]:
        g.put(x, y, c)
    for x in (1, 2, 3, 11, 12, 13):
        g.add("coin", x, 4)
    g.add("diamond", 2, 3)
    g.add("diamond", 12, 3)
    g.entities.append({"type": "arena-edge", "x": 4, "y": 128, "props": {}})
    g.entities.append({"type": "inscriber", "x": 152, "y": 128, "props": {}})
    return g


def main():
    save("btc-canyon", "Satoshi Canyon", canyon(), "satoshi-canyon", SKY, 4, (5, 2))
    save("btc-town", "Satoshi town", town(), "satoshi-town", SKY, 4, (4, 2))
    save("satoshi-mine", "Satoshi mines", mine(), "satoshi-mine", CAVE, 6, (3, 3))
    save("inscriber-vault", "Inscriber's vault (boss arena)", vault(), "satoshi-mine", CAVE, 6, (1, 1))
    places = [("btc-canyon", 0, 19, 5, 2), ("btc-town", 5, 19, 4, 2), ("satoshi-mine", 9, 20, 3, 3), ("inscriber-vault", 12, 22, 1, 1)]
    # Grow the world map to hold them, below Ethereum City with a row of screens between.
    world = json.loads(rpc("read_document", {"kind": "world"}))
    world["size"]["width"] = max(world["size"]["width"], max(x + w for _, x, _, w, _ in places))
    world["size"]["height"] = max(world["size"]["height"], max(y + h for _, _, y, _, h in places))
    print(rpc("write_document", {"kind": "world", "document": world}))
    for room, x, y, _, _ in places:
        print(rpc("world_place", {"room": room, "x": x, "y": y}))


if __name__ == "__main__":
    main()
