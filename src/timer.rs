use crate::clocks::Clocks;
use crate::hal::timer::{CountDown, Periodic};
use crate::time::{Hertz, MilliSeconds};
use crate::void::Void;

use mkl25z4::{LPTMR0, PIT, SIM};

pub enum Event {
    Update,
}

pub trait TimerInterrupt {
    fn enable_interrupt(&self);
    fn disable_interrupt(&self);
}

pub struct Timer<TIM> {
    tim: TIM,
    clocks: Clocks,
}

impl Timer<LPTMR0> {
    pub fn lptmr0<T>(lptmr: LPTMR0, timeout: T, clocks: Clocks, sim: &mut SIM) -> Self
    where
        T: Into<MilliSeconds>,
    {
        sim.scgc5.modify(|_, w| w.lptmr().set_bit());
        let mut timer = Timer {
            tim: lptmr,
            clocks: clocks,
        };
        timer.start(timeout);
        timer
    }

    pub fn read(&mut self) -> u32 {
        unsafe {
            self.tim.cnr.write(|w| w.bits(0));
            self.tim.cnr.read().bits()
        }
    }
}

impl CountDown for Timer<LPTMR0> {
    type Time = MilliSeconds;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<MilliSeconds>,
    {
        let timeout = timeout.into();

        let ps_ratio = timeout.0 >> 16;
        let prescale = (ps_ratio + 1).next_power_of_two();
        let compare = if ps_ratio == 0 {
            timeout.0
        } else {
            timeout.0 / prescale
        };

        unsafe {
            self.tim.csr.write(|w| w.bits(0)); // Disable the timer.
        }
        self.tim
            .cmr
            .write(|w| unsafe { w.compare().bits(compare as u16) });
        self.tim.psr.write(|w| {
            let w = w.pcs()._01(); // Select the 1kHz LPO as input.
            let w = if ps_ratio == 0 {
                w.pbyp().set_bit() // Bypass the prescaler.
            } else {
                let prescale_bits = prescale.trailing_zeros() - 1;
                w.pbyp().clear_bit().prescale().bits(prescale_bits as u8)
            };
            w
        });
        //self.tim.csr.modify(|_, w| w.tfc().clear_bit()); // Periodic counter
        self.tim.csr.modify(|_, w| w.ten().set_bit());
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.csr.read().tcf().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl TimerInterrupt for Timer<LPTMR0> {
    fn enable_interrupt(&self) {
        self.tim.csr.modify(|_, w| w.tie().set_bit());
    }
    fn disable_interrupt(&self) {
        self.tim.csr.modify(|_, w| w.tie().clear_bit());
    }
    // TODO: Function to acknowledge the interrupt.
}

impl Periodic for Timer<LPTMR0> {}

impl Timer<PIT> {
    pub fn pit<T>(pit: PIT, timeout: T, clocks: Clocks, sim: &mut SIM) -> Self
    where
        T: Into<Hertz>,
    {
        sim.scgc6.modify(|_, w| w.pit().set_bit());
        let mut timer = Timer {
            tim: pit,
            clocks: clocks,
        };
        timer.start(timeout);
        timer
    }
}

impl CountDown for Timer<PIT> {
    type Time = Hertz;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Hertz>,
    {
        unsafe {
            self.tim.mcr.write(|w| w.bits(0));
            let compare = self.clocks.busclk().0 / timeout.into().0;
            self.tim.ldval0.write(|w| w.bits(compare));
            self.tim.tctrl0.write(|w| w.bits(0x0).ten().clear_bit());
            self.tim.tflg0.write(|w| w.bits(1));
            self.tim.tctrl0.write(|w| w.bits(0x0).ten().set_bit());
        }
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.tflg0.read().tif().bit_is_set() {
            unsafe {
                self.tim.tflg0.write(|w| w.bits(1));
            }
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl TimerInterrupt for Timer<PIT> {
    fn enable_interrupt(&self) {
        self.tim.tctrl0.modify(|_, w| w.tie().set_bit());
    }
    fn disable_interrupt(&self) {
        self.tim.tctrl0.modify(|_, w| w.tie().clear_bit());
    }
}
