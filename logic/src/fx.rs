//! Short-lived visual effects: laser impacts, explosions, pickup glints. They are part
//! of the simulation (spawned by what happens in a step, aged each step) so the ROM and
//! the host model agree on them, but nothing in the game reads them back.

pub const MAX_FX: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FxKind {
    /// Diamond-shaped burst where a laser hits something.
    Impact,
    /// A bot blowing apart.
    Explosion,
    /// Sparkle where a diamond or heal was collected.
    Glint,
    /// Sparks off the wassie when something hits it.
    Sparks,
    /// Jetpack exhaust, pink laser energy drifting down as it fades.
    Puff,
    /// A little cloud of dust behind a running heel or ahead of a skid.
    Dust,
    /// Dust kicked out both ways when the wassie jumps or lands hard.
    Kick,
    /// A gold coin flipping up out of a beaten bot, a pickup or the boss's stash.
    CoinPop,
    /// A cucumber bursting on impact.
    Splat,
}

impl FxKind {
    /// Frames the effect lasts.
    pub fn frames(self) -> u8 {
        match self {
            FxKind::Impact => 9,
            FxKind::Explosion => 16,
            FxKind::Glint => 12,
            FxKind::Sparks => 10,
            FxKind::Puff => 14,
            FxKind::Dust => 15,
            FxKind::Kick => 12,
            FxKind::CoinPop => 24,
            FxKind::Splat => 12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fx {
    pub kind: FxKind,
    /// Centre, level pixels.
    pub x: i32,
    pub y: i32,
    /// Frames since it spawned.
    pub age: u8,
    pub active: bool,
}

impl Fx {
    pub const EMPTY: Fx = Fx {
        kind: FxKind::Impact,
        x: 0,
        y: 0,
        age: 0,
        active: false,
    };

    /// Animation frame out of `count`, spread over the effect's life.
    pub fn frame(&self, count: u8) -> usize {
        (self.age as usize * count as usize / self.kind.frames() as usize).min(count as usize - 1)
    }
}

/// A fixed ring of effects: when full, the oldest is replaced.
#[derive(Clone, Debug)]
pub struct FxPool {
    pub items: [Fx; MAX_FX],
    next: usize,
}

impl FxPool {
    pub fn new() -> Self {
        Self {
            items: [Fx::EMPTY; MAX_FX],
            next: 0,
        }
    }

    pub fn spawn(&mut self, kind: FxKind, x: i32, y: i32) {
        let slot = self
            .items
            .iter()
            .position(|f| !f.active)
            .unwrap_or(self.next);
        self.items[slot] = Fx {
            kind,
            x,
            y,
            age: 0,
            active: true,
        };
        self.next = (slot + 1) % MAX_FX;
    }

    pub fn update(&mut self) {
        for f in self.items.iter_mut().filter(|f| f.active) {
            f.age += 1;
            if f.age >= f.kind.frames() {
                f.active = false;
            }
        }
    }

    pub fn clear(&mut self) {
        self.items = [Fx::EMPTY; MAX_FX];
    }

    pub fn active(&self) -> impl Iterator<Item = &Fx> {
        self.items.iter().filter(|f| f.active)
    }
}

impl Default for FxPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_age_out() {
        let mut pool = FxPool::new();
        pool.spawn(FxKind::Impact, 10, 20);
        for _ in 0..FxKind::Impact.frames() - 1 {
            pool.update();
        }
        assert_eq!(pool.active().count(), 1);
        assert_eq!(pool.active().next().unwrap().frame(3), 2);
        pool.update();
        assert_eq!(pool.active().count(), 0);
    }

    #[test]
    fn a_full_pool_replaces_the_oldest() {
        let mut pool = FxPool::new();
        for i in 0..MAX_FX as i32 + 1 {
            pool.spawn(FxKind::Glint, i, 0);
        }
        assert_eq!(pool.active().count(), MAX_FX);
        assert!(pool.active().any(|f| f.x == MAX_FX as i32));
        assert!(!pool.active().any(|f| f.x == 0), "the oldest was replaced");
    }
}
