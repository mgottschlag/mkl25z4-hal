
use super::hal::timer::{CountDown, Periodic};
use super::time::Millisecond;
use super::void::Void;
use mkl25z4::{SIM, LPTMR0};

pub struct Timer<TIM> {
    tim: TIM,
}

impl Timer<LPTMR0> {
    pub fn lptmr0<T>(lptmr: LPTMR0, timeout: T, sim: &mut SIM) -> Self
    where
        T: Into<Millisecond>,
    {
        sim.scgc5.modify(|_, w| w.lptmr().set_bit());
        let mut timer = Timer{
            tim: lptmr,
        };
        timer.start(timeout);
        timer
    }
}

impl CountDown for Timer<LPTMR0> {
    type Time = Millisecond;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Millisecond>,
    {
        unsafe {
            self.tim.csr.write(|w| w.bits(0));
            self.tim.psr.write(|w| w.bits(0));
            self.tim.cmr.write(|w| w.bits(0));
            self.tim.psr.modify(|_, w| w.pcs().bits(0x1).pbyp().set_bit());
            self.tim.cmr.write(|w| w.bits(0x1000/* TODO: Compare value */));
            self.tim.csr.modify(|_, w| w.tfc().set_bit()); // Periodic counter (freerunning)
            self.tim.csr.modify(|_, w| w.ten().set_bit());
            // TODO: Interrupts
        }
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.csr.read().tcf().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl Periodic for Timer<LPTMR0> {}

