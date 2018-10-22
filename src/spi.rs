use crate::hal;
pub use crate::hal::spi::{Mode, Phase, Polarity};
use nb;
use mkl25z4::{SPI0, SPI1, SIM};

use crate::gpio::gpioa::{PA14, PA15, PA16, PA17};
use crate::gpio::gpiob::{PB10, PB11, PB16, PB17};
use crate::gpio::gpioc::{PC4, PC5, PC6, PC7};
use crate::gpio::gpiod::{PD0, PD1, PD2, PD3, PD4, PD5, PD6, PD7};
use crate::gpio::gpioe::{PE1, PE2, PE3, PE4};
use crate::gpio::{Alternate2, Alternate5};

/// SPI error
#[derive(Debug)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
    #[doc(hidden)]
    _Extensible,
}

pub trait MOSI<SPI> {
}

impl MOSI<SPI0> for PA16<Alternate2> {}
impl MOSI<SPI0> for PA17<Alternate5> {}
impl MOSI<SPI0> for PC6<Alternate2> {}
impl MOSI<SPI0> for PC7<Alternate5> {}
impl MOSI<SPI0> for PD2<Alternate2> {}
impl MOSI<SPI0> for PD3<Alternate5> {}

impl MOSI<SPI1> for PB16<Alternate2> {}
impl MOSI<SPI1> for PB17<Alternate5> {}
impl MOSI<SPI1> for PD6<Alternate2> {}
impl MOSI<SPI1> for PD7<Alternate5> {}
impl MOSI<SPI1> for PE1<Alternate2> {}
impl MOSI<SPI1> for PE3<Alternate5> {}

pub trait MISO<SPI> {
}

impl MISO<SPI0> for PA16<Alternate5> {}
impl MISO<SPI0> for PA17<Alternate2> {}
impl MISO<SPI0> for PC6<Alternate5> {}
impl MISO<SPI0> for PC7<Alternate2> {}
impl MISO<SPI0> for PD2<Alternate5> {}
impl MISO<SPI0> for PD3<Alternate2> {}

impl MISO<SPI1> for PB16<Alternate5> {}
impl MISO<SPI1> for PB17<Alternate2> {}
impl MISO<SPI1> for PD6<Alternate5> {}
impl MISO<SPI1> for PD7<Alternate2> {}
impl MISO<SPI1> for PE1<Alternate5> {}
impl MISO<SPI1> for PE3<Alternate2> {}

pub trait SCK<SPI> {
}

impl SCK<SPI0> for PA15<Alternate2> {}
impl SCK<SPI0> for PD1<Alternate2> {}
impl SCK<SPI0> for PC5<Alternate2> {}

impl SCK<SPI1> for PB11<Alternate2> {}
impl SCK<SPI1> for PD5<Alternate2> {}
impl SCK<SPI1> for PE2<Alternate2> {}

pub trait SS<SPI> {
}

impl SS<SPI0> for PA14<Alternate2> {}
impl SS<SPI0> for PC4<Alternate2> {}
impl SS<SPI0> for PD0<Alternate2> {}

impl SS<SPI1> for PB10<Alternate2> {}
impl SS<SPI1> for PD4<Alternate2> {}
impl SS<SPI1> for PE4<Alternate2> {}

pub struct Spi<SPI, MOSIPin, MISOPin, SCKPin> {
    spi: SPI,
    mosi: MOSIPin,
    miso: MISOPin,
    sck: SCKPin,
}

impl<MOSIPin, MISOPin, SCKPin> Spi<SPI0, MOSIPin, MISOPin, SCKPin> {
    pub fn spi0/*<F>*/(
        spi: SPI0,
        mosi: MOSIPin,
        miso: MISOPin,
        sck: SCKPin,
        mode: Mode,
        /*freq: F,
        clocks: Clocks,*/
        sim: &mut SIM,
    ) -> Self
    where
        //F: Into<Hertz>, // TODO: Frequency
        MOSIPin: MOSI<SPI0>,
        MISOPin: MISO<SPI0>,
        SCKPin: SCK<SPI0>,
    {
        Spi::_spi0(spi, mosi, miso, sck, mode/*, freq.into(), clocks*/, sim)
    }
}

impl<MOSIPin, MISOPin, SCKPin> Spi<SPI1, MOSIPin, MISOPin, SCKPin> {
    pub fn spi1/*<F>*/(
        spi: SPI1,
        mosi: MOSIPin,
        miso: MISOPin,
        sck: SCKPin,
        mode: Mode,
        /*freq: F,
        clocks: Clocks,*/
        sim: &mut SIM,
    ) -> Self
    where
        //F: Into<Hertz>,
        MOSIPin: MOSI<SPI0>,
        MISOPin: MISO<SPI0>,
        SCKPin: SCK<SPI0>,
    {
        Spi::_spi1(spi, mosi, miso, sck, mode/*, freq.into(), clocks*/, sim)
    }
}

macro_rules! hal {
    ($($SPIX:ident: ($_spiX:ident, $spiX:ident),)+) => {
        $(
            impl<MOSIPin, MISOPin, SCKPin> Spi<$SPIX, MOSIPin, MISOPin, SCKPin> {
                fn $_spiX(
                    spi: $SPIX,
                    mosi: MOSIPin,
                    miso: MISOPin,
                    sck: SCKPin,
                    mode: Mode,
                    /*freq: Hertz,
                    clocks: Clocks,
                    apb: &mut $APB,*/
                    sim: &mut SIM,
                ) -> Self {
                    // Enable $SPIX
                    sim.scgc4.modify(|_, w| w.$spiX().set_bit());

                    /*let br = match clocks.pclk2().0 / freq.0 {
                        0 => unreachable!(),
                        1...2 => 0b000,
                        3...5 => 0b001,
                        6...11 => 0b010,
                        12...23 => 0b011,
                        24...47 => 0b100,
                        48...95 => 0b101,
                        96...191 => 0b110,
                        _ => 0b111,
                    };*/
                    // TODO: Frequency

                    spi.c1.write(|w| {
                        w.spie().clear_bit() // Disable interrupts
                            .spe().set_bit() // Enable SPI
                            .sptie().clear_bit() // Disable transmit interrupt
                            .mstr().set_bit() // Enable master mode
                            .cpol().bit(mode.polarity == Polarity::IdleHigh) // Polarity
                            .cpha().bit(mode.phase == Phase::CaptureOnSecondTransition) // Phase
                            .ssoe().clear_bit() // Disable SS output
                    });
                    spi.c2.write(|w| {
                        w.spmie().clear_bit()
                            .txdmae().clear_bit()
                            .modfen().clear_bit()
                            .bidiroe().clear_bit()
                            .rxdmae().clear_bit()
                            .spiswai().clear_bit()
                            .spc0().clear_bit()
                    });
                    spi.br.write(|w| {
                        w.sppr()._000() // Prescale divisor 1
                            .spr()._0001() // Baud rate divisor 4
                    });

                    Spi { spi, mosi, miso, sck }
                }

                pub fn free(self) -> ($SPIX, MOSIPin, MISOPin, SCKPin) {
                    (self.spi, self.mosi, self.miso, self.sck)
                }
            }

            impl<MOSIPin, MISOPin, SCKPin> hal::spi::FullDuplex<u8> for Spi<$SPIX, MOSIPin, MISOPin, SCKPin> {
                type Error = Error;

                fn read(&mut self) -> nb::Result<u8, Error> {
                    let s = self.spi.s.read();

                    Err(if s.modf().bit_is_set() {
                        nb::Error::Other(Error::ModeFault)
                    } else if s.sprf().bit_is_set() {
                        return Ok(self.spi.d.read().bits())
                    } else {
                        nb::Error::WouldBlock
                    })
                }

                fn send(&mut self, byte: u8) -> nb::Result<(), Error> {
                    let s = self.spi.s.read();

                    Err(if s.modf().bit_is_set() {
                        nb::Error::Other(Error::ModeFault)
                    } else if s.sptef().bit_is_set() {
                        self.spi.d.write(|w| unsafe {
                            w.bits(byte)
                        });
                        return Ok(());
                    } else {
                        nb::Error::WouldBlock
                    })
                }

            }

            impl<MOSIPin, MISOPin, SCKPin> crate::hal::blocking::spi::transfer::Default<u8> for Spi<$SPIX, MOSIPin, MISOPin, SCKPin> {}

            impl<MOSIPin, MISOPin, SCKPin> crate::hal::blocking::spi::write::Default<u8> for Spi<$SPIX, MOSIPin, MISOPin, SCKPin> {}
        )+
    }
}

hal! {
    SPI0: (_spi0, spi0),
    SPI1: (_spi1, spi1),
}
