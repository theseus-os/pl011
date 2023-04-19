//! Driver for the pl011 UARTs in the QEMU implementation
//!
//! This crate provides basic drivers for the UARTS exposed by
//! QEMU. You can see the implementation of these uarts
//! [here](https://github.com/qemu/qemu/blob/master/hw/char/pl011.c)

#![no_std]
use core::fmt;
use volatile_register::{RO, RW, WO};

/// Struct representing PL011 registers. Not intended to be directly used
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

const UARTIMSC_RXIM: u32 = 1 << 4;

const UARTLCR_FEN: u32 = 1 << 4;

const UARTCR_RX_ENABLED: u32 = 1 << 9;
const UARTCR_TX_ENABLED: u32 = 1 << 8;
const UARTCR_UART_ENABLED: u32 = 1 << 0;

const UARTFR_RX_BUF_EMPTY: u32 = 1 << 4;
const UARTFR_TX_BUF_FULL: u32 = 1 << 5;

/// A PL011 Single-Serial-Port Controller.
pub struct PL011 {
    regs: &'static mut PL011_Regs,
}

/// Generic methods
impl PL011 {
    /// Initialize a UART driver. Needs a UART struct to be passed in
    pub fn new(regs: *mut PL011_Regs) -> Self {
        let regs = unsafe { regs.as_mut() }.unwrap();
        Self { regs }
    }

    /// Enable on-receive interrupt
    pub fn enable_rx_interrupt(&mut self, enable: bool) {
        let mut reg = (*self.regs).uartimsc.read();

        match enable {
            true  => reg |=  UARTIMSC_RXIM,
            false => reg &= !UARTIMSC_RXIM,
        };

        unsafe { (*self.regs).uartimsc.write(reg) };
    }

    /// Set FIFO mode
    pub fn set_fifo_mode(&mut self, enable: bool) {
        let mut reg = (*self.regs).uartlcr_h.read();

        match enable {
            true  => reg |=  UARTLCR_FEN,
            false => reg &= !UARTLCR_FEN,
        };

        unsafe { (*self.regs).uartlcr_h.write(reg) };
    }

    /// Outputs a summary of the state of the controller using `log::info!()`
    pub fn log_status(&self) {
        let reg = (*self.regs).uartcr.read();
        log::info!("RX enabled: {}", (reg & UARTCR_RX_ENABLED) > 0);
        log::info!("TX enabled: {}", (reg & UARTCR_TX_ENABLED) > 0);
        log::info!("UART enabled: {}", (reg & UARTCR_UART_ENABLED) > 0);
    }

    /// Returns true if the receive-buffer-empty flag is clear.
    pub fn has_incoming_data(&self) -> bool {
        let uartfr = (*self.regs).uartfr.read();
        uartfr & UARTFR_RX_BUF_EMPTY == 0
    }

    /// Reads a single byte out the uart
    ///
    /// Spins until a byte is available in the fifo.
    pub fn read_byte(&self) -> u8 {
        while !self.has_incoming_data() {}
        (*self.regs).uartdr.read() as u8
    }

    /// Reads bytes into a slice until there is none available.
    pub fn read_bytes(&self, bytes: &mut [u8]) -> usize {
        let mut read = 0;

        while read < bytes.len() && self.has_incoming_data() {
            bytes[read] = self.read_byte();
            read += 1;
        }

        read
    }

    /// Returns true if the transmit-buffer-full flag is clear.
    pub fn is_writeable(&self) -> bool {
        let uartfr = (*self.regs).uartfr.read();
        uartfr & UARTFR_TX_BUF_FULL == 0
    }

    /// Writes a single byte out the uart.
    ///
    /// Spins until space is available in the fifo.
    pub fn write_byte(&self, data: u8) {
        while !self.is_writeable() {}
        unsafe { (*self.regs).uartdr.write(data as u32) };
    }

    /// Writes a byte slice out the uart.
    ///
    /// Spins until space is available in the fifo.
    pub fn write_bytes(&self, bytes: &[u8]) {
        for b in bytes {
            self.write_byte(*b);
        }
    }
}

impl fmt::Write for PL011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}
