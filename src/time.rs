// Mostly copied from stm32f103xx-hal (C) 2018 Jorge Aparicio

//! Time units

use mkl25z4::{PIT, SIM};

use crate::clocks::Clocks;

/// Bits per second
#[derive(Clone, Copy)]
pub struct Bps(pub u32);

/// Hertz
#[derive(Clone, Copy)]
pub struct Hertz(pub u32);

/// KiloHertz
#[derive(Clone, Copy)]
pub struct KiloHertz(pub u32);

/// MegaHertz
#[derive(Clone, Copy)]
pub struct MegaHertz(pub u32);

/// Extension trait that adds convenience methods to the `u32` type
pub trait U32Ext {
    /// Wrap in `Bps`
    fn bps(self) -> Bps;

    /// Wrap in `Hertz`
    fn hz(self) -> Hertz;

    /// Wrap in `KiloHertz`
    fn khz(self) -> KiloHertz;

    /// Wrap in `MegaHertz`
    fn mhz(self) -> MegaHertz;
}

impl U32Ext for u32 {
    fn bps(self) -> Bps {
        Bps(self)
    }

    fn hz(self) -> Hertz {
        Hertz(self)
    }

    fn khz(self) -> KiloHertz {
        KiloHertz(self)
    }

    fn mhz(self) -> MegaHertz {
        MegaHertz(self)
    }
}

impl Into<Hertz> for KiloHertz {
    fn into(self) -> Hertz {
        Hertz(self.0 * 1_000)
    }
}

impl Into<Hertz> for MegaHertz {
    fn into(self) -> Hertz {
        Hertz(self.0 * 1_000_000)
    }
}

impl Into<KiloHertz> for MegaHertz {
    fn into(self) -> KiloHertz {
        KiloHertz(self.0 * 1_000)
    }
}

/// A monotonic nondecreasing timer
pub struct MonoTimer {
    pit: PIT,
    frequency: Hertz,
}

impl MonoTimer {
    /// Creates a new `Monotonic` timer
    pub fn new(pit: PIT, clocks: Clocks, sim: &mut SIM) -> Self {
        sim.scgc6.modify(|_, w| w.pit().set_bit());
        unsafe {
            pit.mcr.write(|w| w.bits(0));
            pit.tctrl0.modify(|_, w| w.tie().clear_bit());
            pit.ldval0.write(|w| w.bits(0xffffffff));
            pit.tctrl0.write(|w| w.bits(0x0).ten().set_bit());
        }

        MonoTimer {
            pit: pit,
            frequency: clocks.busclk(),
        }
    }

    /// Returns the frequency at which the monotonic timer is operating at
    pub fn frequency(&self) -> Hertz {
        self.frequency
    }

    /// Returns an `Instant` corresponding to "now"
    pub fn now(&self) -> Instant {
        Instant {
            // The PIT counts from 0xffffffff down to 0.
            now: 0xffffffff_u32.wrapping_sub(self.pit.cval0.read().bits()),
        }
    }

    pub fn delay_ticks(&self, ticks: u32) {
        let start = self.now();
        // Wait *at least* 'ticks' ticks, so wait for one additional count.
        while start.elapsed(self.now()) <= ticks {}
    }

    pub fn delay_ms(&self, ms: u32) {
        // TODO: Check for overflow and employ different strategy?
        //let ticks = ms * self.frequency.0 / 1000;
        for _ in 0..ms {
            self.delay_ticks(self.frequency.0 / 1000);
        }
    }

    pub fn deactivate(self, sim: &mut SIM) -> PIT {
        self.pit.tctrl0.modify(|_, w| w.ten().clear_bit());
        sim.scgc6.modify(|_, w| w.pit().clear_bit());
        self.pit
    }
}

/// A measurement of a monotonically nondecreasing clock
#[derive(Clone, Copy)]
pub struct Instant {
    now: u32,
}

impl Instant {
    /// Ticks elapsed since the `Instant` was created
    pub fn elapsed(&self, other: Instant) -> u32 {
        other.now.wrapping_sub(self.now)
    }
}
