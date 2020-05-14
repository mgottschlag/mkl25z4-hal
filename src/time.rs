// Mostly copied from stm32f103xx-hal (C) 2018 Jorge Aparicio

//! Time units

use mkl25z4::{PIT, SIM};

use crate::clocks::Clocks;
use crate::hal::blocking::delay::{DelayMs, DelayUs};

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

/// Seconds
#[derive(Clone, Copy)]
pub struct Seconds(pub u32);

/// Milliseconds
#[derive(Clone, Copy)]
pub struct MilliSeconds(pub u32);

/// Microseconds
#[derive(Clone, Copy)]
pub struct MicroSeconds(pub u32);

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

    /// Wrap in `Seconds`
    fn s(self) -> Seconds;

    /// Wrap in `MilliSeconds`
    fn ms(self) -> MilliSeconds;

    /// Wrap in `MicroSeconds`
    fn us(self) -> MicroSeconds;
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

    fn s(self) -> Seconds {
        Seconds(self)
    }

    fn ms(self) -> MilliSeconds {
        MilliSeconds(self)
    }

    fn us(self) -> MicroSeconds {
        MicroSeconds(self)
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

impl Into<MilliSeconds> for Seconds {
    fn into(self) -> MilliSeconds {
        MilliSeconds(self.0 * 1_000)
    }
}

impl Into<MicroSeconds> for Seconds {
    fn into(self) -> MicroSeconds {
        MicroSeconds(self.0 * 1_000_000)
    }
}

impl Into<MicroSeconds> for MilliSeconds {
    fn into(self) -> MicroSeconds {
        MicroSeconds(self.0 * 1_000)
    }
}

pub trait MonoTimer {
    /// Returns the frequency at which the monotonic timer is operating.
    fn frequency(&self) -> Hertz;

    /// Returns an `Instant` corresponding to "now".
    fn now(&self) -> Instant;

    fn delay_ticks(&self, ticks: u32) {
        let start = self.now();
        // Wait *at least* 'ticks' ticks, so wait for one additional count.
        while start.elapsed(self.now()) <= ticks {}
    }
}

// TODO: Deduplicate the code here and for CopyableMonoTimer.
impl DelayMs<u8> for NonCopyableMonoTimer {
    fn delay_ms(&mut self, ms: u8) {
        let freq = self.frequency();
        // TODO: Check for overflow and employ different strategy?
        //let ticks = ms * freq.0 / 1000;
        for _ in 0..ms {
            self.delay_ticks(freq.0 / 1000);
        }
    }
}

impl DelayUs<u8> for NonCopyableMonoTimer {
    fn delay_us(&mut self, us: u8) {
        // TODO: Most of the code is not needed.
        let freq = self.frequency();
        self.delay_ms(us as u32 / 1000);
        let mut us = us as u32 % 1000;
        while us > 100 {
            self.delay_ticks(freq.0 / 10000);
            us -= 100;
        }
        self.delay_ticks(freq.0 * us as u32 / 1000000);
    }
}

impl DelayMs<u32> for NonCopyableMonoTimer {
    fn delay_ms(&mut self, ms: u32) {
        let freq = self.frequency();
        // TODO: Check for overflow and employ different strategy?
        //let ticks = ms * freq.0 / 1000;
        for _ in 0..ms {
            self.delay_ticks(freq.0 / 1000);
        }
    }
}

impl DelayUs<u32> for NonCopyableMonoTimer {
    fn delay_us(&mut self, us: u32) {
        let freq = self.frequency();
        self.delay_ms(us / 1000);
        let mut us = us % 1000;
        while us > 100 {
            self.delay_ticks(freq.0 / 10000);
            us -= 100;
        }
        self.delay_ticks(freq.0 * us / 1000000);
    }
}

/// A monotonic nondecreasing timer
pub struct NonCopyableMonoTimer {
    pit: PIT,
    frequency: Hertz,
}

impl NonCopyableMonoTimer {
    /// Creates a new `Monotonic` timer
    pub fn new(pit: PIT, clocks: Clocks, sim: &mut SIM) -> Self {
        sim.scgc6.modify(|_, w| w.pit().set_bit());
        unsafe {
            pit.mcr.write(|w| w.bits(0));
            pit.tctrl0.modify(|_, w| w.tie().clear_bit());
            pit.ldval0.write(|w| w.bits(0xffffffff));
            pit.tctrl0.write(|w| w.bits(0x0).ten().set_bit());
        }

        NonCopyableMonoTimer {
            pit: pit,
            frequency: clocks.busclk(),
        }
    }

    pub fn free(self, sim: &mut SIM) -> PIT {
        self.pit.tctrl0.modify(|_, w| w.ten().clear_bit());
        sim.scgc6.modify(|_, w| w.pit().clear_bit());
        self.pit
    }
}

impl MonoTimer for NonCopyableMonoTimer {
    /// Returns the frequency at which the monotonic timer is operating at
    fn frequency(&self) -> Hertz {
        self.frequency
    }

    /// Returns an `Instant` corresponding to "now"
    fn now(&self) -> Instant {
        Instant {
            // The PIT counts from 0xffffffff down to 0.
            now: 0xffffffff_u32.wrapping_sub(self.pit.cval0.read().bits()),
        }
    }
}

/// A monotonic nondecreasing timer - like MonoTimer, but the resources cannot
/// be released, so the type can implement Copy.
#[derive(Clone, Copy)]
pub struct CopyableMonoTimer {
    frequency: Hertz,
}

impl CopyableMonoTimer {
    pub fn new(timer: NonCopyableMonoTimer) -> Self {
        CopyableMonoTimer {
            frequency: timer.frequency,
        }
    }
}

impl MonoTimer for CopyableMonoTimer {
    /// Returns the frequency at which the monotonic timer is operating at
    fn frequency(&self) -> Hertz {
        self.frequency
    }

    /// Returns an `Instant` corresponding to "now"
    fn now(&self) -> Instant {
        Instant {
            // The PIT counts from 0xffffffff down to 0.
            now: 0xffffffff_u32.wrapping_sub(unsafe { (*PIT::ptr()).cval0.read().bits() }),
        }
    }
}

impl DelayMs<u8> for CopyableMonoTimer {
    fn delay_ms(&mut self, ms: u8) {
        let freq = self.frequency();
        // TODO: Check for overflow and employ different strategy?
        //let ticks = ms * freq.0 / 1000;
        for _ in 0..ms {
            self.delay_ticks(freq.0 / 1000);
        }
    }
}

impl DelayUs<u8> for CopyableMonoTimer {
    fn delay_us(&mut self, us: u8) {
        // TODO: Most of the code is not needed.
        let freq = self.frequency();
        self.delay_ms(us as u32 / 1000);
        let mut us = us as u32 % 1000;
        while us > 100 {
            self.delay_ticks(freq.0 / 10000);
            us -= 100;
        }
        self.delay_ticks(freq.0 * us as u32 / 1000000);
    }
}

impl DelayMs<u32> for CopyableMonoTimer {
    fn delay_ms(&mut self, ms: u32) {
        let freq = self.frequency();
        // TODO: Check for overflow and employ different strategy?
        //let ticks = ms * freq.0 / 1000;
        for _ in 0..ms {
            self.delay_ticks(freq.0 / 1000);
        }
    }
}

impl DelayUs<u32> for CopyableMonoTimer {
    fn delay_us(&mut self, us: u32) {
        let freq = self.frequency();
        self.delay_ms(us / 1000);
        let mut us = us % 1000;
        while us > 100 {
            self.delay_ticks(freq.0 / 10000);
            us -= 100;
        }
        self.delay_ticks(freq.0 * us / 1000000);
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
