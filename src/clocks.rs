// Copyright (c) 2012-2013 Andrew Payne
//               2018 Mathias Gottschlag
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use crate::gpio::{self, gpioa};
use crate::time::{Hertz, U32Ext};

use mkl25z4::{MCG, OSC0, PORTA, SIM};

pub struct ClockConfiguration {
    core_clock: Option<Hertz>,
    bus_clock: Option<Hertz>,
    clock_source: ClockSource,
}

impl ClockConfiguration {
    pub fn new() -> Self {
        Self {
            core_clock: None,
            bus_clock: None,
            clock_source: ClockSource::Internal,
        }
    }

    pub fn core_clock<T: Into<Hertz>>(mut self, freq: T) -> Self {
        self.core_clock = Some(freq.into());
        self
    }

    pub fn bus_clock<T: Into<Hertz>>(mut self, freq: T) -> Self {
        self.bus_clock = Some(freq.into());
        self
    }

    /// Use one of the internal RC oscillators as the main clock source.
    ///
    /// Do not select frequencies near the maximum supported by the chip as the resulting frequency
    /// can be up to 20% higher than requested when using the internal reference clock.
    pub fn use_irc(mut self) -> Self {
        self.clock_source = ClockSource::Internal;
        self
    }

    pub fn use_crystal<T: Into<Hertz>>(
        mut self,
        freq: T,
        _extal: gpioa::PA18<gpio::Analog>,
        _xtal: gpioa::PA19<gpio::Analog>,
    ) -> Self {
        // We simply drop the GPIOs to ensure that they are never reconfigured again.
        self.clock_source = ClockSource::External(freq.into(), ExternalMode::Crystal);
        self
    }

    pub fn use_external_clock<T: Into<Hertz>>(
        mut self,
        freq: T,
        _extal: gpioa::PA18<gpio::Analog>,
    ) -> Self {
        // We simply drop the GPIOs to ensure that they are never reconfigured again.
        self.clock_source = ClockSource::External(freq.into(), ExternalMode::Clock);
        self
    }

    pub fn apply(self, sim: &mut SIM, osc: OSC0, mcg: MCG) -> Clocks {
        // Use safe default frequencies if not specified.
        let core_clock = self.core_clock.unwrap_or(24_000_000.hz());
        let bus_clock = self.core_clock.unwrap_or(24_000_000.hz());

        match self.clock_source {
            ClockSource::Internal => {
                // ERCLK32K is provided by the LPO.
                sim.sopt1.modify(|_, w| w.osc32ksel()._11());
            }
            ClockSource::External(_, _) => {
                // ERCLK32K is provided by the system oscillator.
                sim.sopt1.modify(|_, w| w.osc32ksel()._00());
            }
        }

        // Always provide the COP (watchdog) with the LPO clock.
        sim.copc.write(|w| w.copclks().clear_bit());

        // If we use the internal oscillator, the PLL is not in use, so use the FLL output for
        // various peripherals instead.
        match self.clock_source {
            ClockSource::Internal => {
                // MCGFLLCLK
                sim.sopt2.modify(|_, w| w.pllfllsel().clear_bit());
            }
            ClockSource::External(_, _) => {
                // MCGPLLCLK / 2
                sim.sopt2.modify(|_, w| w.pllfllsel().set_bit());
            }
        }
        // Use the PLL or FLL for the TPM counter clock.
        sim.sopt2.modify(|_, w| w.tpmsrc()._01());

        osc.cr.write(|w| {
            w.erclken()
                .clear_bit() // Disable the external reference clock.
                .erefsten()
                .clear_bit() // Disable external reference clock in stop mode.
                .sc2p()
                .clear_bit() // Clear any capacitor load.
                .sc4p()
                .clear_bit()
                .sc8p()
                .clear_bit()
                .sc16p()
                .clear_bit()
        });

        // We assume that we start in FEI mode coming from a reset.

        match self.clock_source {
            ClockSource::Internal => {
                // We want to select a high FLL frequency if the user requested a high core
                // frequency or if lower frequencies are not close to a multiple of it.
                let (outdiv1, range) = match core_clock.0 {
                    0..5800000 => {
                        let outdiv1 = ((640 * 32767) + core_clock.0 / 2) / core_clock.0;
                        (outdiv1 - 1, 0)
                    }
                    5800000..6400000 => (6, 1),   // 5991862 Hz
                    6400000..6700000 => (10, 2),  // 6291456 Hz
                    6700000..7500000 => (2, 0),   // 6990506 Hz
                    7500000..8000000 => (7, 2),   // 7864320 Hz
                    8000000..8700000 => (4, 1),   // 8388608 Hz
                    8700000..9800000 => (6, 2),   // 8987794 Hz
                    9800000..11600000 => (1, 0),  // 10485760 Hz
                    11600000..13300000 => (4, 2), // 12582912 Hz
                    13300000..14900000 => (2, 1), // 15728640 Hz
                    14900000..18400000 => (3, 2), // 15728640 Hz
                    18400000..26300000 => (0, 0), // 20971520 Hz
                    26300000..36800000 => (1, 2), // 31457280 Hz
                    _ => (0, 1),                  // 41943040 Hz
                };

                let actual_core_clock = [640, 1280, 1920, 2560][range] * 32768 / (outdiv1 + 1);
                // TODO: Use actual core clock.
                let outdiv4 = ((actual_core_clock + bus_clock.0 - 1) / bus_clock.0).clamp(1, 8) - 1;
                let actual_bus_clock = actual_core_clock / (outdiv4 + 1);

                // We stay in FEI mode, so we just update the clock dividers and the FLL range.

                sim.clkdiv1.modify(|_, w| {
                    w.outdiv1()
                        .bits(outdiv1 as u8)
                        .outdiv4()
                        .bits(outdiv4 as u8)
                });
                mcg.c4.modify(|_, w| w.drst_drs().bits(range as u8));

                Clocks {
                    coreclk: actual_core_clock.hz(),
                    busclk: actual_bus_clock.hz(),
                }
                /*Clocks {
                    coreclk: core_clock,
                    busclk: bus_clock,
                }*/
            }
            ClockSource::External(freq, mode) => {
                panic!("Not yet implemented.");
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ClockSource {
    Internal,
    External(Hertz, ExternalMode),
}

#[derive(Clone, Copy)]
enum ExternalMode {
    Crystal,
    Clock,
}

// TODO: This function should get the pins and a SIM reference as parameters.
pub fn init() -> Clocks {
    unsafe {
        // Initialize the system clocks to a 48 Mhz core clock speed
        // Mode progression:  FEI (reset) -> FBE -> PBE -> PEE
        //
        // Note:  Generated by Processor Expert, cleaned up by hand.
        //        For detailed information on clock modes, see the
        //        "KL25 Sub-Family Reference Manual" section 24.5.3

        // Enable clock gate to Port A module to enable pin routing (PORTA=1)
        (*SIM::ptr()).scgc5.modify(|_, w| w.porta().set_bit());

        // Divide-by-2 for clock 1 and clock 4 (OUTDIV1=1, OUTDIV4=1)
        (*SIM::ptr())
            .clkdiv1
            .modify(|_, w| w.outdiv1().bits(1).outdiv4().bits(1));

        // System oscillator drives 32 kHz clock for various peripherals (OSC32KSEL=0)
        (*SIM::ptr()).sopt1.modify(|_, w| w.osc32ksel().bits(0));

        // Select PLL as a clock source for various peripherals (PLLFLLSEL=1)
        // Clock source for TPM counter clock is MCGFLLCLK or MCGPLLCLK/2
        (*SIM::ptr()).sopt2.modify(|_, w| w.pllfllsel().set_bit());
        (*SIM::ptr()).sopt2.modify(|_, w| w.tpmsrc().bits(0x1));

        /* PORTA_PCR18: ISF=0,MUX=0 */
        /* PORTA_PCR19: ISF=0,MUX=0 */

        (*PORTA::ptr()).pcr[18].modify(|_, w| w.isf().clear_bit().mux().bits(0x07));
        (*PORTA::ptr()).pcr[19].modify(|_, w| w.isf().clear_bit().mux().bits(0x07));
        /* Switch to FBE Mode */

        /* OSC0_CR: ERCLKEN=0,??=0,EREFSTEN=0,??=0,SC2P=0,SC4P=0,SC8P=0,SC16P=0 */
        (*OSC0::ptr()).cr.write(|w| w.bits(0));
        /* MCG_C2: LOCRE0=0,??=0,RANGE0=2,HGO0=0,EREFS0=1,LP=0,IRCS=0 */
        (*MCG::ptr())
            .c2
            .write(|w| w.bits(0).range0().bits(0x2).erefs0().set_bit());
        /* MCG_C1: CLKS=2,FRDIV=3,IREFS=0,IRCLKEN=0,IREFSTEN=0 */
        (*MCG::ptr())
            .c1
            .write(|w| w.bits(0).clks().bits(0x2).frdiv().bits(0x3));
        /* MCG_C4: DMX32=0,DRST_DRS=0 */
        (*MCG::ptr())
            .c4
            .modify(|_, w| w.dmx32().clear_bit().drst_drs().bits(0));
        /* MCG_C5: ??=0,PLLCLKEN0=0,PLLSTEN0=0,PRDIV0=1 */
        (*MCG::ptr()).c5.write(|w| w.bits(0).prdiv0().bits(0x1));
        /* MCG_C6: LOLIE0=0,PLLS=0,CME0=0,VDIV0=0 */
        (*MCG::ptr()).c6.write(|w| w.bits(0));

        // Check that the source of the FLL reference clock is
        // the external reference clock.
        while (*MCG::ptr()).s.read().irefst().bit_is_set() {}

        while (*MCG::ptr()).s.read().clkst().bits() != 0x2 {
            // Wait until external reference
        }

        // Switch to PBE mode
        //   Select PLL as MCG source (PLLS=1)
        (*MCG::ptr()).c6.write(|w| w.bits(0).plls().set_bit());
        while (*MCG::ptr()).s.read().lock0().bit_is_clear() {
            // Wait until PLL locked
        }

        // Switch to PEE mode
        //    Select PLL output (CLKS=0)
        //    FLL external reference divider (FRDIV=3)
        //    External reference clock for FLL (IREFS=0)
        (*MCG::ptr()).c1.write(|w| w.bits(0).frdiv().bits(0x3));
        while (*MCG::ptr()).s.read().clkst().bits() != 0x3 {
            // Wait until PLL output
        }
    }

    Clocks {
        coreclk: 48000000_u32.hz(),
        busclk: 18000000_u32.hz(), // TODO: Typo?
    }
}

#[derive(Copy, Clone)]
pub struct Clocks {
    // TODO: Rename core/bus.
    coreclk: Hertz,
    busclk: Hertz,
}

impl Clocks {
    pub fn coreclk(&self) -> Hertz {
        self.coreclk
    }

    pub fn busclk(&self) -> Hertz {
        self.busclk
    }
}
