//! Drives the Diamond Hands ROM headlessly on the gba-wasm core and reads the
//! [`Probe`] the game mirrors into RAM each frame.

use std::path::PathBuf;

use diamond_hands_logic::Probe;
use gba_wasm::GbaCore;

pub use diamond_hands_logic::keys;

/// RAM regions searched for the probe: EWRAM then IWRAM.
const PROBE_SEARCH: [(u32, u32); 2] = [(0x0200_0000, 0x0204_0000), (0x0300_0000, 0x0300_8000)];

/// `DIAMOND_HANDS_ROM`, or `dist/diamond-hands.gba` at the repo root.
pub fn rom_path() -> PathBuf {
    std::env::var_os("DIAMOND_HANDS_ROM")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist/diamond-hands.gba")
        })
}

pub fn read_rom() -> Vec<u8> {
    let path = rom_path();
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read ROM {}: {error}. Build it first with scripts/build-rom.sh",
            path.display()
        )
    })
}

pub struct Runner {
    pub core: GbaCore,
    keys: u16,
    probe_addr: Option<u32>,
    framebuffer: Vec<u8>,
    /// While set, frames are stepped instruction by instruction and their cycles charged
    /// to the program counter (see [`Profile`]).
    pub profile: Option<Profile>,
}

/// Where the ROM's CPU time goes: cycles by program counter, and each profiled video
/// frame's busy cycles. A step that leaves the PC where it was is the CPU halted
/// (waiting for vblank) and counts as idle. Profiled frames aren't drawn.
#[derive(Default)]
pub struct Profile {
    pub by_pc: std::collections::HashMap<u32, u64>,
    /// Busy cycles of each profiled video frame, out of [`CYCLES_PER_FRAME`].
    pub frames: Vec<u32>,
    /// Objects in OAM at the end of each profiled frame (128 means some were dropped).
    pub objects: Vec<usize>,
    /// Address ranges whose callers to find: cycles in each range by the link register.
    /// Only right for leaf functions, which keep LR as the return address.
    pub watch: Vec<std::ops::Range<u32>>,
    pub callers: std::collections::HashMap<(usize, u32), u64>,
}

/// One video frame: 228 lines of 1232 cycles.
pub const CYCLES_PER_FRAME: u32 = 280_896;

impl Runner {
    pub fn new(rom: &[u8]) -> Self {
        Self::with_save(rom, None)
    }

    /// Boots with existing cartridge save data, as if the battery kept it.
    pub fn with_save(rom: &[u8], save: Option<&[u8]>) -> Self {
        let mut core = GbaCore::new();
        core.load_rom(rom).expect("gba-wasm rejected the ROM");
        if let Some(save) = save {
            core.memory
                .backup
                .load(save)
                .expect("save size does not match the cartridge's backup type");
        }
        Self {
            core,
            keys: 0,
            probe_addr: None,
            framebuffer: Vec::new(),
            profile: None,
        }
    }

    /// Runs one video frame with `keys` held.
    pub fn frame(&mut self, keys: u16) {
        self.keys = keys;
        self.core.set_pressed_keys(keys);
        if let Some(profile) = &mut self.profile {
            let start = self.core.memory.vblanks_started();
            let mut busy = 0;
            while self.core.memory.vblanks_started() == start {
                let pc = self.core.cpu.registers[15];
                let cycles = self
                    .core
                    .cpu
                    .step_with_cycles(&mut self.core.memory)
                    .unwrap_or_else(|error| panic!("emulation failed: {error}"));
                if self.core.cpu.registers[15] != pc {
                    busy += cycles;
                    *profile.by_pc.entry(pc).or_default() += u64::from(cycles);
                    if let Some(i) = profile.watch.iter().position(|r| r.contains(&pc)) {
                        let lr = self.core.cpu.registers[14];
                        *profile.callers.entry((i, lr)).or_default() += u64::from(cycles);
                    }
                }
            }
            profile.frames.push(busy);
            let objects = self.objects_shown();
            if let Some(profile) = &mut self.profile {
                profile.objects.push(objects);
            }
            return;
        }
        self.framebuffer = self
            .core
            .step_frame()
            .unwrap_or_else(|error| panic!("emulation failed: {error}"));
    }

    pub fn frames(&mut self, count: usize, keys: u16) {
        for _ in 0..count {
            self.frame(keys);
        }
    }

    /// Presses `keys` for one frame, then releases for one frame.
    pub fn tap(&mut self, keys: u16) {
        self.frame(keys);
        self.frame(0);
    }

    /// Screen boxes `(x, y, w, h)` of the objects in OAM, in OAM order. Double-size
    /// affine objects give their doubled box; y is in 0..256 as the hardware has it.
    pub fn object_boxes(&self) -> Vec<(i32, i32, i32, i32)> {
        const SIZES: [[(i32, i32); 4]; 3] = [
            [(8, 8), (16, 16), (32, 32), (64, 64)],
            [(16, 8), (32, 8), (32, 16), (64, 32)],
            [(8, 16), (8, 32), (16, 32), (32, 64)],
        ];
        self.core
            .memory
            .oam
            .chunks(8)
            .filter_map(|attrs| {
                let attr0 = u16::from_le_bytes([attrs[0], attrs[1]]);
                let attr1 = u16::from_le_bytes([attrs[2], attrs[3]]);
                let (affine, double) = (attr0 & 0x100 != 0, attr0 & 0x200 != 0);
                let shape = usize::from(attr0 >> 14);
                if (!affine && double) || shape == 3 {
                    return None;
                }
                let (w, h) = SIZES[shape][usize::from(attr1 >> 14)];
                let scale = if affine && double { 2 } else { 1 };
                let x = (i32::from(attr1 & 0x1FF) << 23) >> 23;
                Some((x, i32::from(attr0 & 0xFF), w * scale, h * scale))
            })
            .collect()
    }

    /// Whether an object covers screen pixel `(x, y)` (y taken modulo 256).
    pub fn object_covers(&self, x: i32, y: i32) -> bool {
        self.object_boxes()
            .iter()
            .any(|&(ox, oy, w, h)| (ox..ox + w).contains(&x) && (y - oy).rem_euclid(256) < h)
    }

    /// How many of the 128 OAM entries are showing an object (not disabled).
    pub fn objects_shown(&self) -> usize {
        self.core
            .memory
            .oam
            .chunks(8)
            .filter(|attrs| {
                let attr0 = u16::from_le_bytes([attrs[0], attrs[1]]);
                attr0 & 0x300 != 0x200
            })
            .count()
    }

    pub fn keys(&self) -> u16 {
        self.keys
    }

    /// The game's probe, or `None` before the ROM has initialised it.
    pub fn probe(&mut self) -> Option<Probe> {
        if let Some(probe) = self.probe_addr.and_then(|addr| self.read_probe(addr)) {
            return Some(probe);
        }
        self.probe_addr = PROBE_SEARCH.iter().find_map(|&(start, end)| {
            (start..end)
                .step_by(4)
                .find(|&addr| self.read_probe(addr).is_some())
        });
        self.probe_addr.and_then(|addr| self.read_probe(addr))
    }

    /// Like [`Runner::probe`] but fails the test when the probe is missing.
    pub fn expect_probe(&mut self) -> Probe {
        self.probe()
            .expect("probe not found in RAM: is the ROM a Diamond Hands build?")
    }

    fn read_probe(&self, addr: u32) -> Option<Probe> {
        if self.core.memory.read_u32(addr) != Probe::MAGIC {
            return None;
        }
        let words = std::array::from_fn(|i| self.core.memory.read_u32(addr + 4 * i as u32));
        Probe::from_words(words)
    }

    /// Runs frames with `keys` held until `done` holds for the probe, up to `limit` frames.
    pub fn run_until(
        &mut self,
        limit: usize,
        keys: u16,
        mut done: impl FnMut(&Probe) -> bool,
    ) -> Probe {
        for _ in 0..limit {
            self.frame(keys);
            if let Some(probe) = self.probe().filter(|p| done(p)) {
                return probe;
            }
        }
        panic!(
            "condition not reached within {limit} frames; last probe {:?}",
            self.probe()
        );
    }

    /// Cartridge save memory, as the browser UI would persist it.
    pub fn save_data(&self) -> Vec<u8> {
        self.core.memory.backup.data().to_vec()
    }

    /// The last frame as 240x160 RGBA.
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    /// FNV-1a over the last RGBA frame, matching gba-wasm's own screen tests.
    pub fn frame_hash(&self) -> u64 {
        self.framebuffer
            .iter()
            .fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
                (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3)
            })
    }
}
