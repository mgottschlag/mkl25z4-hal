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

#![no_std]

extern crate cortex_m;
extern crate embedded_hal as hal;
pub extern crate embedded_hal_time as time;
pub extern crate mkl25z4;
extern crate void;

use mkl25z4::{SIM, PORTA, OSC0, MCG};

pub mod timer;
pub mod gpio;

pub fn init_clocks() {
    unsafe {
        // Disable the watchdog timer.
        (*SIM::ptr()).copc.write(|w| w.bits(0));

        // Initialize the system clocks to a 48 Mhz core clock speed
        // Mode progression:  FEI (reset) -> FBE -> PBE -> PEE
        //
        // Note:  Generated by Processor Expert, cleaned up by hand. 
        //        For detailed information on clock modes, see the 
        //        "KL25 Sub-Family Reference Manual" section 24.5.3

        // Enable clock gate to Port A module to enable pin routing (PORTA=1)
        (*SIM::ptr()).scgc5.modify(|_, w| w.porta().set_bit());

        // Divide-by-2 for clock 1 and clock 4 (OUTDIV1=1, OUTDIV4=1)   
        (*SIM::ptr()).clkdiv1.modify(|_, w| w.outdiv1().bits(1).outdiv4().bits(1));

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
        (*MCG::ptr()).c2.write(|w| w.bits(0).range0().bits(0x2).erefs0().set_bit());
        /* MCG_C1: CLKS=2,FRDIV=3,IREFS=0,IRCLKEN=0,IREFSTEN=0 */
        (*MCG::ptr()).c1.write(|w| w.bits(0).clks().bits(0x2).frdiv().bits(0x3));
        /* MCG_C4: DMX32=0,DRST_DRS=0 */
        (*MCG::ptr()).c4.modify(|_, w| w.dmx32().clear_bit().drst_drs().bits(0));
        /* MCG_C5: ??=0,PLLCLKEN0=0,PLLSTEN0=0,PRDIV0=1 */
        (*MCG::ptr()).c5.write(|w| w.bits(0).prdiv0().bits(0x1));
        /* MCG_C6: LOLIE0=0,PLLS=0,CME0=0,VDIV0=0 */
        (*MCG::ptr()).c6.write(|w| w.bits(0));

        // Check that the source of the FLL reference clock is 
        // the external reference clock.
        while (*MCG::ptr()).s.read().irefst().bit_is_set() {
        }

        while (*MCG::ptr()).s.read().clkst().bits() != 0x2 { // Wait until external reference
        }

        // Switch to PBE mode
        //   Select PLL as MCG source (PLLS=1)
        (*MCG::ptr()).c6.write(|w| w.bits(0).plls().set_bit());
        while (*MCG::ptr()).s.read().lock0().bit_is_clear() { // Wait until PLL locked
        }

        // Switch to PEE mode
        //    Select PLL output (CLKS=0)
        //    FLL external reference divider (FRDIV=3)
        //    External reference clock for FLL (IREFS=0)
        (*MCG::ptr()).c1.write(|w| w.bits(0).frdiv().bits(0x3));
        while (*MCG::ptr()).s.read().clkst().bits() != 0x3 { // Wait until PLL output
        }
    }
}
