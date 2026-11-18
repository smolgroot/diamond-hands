#!/usr/bin/env python3
"""Brings the art exported by `tools/export-art` into the GBA-Studio project.

    cargo run -p export-art -- <dir>
    python3 tools/import-art.py <dir>          # runs the studio's MCP server from ../gba-studio (GBA_STUDIO to move it)

Copies the sprite sheets, palette files and backgrounds into project/assets/ and registers
every sheet as a studio sprite (sharing its palette file with the other sheets that use it).
Files already in project/assets are never overwritten: remove one to bring it in again.
"""
import json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "project"
# The studio's own MCP server, run from its checkout over stdio, so the schema it checks
# against is the checkout's (an already-open studio may hold an older one). Set
# GBA_STUDIO to the checkout, next to this repo by default.
STUDIO = Path(os.environ.get("GBA_STUDIO", ROOT.parent.parent / "gba-studio"))
_server = None


def call(tool, args):
    global _server
    if _server is None:
        _server = subprocess.Popen(["node", str(STUDIO / "server/src/cli.ts"), "mcp", "."], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        rpc("initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "import-art", "version": "1"}})
        _server.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    result = rpc("tools/call", {"name": tool, "arguments": args})
    text = "".join(c.get("text", "") for c in result["content"])
    if result.get("isError"):
        raise SystemExit(f"{tool} failed: {text}")
    return text


def rpc(method, params):
    _server.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}) + "\n")
    _server.stdin.flush()
    while True:
        line = _server.stdout.readline()
        if not line:
            raise SystemExit("the studio's MCP server stopped")
        reply = json.loads(line)
        if reply.get("id") == 1:
            if "error" in reply:
                raise SystemExit(f"{method} failed: {reply['error']}")
            return reply["result"]


def copy_new(src: Path, dst: Path) -> bool:
    if dst.exists():
        print(f"  kept {dst.relative_to(ROOT)} (already there)")
        return False
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)
    return True


def main():
    export = Path(sys.argv[1])
    manifest = json.loads((export / "export.json").read_text())
    for png in sorted((export / "palettes").glob("*.png")):
        copy_new(png, ROOT / "assets/palettes" / png.name)
    for png in sorted((export / "backgrounds").glob("*.png")):
        copy_new(png, ROOT / "assets/backgrounds" / png.name)
    made = 0
    for sheet in manifest["sprites"]:
        name = sheet["name"]
        sprite_path = ROOT / "studio/sprites" / f"{name}.json"
        if sprite_path.exists():
            print(f"  kept studio/sprites/{name}.json (already there)")
            continue
        copy_new(export / "sprites" / f"{name}.png", ROOT / "assets/sprites" / f"{name}.png")
        animations = [
            {"name": field, "frames": [{"index": i, "duration": 8} for i in anim["frames"]], "loop": True}
            for field, anim in sheet["animations"].items()
        ]
        doc = {
            "version": 1,
            "name": name,
            "image": f"assets/sprites/{name}.png",
            "frame": {"width": sheet["width"], "height": sheet["height"]},
            # The art code places sprites by their top-left corner.
            "anchor": {"x": 0, "y": 0},
            "animations": animations,
            "palette": f"assets/palettes/{sheet['palette']}.png",
        }
        call("write_document", {"kind": "sprite", "id": name, "document": doc})
        # The studio keeps what its schema knows: check the palette survived.
        saved = json.loads(call("read_document", {"kind": "sprite", "id": name}))
        if "palette" not in saved:
            raise SystemExit("the studio dropped the sprite's palette: restart it so it loads the new schema")
        made += 1
    print(f"registered {made} sprites; run `validate` in the studio")
    if _server:
        _server.stdin.close()


main()
