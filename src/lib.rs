#![no_std]

extern crate cortex_m;
extern crate embedded_hal as hal;
pub extern crate mkl25z4;
extern crate void;

pub mod clocks;
pub mod gpio;
pub mod spi;
pub mod time;
pub mod timer;
pub mod watchdog;

#[derive(Debug, PartialEq, Eq)]
pub enum NoError {}
