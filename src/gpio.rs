// Taken from stm32f103xx-hal
//
// Copyright (c) 2017-2018 Jorge Aparicio
//               2018 Mathias Gottschlag
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! General Purpose Input / Output

use core::marker::PhantomData;

use mkl25z4::SIM;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self, sim: &mut SIM) -> Self::Parts;
}

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;
/// Pulled down input (type state)
pub struct PullDown;
/// Pulled up input (type state)
pub struct PullUp;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;
/// Open drain output (type state)
pub struct OpenDrain;

/*/// Alternate function
pub struct Alternate<MODE> {
    _mode: PhantomData<MODE>,
}*/

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PORTX:ident, $portx:ident, $PXx:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use core::marker::PhantomData;

            use super::super::hal::digital::{InputPin, OutputPin, StatefulOutputPin, toggleable};
            use mkl25z4::{$PORTX, $GPIOX, $gpiox, SIM};

            use super::{
                /*Alternate, */Floating, GpioExt, Input,
                // OpenDrain,
                Output,
                // PullDown, PullUp,
                PushPull,
            };

            /// GPIO parts
            pub struct Parts {
                pub pddr: PDDR,
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self, sim: &mut SIM) -> Parts {
                    sim.scgc5.modify(|_, w| w.$portx().set_bit());

                    Parts {
                        pddr: PDDR { _0: () },
                        $(
                            $pxi: $PXi { _mode: PhantomData },
                        )+
                    }
                }
            }

            /// Opaque PDDR register
            pub struct PDDR {
                _0: (),
            }

            impl PDDR {
                pub(crate) fn pddr(&mut self) -> &$gpiox::PDDR {
                    unsafe { &(*$GPIOX::ptr()).pddr }
                }
            }

            /// Partially erased pin
            pub struct $PXx<MODE> {
                i: u8,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> OutputPin for $PXx<Output<MODE>> {
                fn set_high(&mut self) {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).psor.write(|w| w.bits(1 << self.i)) }
                }

                fn set_low(&mut self) {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).pcor.write(|w| w.bits(1 << self.i)) }
                }
            }

            impl <MODE> StatefulOutputPin for $PXx<Output<MODE>> {
                fn is_set_high(&self) -> bool {
                    !self.is_set_low()
                }

                fn is_set_low(&self) -> bool {
                    // NOTE(unsafe) atomic read with no side effects
                    unsafe { (*$GPIOX::ptr()).pdor.read().bits() & (1 << self.i) == 0 }
                }
            }

            impl <MODE> toggleable::Default for $PXx<Output<MODE>> {}

            $(
                /// Pin
                pub struct $PXi<MODE> {
                    _mode: PhantomData<MODE>,
                }

                impl<MODE> $PXi<MODE> {
                    /*/// Configures the pin to operate as an alternate function push pull output pin
                    pub fn into_alternate_push_pull(
                        self,
                        pddr: &mut PDDR,
                    ) -> $PXi<Alternate<PushPull>> {
                        let offset = (4 * $i) % 32;
                        // Alternate function output push pull
                        let cnf = 0b10;
                        // Output mode, max speed 50 MHz
                        let mode = 0b11;
                        let bits = (cnf << 2) | mode;

                        // input mode
                        pcr
                            .cr()
                            .modify(|r, w| unsafe {
                                w.bits((r.bits() & !(0b1111 << offset)) | (bits << offset))
                            });

                        $PXi { _mode: PhantomData }
                    }*/

                    /// Configures the pin to operate as a floating input pin
                    pub fn into_floating_input(
                        self,
                        pddr: &mut PDDR,
                    ) -> $PXi<Input<Floating>> {
                        unsafe {
                            // Congigure GPIO as input.
                            pddr.pddr().modify(|r, w| w.pdd().bits(r.pdd().bits() & !(1 << $i)));
                            // Configure pin (no pullup/pulldown configured).
                            (*$PORTX::ptr()).pcr[$i].write(|w| w.bits(0)
                                                           .mux()._001() // GPIO
                                                           .dse().set_bit() // High drive strength
                                                           );
                        }

                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an push pull output pin
                    pub fn into_push_pull_output(
                        self,
                        pddr: &mut PDDR,
                    ) -> $PXi<Output<PushPull>> {
                        unsafe {
                            // Congigure GPIO as output.
                            pddr.pddr().modify(|r, w| w.pdd().bits(r.pdd().bits() | (1 << $i)));
                            // Configure pin.
                            (*$PORTX::ptr()).pcr[$i].write(|w| w.bits(0)
                                                           .mux()._001() // GPIO
                                                           .dse().set_bit() // High drive strength
                                                           );
                        }

                        $PXi { _mode: PhantomData }
                    }
                }

                impl<MODE> $PXi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Output<MODE>> {
                        $PXx {
                            i: $i,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>> {
                    fn set_high(&mut self) {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).psor.write(|w| w.bits(1 << $i)) }
                    }

                    fn set_low(&mut self) {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).pcor.write(|w| w.bits(1 << $i)) }
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>> {
                    fn is_set_high(&self) -> bool {
                        !self.is_set_low()
                    }

                    fn is_set_low(&self) -> bool {
                        // NOTE(unsafe) atomic read with no side effects
                        unsafe { (*$GPIOX::ptr()).pdor.read().bits() & (1 << $i) == 0 }
                    }
                }

                impl <MODE> toggleable::Default for $PXi<Output<MODE>> {}

                impl<MODE> InputPin for $PXi<Input<MODE>> {
                    fn is_high(&self) -> bool {
                        !self.is_low()
                    }

                    fn is_low(&self) -> bool {
                        // NOTE(unsafe) atomic read with no side effects
                        unsafe { (*$GPIOX::ptr()).pdir.read().bits() & (1 << $i) == 0 }
                    }
                }
            )+
        }
    }
}

gpio!(GPIOA, gpioa, PORTA, porta, PAx, [
    PA0: (pa0, 0, Input<Floating>),
    PA1: (pa1, 1, Input<Floating>),
    PA2: (pa2, 2, Input<Floating>),
    PA3: (pa3, 3, Input<Floating>),
    PA4: (pa4, 4, Input<Floating>),
    PA5: (pa5, 5, Input<Floating>),
    PA6: (pa6, 6, Input<Floating>),
    PA7: (pa7, 7, Input<Floating>),
    PA8: (pa8, 8, Input<Floating>),
    PA9: (pa9, 9, Input<Floating>),
    PA10: (pa10, 10, Input<Floating>),
    PA11: (pa11, 11, Input<Floating>),
    PA12: (pa12, 12, Input<Floating>),
    PA13: (pa13, 13, Input<Floating>),
    PA14: (pa14, 14, Input<Floating>),
    PA15: (pa15, 15, Input<Floating>),
    PA16: (pa16, 16, Input<Floating>),
    PA17: (pa17, 17, Input<Floating>),
    PA18: (pa18, 18, Input<Floating>),
    PA19: (pa19, 19, Input<Floating>),
    PA20: (pa20, 20, Input<Floating>),
    PA21: (pa21, 21, Input<Floating>),
    PA22: (pa22, 22, Input<Floating>),
    PA23: (pa23, 23, Input<Floating>),
    PA24: (pa24, 24, Input<Floating>),
    PA25: (pa25, 25, Input<Floating>),
    PA26: (pa26, 26, Input<Floating>),
    PA27: (pa27, 27, Input<Floating>),
    PA28: (pa28, 28, Input<Floating>),
    PA29: (pa29, 29, Input<Floating>),
    PA30: (pa30, 30, Input<Floating>),
    PA31: (pa31, 31, Input<Floating>),
]);

gpio!(GPIOB, gpiob, PORTB, portb, PBx, [
    PB0: (pb0, 0, Input<Floating>),
    PB1: (pb1, 1, Input<Floating>),
    PB2: (pb2, 2, Input<Floating>),
    PB3: (pb3, 3, Input<Floating>),
    PB4: (pb4, 4, Input<Floating>),
    PB5: (pb5, 5, Input<Floating>),
    PB6: (pb6, 6, Input<Floating>),
    PB7: (pb7, 7, Input<Floating>),
    PB8: (pb8, 8, Input<Floating>),
    PB9: (pb9, 9, Input<Floating>),
    PB10: (pb10, 10, Input<Floating>),
    PB11: (pb11, 11, Input<Floating>),
    PB12: (pb12, 12, Input<Floating>),
    PB13: (pb13, 13, Input<Floating>),
    PB14: (pb14, 14, Input<Floating>),
    PB15: (pb15, 15, Input<Floating>),
    PB16: (pb16, 16, Input<Floating>),
    PB17: (pb17, 17, Input<Floating>),
    PB18: (pb18, 18, Input<Floating>),
    PB19: (pb19, 19, Input<Floating>),
    PB20: (pb20, 20, Input<Floating>),
    PB21: (pb21, 21, Input<Floating>),
    PB22: (pb22, 22, Input<Floating>),
    PB23: (pb23, 23, Input<Floating>),
    PB24: (pb24, 24, Input<Floating>),
    PB25: (pb25, 25, Input<Floating>),
    PB26: (pb26, 26, Input<Floating>),
    PB27: (pb27, 27, Input<Floating>),
    PB28: (pb28, 28, Input<Floating>),
    PB29: (pb29, 29, Input<Floating>),
    PB30: (pb30, 30, Input<Floating>),
    PB31: (pb31, 31, Input<Floating>),
]);

gpio!(GPIOC, gpioc, PORTC, portc, PCx, [
    PC0: (pc0, 0, Input<Floating>),
    PC1: (pc1, 1, Input<Floating>),
    PC2: (pc2, 2, Input<Floating>),
    PC3: (pc3, 3, Input<Floating>),
    PC4: (pc4, 4, Input<Floating>),
    PC5: (pc5, 5, Input<Floating>),
    PC6: (pc6, 6, Input<Floating>),
    PC7: (pc7, 7, Input<Floating>),
    PC8: (pc8, 8, Input<Floating>),
    PC9: (pc9, 9, Input<Floating>),
    PC10: (pc10, 10, Input<Floating>),
    PC11: (pc11, 11, Input<Floating>),
    PC12: (pc12, 12, Input<Floating>),
    PC13: (pc13, 13, Input<Floating>),
    PC14: (pc14, 14, Input<Floating>),
    PC15: (pc15, 15, Input<Floating>),
    PC16: (pc16, 16, Input<Floating>),
    PC17: (pc17, 17, Input<Floating>),
    PC18: (pc18, 18, Input<Floating>),
    PC19: (pc19, 19, Input<Floating>),
    PC20: (pc20, 20, Input<Floating>),
    PC21: (pc21, 21, Input<Floating>),
    PC22: (pc22, 22, Input<Floating>),
    PC23: (pc23, 23, Input<Floating>),
    PC24: (pc24, 24, Input<Floating>),
    PC25: (pc25, 25, Input<Floating>),
    PC26: (pc26, 26, Input<Floating>),
    PC27: (pc27, 27, Input<Floating>),
    PC28: (pc28, 28, Input<Floating>),
    PC29: (pc29, 29, Input<Floating>),
    PC30: (pc30, 30, Input<Floating>),
    PC31: (pc31, 31, Input<Floating>),
]);

gpio!(GPIOD, gpiod, PORTD, portd, PDx, [
    PD0: (pd0, 0, Input<Floating>),
    PD1: (pd1, 1, Input<Floating>),
    PD2: (pd2, 2, Input<Floating>),
    PD3: (pd3, 3, Input<Floating>),
    PD4: (pd4, 4, Input<Floating>),
    PD5: (pd5, 5, Input<Floating>),
    PD6: (pd6, 6, Input<Floating>),
    PD7: (pd7, 7, Input<Floating>),
    PD8: (pd8, 8, Input<Floating>),
    PD9: (pd9, 9, Input<Floating>),
    PD10: (pd10, 10, Input<Floating>),
    PD11: (pd11, 11, Input<Floating>),
    PD12: (pd12, 12, Input<Floating>),
    PD13: (pd13, 13, Input<Floating>),
    PD14: (pd14, 14, Input<Floating>),
    PD15: (pd15, 15, Input<Floating>),
    PD16: (pd16, 16, Input<Floating>),
    PD17: (pd17, 17, Input<Floating>),
    PD18: (pd18, 18, Input<Floating>),
    PD19: (pd19, 19, Input<Floating>),
    PD20: (pd20, 20, Input<Floating>),
    PD21: (pd21, 21, Input<Floating>),
    PD22: (pd22, 22, Input<Floating>),
    PD23: (pd23, 23, Input<Floating>),
    PD24: (pd24, 24, Input<Floating>),
    PD25: (pd25, 25, Input<Floating>),
    PD26: (pd26, 26, Input<Floating>),
    PD27: (pd27, 27, Input<Floating>),
    PD28: (pd28, 28, Input<Floating>),
    PD29: (pd29, 29, Input<Floating>),
    PD30: (pd30, 30, Input<Floating>),
    PD31: (pd31, 31, Input<Floating>),
]);

gpio!(GPIOE, gpioe, PORTE, porte, PEx, [
    PE0: (pe0, 0, Input<Floating>),
    PE1: (pe1, 1, Input<Floating>),
    PE2: (pe2, 2, Input<Floating>),
    PE3: (pe3, 3, Input<Floating>),
    PE4: (pe4, 4, Input<Floating>),
    PE5: (pe5, 5, Input<Floating>),
    PE6: (pe6, 6, Input<Floating>),
    PE7: (pe7, 7, Input<Floating>),
    PE8: (pe8, 8, Input<Floating>),
    PE9: (pe9, 9, Input<Floating>),
    PE10: (pe10, 10, Input<Floating>),
    PE11: (pe11, 11, Input<Floating>),
    PE12: (pe12, 12, Input<Floating>),
    PE13: (pe13, 13, Input<Floating>),
    PE14: (pe14, 14, Input<Floating>),
    PE15: (pe15, 15, Input<Floating>),
    PE16: (pe16, 16, Input<Floating>),
    PE17: (pe17, 17, Input<Floating>),
    PE18: (pe18, 18, Input<Floating>),
    PE19: (pe19, 19, Input<Floating>),
    PE20: (pe20, 20, Input<Floating>),
    PE21: (pe21, 21, Input<Floating>),
    PE22: (pe22, 22, Input<Floating>),
    PE23: (pe23, 23, Input<Floating>),
    PE24: (pe24, 24, Input<Floating>),
    PE25: (pe25, 25, Input<Floating>),
    PE26: (pe26, 26, Input<Floating>),
    PE27: (pe27, 27, Input<Floating>),
    PE28: (pe28, 28, Input<Floating>),
    PE29: (pe29, 29, Input<Floating>),
    PE30: (pe30, 30, Input<Floating>),
    PE31: (pe31, 31, Input<Floating>),
]);
