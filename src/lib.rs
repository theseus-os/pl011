//! Driver for the pl011 UARTs in the QEMU implementation
//!
//! This crate provides basic drivers for the UARTS exposed by
//! QEMU. You can see the implementation of these uarts
//! [here](https://github.com/qemu/qemu/blob/master/hw/arm/stellaris.c)
//!
//! The QEMU target actually exposes 4 different UARTS, that can each
//! be redirected to arbitary character devices or files. This crate
//! allows those UARTS to be accessed in order to support more
//! complicated use cases than can be provided by
//! [cortex_m_semihosting](https://crates.io/crates/cortex-m-semihosting).

#![deny(missing_docs)]
#![no_std]
use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use embedded_hal::serial;
use nb;
use volatile_register::{RO, RW, WO};

/// Struct representing PL011 registers. Not intended to be directly
/// used
#[repr(C)]
pub struct PL011_Regs {
    /// Data Register
    pub uartdr: RW<u32>,
    /// receive status / error clear
    pub uartrsr: RW<u32>,
    reserved0: [u32; 4],
    /// flag register
    pub uartfr: RO<u32>,
    reserved1: u32,
    /// IrDA Low power counter register
    pub uartilpr: RW<u32>,
    /// integer baud rate
    pub uartibrd: RW<u32>,
    /// fractional baud rate
    pub uartfbrd: RW<u32>,
    /// line control
    pub uartlcr_h: RW<u32>,
    /// control
    pub uartcr: RW<u32>,
    /// interrupt fifo level select
    pub uartifls: RW<u32>,
    /// interrupt mask set/clear
    pub uartimsc: RW<u32>,
    /// raw interrupt status
    pub uartris: RO<u32>,
    /// masked interrupt status
    pub uartmis: RO<u32>,
    /// interrupt clear
    pub uarticr: WO<u32>,
    /// dma control
    pub uartdmacr: RW<u32>,
    reserved2: [u32; 997],
    /// UART Periph ID0
    pub uartperiphid0: RO<u32>,
    /// UART Periph ID1
    pub uartperiphid1: RO<u32>,
    /// UART Periph ID2
    pub uartperiphid2: RO<u32>,
    /// UART Periph ID3
    pub uartperiphid3: RO<u32>,
    /// UART PCell ID0
    pub uartpcellid0: RO<u32>,
    /// UART PCell ID1
    pub uartpcellid1: RO<u32>,
    /// UART PCell ID2
    pub uartpcellid2: RO<u32>,
    /// UART PCell ID3
    pub uartpcellid3: RO<u32>,
}

/// Error type necessary for embedded_hal usage. No errors supported
#[derive(Debug, Copy, Clone)]
pub struct Error;

/// Struct representing the actual driver.
///
/// Notice that there are no silly ideas like setting the baud rate,
/// or assigning GPIO pins to the driver: the qemu implementation
/// doesnt need any of that, we can just write to the registers
/// directly.
///
/// Implements embedded_hal::serial as well as core::fmt::Write
///
/// # Examples
///
/// ```
/// use pl011_qemu;
/// // build a driver for UART1
/// let mut uart = pl011_qemu::PL011::new(pl011_qemu::UART1::take().unwrap());
/// ```
pub struct PL011 {
    regs: &'static mut PL011_Regs,
}

/// RX methods
impl PL011 {
    /// Is the receive-buffer-empty flag clear?
    pub fn has_incoming_data(&self) -> bool {
        let uartfr = unsafe { (*self.regs).uartfr.read() };
        uartfr & 0x10 == 0
    }

    /// reads a single byte out the uart
    ///
    /// spins until a byte is available in the fifo
    pub fn read_byte(&self) -> u8 {
        // loop while RXFE is set
        while !self.has_incoming_data() {}
        // read the data register. Atomic read is side effect free
        let data = unsafe { (*self.regs).uartdr.read() & 0xff };
        data as u8
    }
}

/// TX methods
impl PL011 {
    /// writes a single byte out the uart
    ///
    /// spins until space is available in the fifo
    pub fn write_byte(&self, data: u8) {
        while !self.is_writeable() {}
        unsafe { (*self.regs).uartdr.write(data as u32) };
    }

    /// Is the transmit-buffer-full flag clear?
    pub fn is_writeable(&self) -> bool {
        let uartfr = unsafe { (*self.regs).uartfr.read() };
        uartfr & 0x20 == 0
    }
}

impl serial::Read<u8> for PL011 {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        // if RXFE is set (rx fifo is empty)
        if self.has_incoming_data() {
            Ok(unsafe { (*self.regs).uartdr.read() & 0xff } as u8)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl serial::Write<u8> for PL011 {
    type Error = Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.flush()?;
        unsafe { (*self.regs).uartdr.write(word as u32) };
        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        if self.is_writeable() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl fmt::Write for PL011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use embedded_hal::serial::Write;
        for b in s.as_bytes().iter() {
            if nb::block!(self.write(*b)).is_err() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}

/// Generic methods
impl PL011 {
    /// Initialize a UART driver. Needs a UART struct to be passed in
    pub fn new(regs: *mut PL011_Regs) -> Self {
        let regs = unsafe { regs.as_mut() }.unwrap();
        Self { regs }
    }
}
