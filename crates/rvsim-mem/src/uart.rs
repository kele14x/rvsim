//! Minimal NS16550A subset — just enough for OpenSBI/Linux `printf` early console.
//!
//! Registers (1 byte each, offsets from base):
//! - 0: THR (write) / RBR (read)
//! - 1: IER
//! - 2: IIR (read) / FCR (write)
//! - 3: LCR
//! - 4: MCR
//! - 5: LSR  — read-only
//! - 6: MSR  — read-only
//! - 7: SCR  — scratch
//!
//! Output goes to a user-supplied sink (`Box<dyn Write>`), defaulting to stdout.
//! No RX yet — reading RBR returns 0 and LSR.DR stays clear.

use std::cell::RefCell;
use std::io::{self, Write};

use rvsim_core::trap::Trap;

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x100;

const REG_THR_RBR: u32 = 0;
const REG_IER: u32 = 1;
const REG_IIR_FCR: u32 = 2;
const REG_LCR: u32 = 3;
const REG_MCR: u32 = 4;
const REG_LSR: u32 = 5;
const REG_MSR: u32 = 6;
const REG_SCR: u32 = 7;

// LSR bits: bit 6 = transmitter empty, bit 5 = THR empty. Always set —
// we never block on TX. Bit 0 = data-ready (RX) — always clear (no input).
const LSR_DEFAULT: u8 = (1 << 6) | (1 << 5);
const MSR_DEFAULT: u8 = 0xB0; // CD, DSR, CTS asserted

struct State {
    ier: u8,
    lcr: u8,
    mcr: u8,
    scr: u8,
    sink: Box<dyn Write + Send>,
}

pub struct Uart {
    state: RefCell<State>,
}

impl Uart {
    pub fn stdout() -> Self {
        Self::with_sink(Box::new(io::stdout()))
    }

    pub fn with_sink(sink: Box<dyn Write + Send>) -> Self {
        Self {
            state: RefCell::new(State {
                ier: 0,
                lcr: 0,
                mcr: 0,
                scr: 0,
                sink,
            }),
        }
    }

    pub fn read8(&self, offset: u32) -> Result<u8, Trap> {
        let s = self.state.borrow();
        let val = match offset & 0x7 {
            REG_THR_RBR => 0, // no RX
            REG_IER => s.ier,
            REG_IIR_FCR => 0x01, // "no interrupt pending"
            REG_LCR => s.lcr,
            REG_MCR => s.mcr,
            REG_LSR => LSR_DEFAULT,
            REG_MSR => MSR_DEFAULT,
            REG_SCR => s.scr,
            _ => 0,
        };
        Ok(val)
    }

    pub fn write8(&self, offset: u32, val: u8) -> Result<(), Trap> {
        let mut s = self.state.borrow_mut();
        match offset & 0x7 {
            REG_THR_RBR => {
                // Best-effort write; ignore I/O errors so a closed stdout
                // doesn't break the simulator.
                let _ = s.sink.write_all(&[val]);
                let _ = s.sink.flush();
            }
            REG_IER => s.ier = val,
            REG_IIR_FCR => { /* FCR write: ignore */ }
            REG_LCR => s.lcr = val,
            REG_MCR => s.mcr = val,
            REG_LSR | REG_MSR => { /* read-only */ }
            REG_SCR => s.scr = val,
            _ => {}
        }
        Ok(())
    }

    // 16/32-bit accesses operate on the addressed byte.
    pub fn read16(&self, offset: u32) -> Result<u16, Trap> {
        Ok(self.read8(offset)? as u16)
    }
    pub fn read32(&self, offset: u32) -> Result<u32, Trap> {
        Ok(self.read8(offset)? as u32)
    }
    pub fn write16(&self, offset: u32, val: u16) -> Result<(), Trap> {
        self.write8(offset, val as u8)
    }
    pub fn write32(&self, offset: u32, val: u32) -> Result<(), Trap> {
        self.write8(offset, val as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // A Write impl that pushes to a shared Vec so tests can inspect output.
    #[derive(Clone)]
    struct VecSink(Arc<Mutex<Vec<u8>>>);
    impl Write for VecSink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }

    fn uart_with_sink() -> (Uart, Arc<Mutex<Vec<u8>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let sink = VecSink(buf.clone());
        (Uart::with_sink(Box::new(sink)), buf)
    }

    #[test]
    fn tx_writes_byte_to_sink() {
        let (uart, buf) = uart_with_sink();
        uart.write8(REG_THR_RBR, b'A').unwrap();
        uart.write8(REG_THR_RBR, b'B').unwrap();
        assert_eq!(buf.lock().unwrap().as_slice(), b"AB");
    }

    #[test]
    fn lsr_always_reports_thr_empty() {
        let (uart, _) = uart_with_sink();
        let lsr = uart.read8(REG_LSR).unwrap();
        assert!(lsr & (1 << 5) != 0, "THR-empty bit must be set");
        assert!(lsr & (1 << 0) == 0, "data-ready bit must be clear (no RX)");
    }

    #[test]
    fn ier_and_lcr_round_trip() {
        let (uart, _) = uart_with_sink();
        uart.write8(REG_IER, 0xAB).unwrap();
        uart.write8(REG_LCR, 0x83).unwrap();
        assert_eq!(uart.read8(REG_IER).unwrap(), 0xAB);
        assert_eq!(uart.read8(REG_LCR).unwrap(), 0x83);
    }

    #[test]
    fn lsr_is_read_only() {
        let (uart, _) = uart_with_sink();
        uart.write8(REG_LSR, 0xFF).unwrap();
        assert_eq!(uart.read8(REG_LSR).unwrap(), LSR_DEFAULT);
    }
}
