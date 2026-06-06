//! Address-routing bus that composes RAM with memory-mapped devices.

use rvsim_core::csr::{MIP_MEIP, MIP_MSIP, MIP_MTIP, MIP_SEIP};
use rvsim_core::mem::Memory;
use rvsim_core::trap::Trap;

use crate::clint::{Clint, CLINT_BASE, CLINT_SIZE};
use crate::flat::FlatMemory;
use crate::plic::{Plic, PLIC_BASE, PLIC_SIZE};
use crate::uart::{Uart, UART_BASE, UART_SIZE};

pub struct Bus {
    pub ram: FlatMemory,
    pub clint: Clint,
    pub plic: Plic,
    pub uart: Uart,
}

#[derive(Debug, Clone, Copy)]
enum Region {
    Clint(u32),
    Plic(u32),
    Uart(u32),
    Ram,
}

impl Bus {
    pub fn new(ram: FlatMemory, clint: Clint, plic: Plic, uart: Uart) -> Self {
        Self { ram, clint, plic, uart }
    }

    fn region(addr: u32) -> Region {
        if (CLINT_BASE..CLINT_BASE.wrapping_add(CLINT_SIZE)).contains(&addr) {
            Region::Clint(addr - CLINT_BASE)
        } else if (PLIC_BASE..PLIC_BASE.wrapping_add(PLIC_SIZE)).contains(&addr) {
            Region::Plic(addr - PLIC_BASE)
        } else if (UART_BASE..UART_BASE.wrapping_add(UART_SIZE)).contains(&addr) {
            Region::Uart(addr - UART_BASE)
        } else {
            Region::Ram
        }
    }

    /// Per-cycle tick — advance device counters. Called once per `Hart::step`.
    pub fn tick(&mut self, cycle: u64) {
        self.clint.tick(cycle);
        if self.uart.interrupt_pending() {
            self.plic.set_pending(10);
        } else {
            self.plic.clear_pending(10);
        }
    }

    /// Returns the set of `mip` bits the bus is currently driving from hardware.
    /// Caller folds these into `mip` and passes to `Hart::step`.
    pub fn pending_interrupts(&self) -> u32 {
        let mut bits = 0u32;
        if self.clint.mtip_pending() {
            bits |= MIP_MTIP;
        }
        if self.clint.msip_pending() {
            bits |= MIP_MSIP;
        }
        if self.plic.meip_pending() {
            bits |= MIP_MEIP;
        }
        if self.plic.seip_pending() {
            bits |= MIP_SEIP;
        }
        bits
    }
}

impl Memory for Bus {
    fn read8(&self, addr: u32) -> Result<u8, Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.read8(off),
            Region::Plic(off) => self.plic.read8(off),
            Region::Uart(off) => self.uart.read8(off),
            Region::Ram => self.ram.read8(addr),
        }
    }

    fn read16(&self, addr: u32) -> Result<u16, Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.read16(off),
            Region::Plic(off) => self.plic.read16(off),
            Region::Uart(off) => self.uart.read16(off),
            Region::Ram => self.ram.read16(addr),
        }
    }

    fn read32(&self, addr: u32) -> Result<u32, Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.read32(off),
            Region::Plic(off) => self.plic.read32(off),
            Region::Uart(off) => self.uart.read32(off),
            Region::Ram => self.ram.read32(addr),
        }
    }

    fn write8(&mut self, addr: u32, val: u8) -> Result<(), Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.write8(off, val),
            Region::Plic(off) => self.plic.write8(off, val),
            Region::Uart(off) => self.uart.write8(off, val),
            Region::Ram => self.ram.write8(addr, val),
        }
    }

    fn write16(&mut self, addr: u32, val: u16) -> Result<(), Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.write16(off, val),
            Region::Plic(off) => self.plic.write16(off, val),
            Region::Uart(off) => self.uart.write16(off, val),
            Region::Ram => self.ram.write16(addr, val),
        }
    }

    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Trap> {
        match Self::region(addr) {
            Region::Clint(off) => self.clint.write32(off, val),
            Region::Plic(off) => self.plic.write32(off, val),
            Region::Uart(off) => self.uart.write32(off, val),
            Region::Ram => self.ram.write32(addr, val),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct VecSink(Arc<Mutex<Vec<u8>>>);
    impl Write for VecSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }

    fn make_bus() -> (Bus, Arc<Mutex<Vec<u8>>>) {
        let ram = FlatMemory::new(0x1000, 0x8000_0000);
        let clint = Clint::new();
        let plic = Plic::new();
        let buf = Arc::new(Mutex::new(Vec::new()));
        let uart = Uart::with_sink(Box::new(VecSink(buf.clone())));
        (Bus::new(ram, clint, plic, uart), buf)
    }

    #[test]
    fn routes_to_ram() {
        let (mut bus, _) = make_bus();
        bus.write32(0x8000_0000, 0xDEAD_BEEF).unwrap();
        assert_eq!(bus.read32(0x8000_0000).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn routes_to_uart_tx() {
        let (mut bus, buf) = make_bus();
        bus.write8(UART_BASE, b'X').unwrap();
        assert_eq!(buf.lock().unwrap().as_slice(), b"X");
    }

    #[test]
    fn routes_to_clint_mtime() {
        let (mut bus, _) = make_bus();
        bus.tick(0xCAFE);
        assert_eq!(bus.read32(CLINT_BASE + 0xBFF8).unwrap(), 0xCAFE);
    }

    #[test]
    fn oob_ram_faults() {
        let (bus, _) = make_bus();
        assert_eq!(bus.read32(0x9000_0000), Err(Trap::LoadAccessFault));
    }

    #[test]
    fn pending_interrupts_reports_mtip() {
        let (mut bus, _) = make_bus();
        // Set mtimecmp = 5 via the bus.
        bus.write32(CLINT_BASE + 0x4000, 5).unwrap();
        bus.write32(CLINT_BASE + 0x4004, 0).unwrap();
        bus.tick(4);
        assert_eq!(bus.pending_interrupts(), 0);
        bus.tick(5);
        assert_eq!(bus.pending_interrupts(), MIP_MTIP);
    }
}
