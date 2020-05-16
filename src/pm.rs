use core::cell::Cell;
use cortex_m::asm;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::SCB;
use mkl25z4::SMC;

// TODO: More sleep modes.

static SLEEP_COUNTERS: Mutex<Cell<[u32; 4]>> = Mutex::new(Cell::new([0; 4]));

pub struct MaximumSleepMode {
    mode: SleepMode,
}

impl MaximumSleepMode {
    pub fn new() -> MaximumSleepMode {
        cortex_m::interrupt::free(|cs| {
            let mut counters = SLEEP_COUNTERS.borrow(cs).get();
            counters[SleepMode::Wait.to_index()] += 1;
            SLEEP_COUNTERS.borrow(cs).set(counters);
        });
        MaximumSleepMode {
            mode: SleepMode::Wait,
        }
    }

    pub fn set(&mut self, mode: SleepMode) {
        cortex_m::interrupt::free(|cs| {
            let mut counters = SLEEP_COUNTERS.borrow(cs).get();
            counters[self.mode.to_index()] -= 1;
            counters[mode.to_index()] += 1;
            SLEEP_COUNTERS.borrow(cs).set(counters);
        });
        self.mode = mode;
    }

    pub fn get(&self) -> SleepMode {
        self.mode
    }

    pub fn enter(scb: &mut SCB, smc: &mut SMC) {
        // Wait is the default mode if no MaximumSleepMode instances exist.
        let mut max_sleep = SleepMode::Wait.to_index();
        cortex_m::interrupt::free(|cs| {
            let counters = SLEEP_COUNTERS.borrow(cs).get();
            // Find the first mode with non-zero MaximumSleepMode instances.
            for i in 0..counters.len() {
                if counters[i] != 0 {
                    max_sleep = i;
                    break;
                }
            }
        });
        SleepMode::from_index(max_sleep).enter(scb, smc);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SleepMode {
    Wait,
    Stop,
    VLPS,
    LLS,
}

impl SleepMode {
    fn from_index(index: usize) -> SleepMode {
        match index {
            0 => SleepMode::Wait,
            1 => SleepMode::Stop,
            2 => SleepMode::VLPS,
            _ => SleepMode::LLS,
        }
    }

    fn to_index(self) -> usize {
        match self {
            SleepMode::Wait => 0,
            SleepMode::Stop => 1,
            SleepMode::VLPS => 2,
            SleepMode::LLS => 3,
        }
    }

    pub fn enter(self, scb: &mut SCB, smc: &mut SMC) {
        match self {
            SleepMode::Wait => {
                scb.clear_sleepdeep();
                asm::wfi();
            }
            SleepMode::Stop => {
                scb.set_sleepdeep();
                asm::wfi();
            }
            SleepMode::VLPS => {
                smc.pmprot.modify(|_, w| w.avlp().set_bit());
                smc.pmctrl.modify(|_, w| w.stopm()._010());
                scb.set_sleepdeep();
                asm::wfi();
                smc.pmctrl.modify(|_, w| w.stopm()._000());
                smc.pmprot.modify(|_, w| w.avlp().clear_bit());
            }
            SleepMode::LLS => {
                smc.pmprot.modify(|_, w| w.alls().set_bit());
                smc.pmctrl.modify(|_, w| w.stopm()._011());
                scb.set_sleepdeep();
                asm::wfi();
                smc.pmctrl.modify(|_, w| w.stopm()._000());
                smc.pmprot.modify(|_, w| w.alls().clear_bit());
            }
        }
    }
}
