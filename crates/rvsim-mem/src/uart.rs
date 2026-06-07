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

const LCR_DLAB: u8 = 1 << 7;
const MCR_LOOP: u8 = 1 << 4;

const IER_THRE: u8 = 1 << 1;
const IIR_NONE: u8 = 0x01;
const IIR_THRE: u8 = 0x02;

struct State {
    ier: u8,
    fcr: u8,
    lcr: u8,
    mcr: u8,
    scr: u8,
    dll: u8,
    dlm: u8,
    thre_ip: bool,
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
                fcr: 0,
                lcr: 0,
                mcr: 0,
                scr: 0,
                dll: 0,
                dlm: 0,
                thre_ip: false,
                sink,
            }),
        }
    }

    pub fn interrupt_pending(&self) -> bool {
        let s = self.state.borrow();
        s.ier & IER_THRE != 0 && s.thre_ip
    }

    pub fn read8(&self, offset: u32) -> Result<u8, Trap> {
        let reg = offset & 0x7;
        let val = match reg {
            REG_THR_RBR => {
                let s = self.state.borrow();
                if s.lcr & LCR_DLAB != 0 { s.dll } else { 0 }
            }
            REG_IER => {
                let s = self.state.borrow();
                if s.lcr & LCR_DLAB != 0 { s.dlm } else { s.ier }
            }
            REG_IIR_FCR => {
                let mut s = self.state.borrow_mut();
                let fifo_bits = if s.fcr & 0x01 != 0 { 0xC0u8 } else { 0 };
                if s.ier & IER_THRE != 0 && s.thre_ip {
                    s.thre_ip = false;
                    IIR_THRE | fifo_bits
                } else {
                    IIR_NONE | fifo_bits
                }
            }
            REG_LCR => self.state.borrow().lcr,
            REG_MCR => self.state.borrow().mcr,
            REG_LSR => LSR_DEFAULT,
            REG_MSR => {
                let s = self.state.borrow();
                if s.mcr & MCR_LOOP != 0 {
                    // Loopback: MCR outputs reflect to MSR inputs
                    let mcr = s.mcr;
                    let mut msr: u8 = 0;
                    if mcr & 0x01 != 0 { msr |= 0x20; } // DTR → DSR
                    if mcr & 0x02 != 0 { msr |= 0x10; } // RTS → CTS
                    if mcr & 0x04 != 0 { msr |= 0x40; } // OUT1 → RI
                    if mcr & 0x08 != 0 { msr |= 0x80; } // OUT2 → DCD
                    msr
                } else {
                    MSR_DEFAULT
                }
            }
            REG_SCR => self.state.borrow().scr,
            _ => 0,
        };
        if std::env::var("RVSIM_UART_TRACE").is_ok() && reg != REG_LSR {
            let dlab = self.state.borrow().lcr & LCR_DLAB != 0;
            let reg_name = match reg {
                REG_THR_RBR if dlab => "DLL",
                REG_THR_RBR => "RBR",
                REG_IER if dlab => "DLM",
                REG_IER => "IER",
                REG_IIR_FCR => "IIR",
                REG_LCR => "LCR", REG_MCR => "MCR", REG_MSR => "MSR",
                REG_SCR => "SCR", _ => "???",
            };
            eprintln!("[UART] read  {} = 0x{:02x}", reg_name, val);
        }
        Ok(val)
    }

    pub fn write8(&self, offset: u32, val: u8) -> Result<(), Trap> {
        let reg = offset & 0x7;
        if std::env::var("RVSIM_UART_TRACE").is_ok() {
            let dlab = self.state.borrow().lcr & LCR_DLAB != 0;
            let reg_name = match reg {
                REG_THR_RBR if dlab => "DLL",
                REG_THR_RBR => "THR",
                REG_IER if dlab => "DLM",
                REG_IER => "IER",
                REG_IIR_FCR => "FCR",
                REG_LCR => "LCR",
                REG_MCR => "MCR",
                _ => "???",
            };
            if reg == REG_THR_RBR && !dlab {
                if (0x20..0x7f).contains(&val) {
                    eprintln!("[UART] write THR = '{}'", val as char);
                } else {
                    eprintln!("[UART] write THR = 0x{:02x}", val);
                }
            } else {
                eprintln!("[UART] write {} = 0x{:02x}", reg_name, val);
            }
        }
        let mut s = self.state.borrow_mut();
        match reg {
            REG_THR_RBR => {
                if s.lcr & LCR_DLAB != 0 {
                    s.dll = val;
                } else {
                    let _ = s.sink.write_all(&[val]);
                    let _ = s.sink.flush();
                    s.thre_ip = true;
                }
            }
            REG_IER => {
                if s.lcr & LCR_DLAB != 0 {
                    s.dlm = val;
                } else {
                    let old = s.ier;
                    s.ier = val;
                    if val & IER_THRE != 0 && old & IER_THRE == 0 {
                        s.thre_ip = true;
                    }
                }
            }
            REG_IIR_FCR => {
                s.fcr = val & 0x01;
            }
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
        uart.write8(REG_LCR, 0x03).unwrap(); // DLAB clear
        assert_eq!(uart.read8(REG_IER).unwrap(), 0xAB);
        assert_eq!(uart.read8(REG_LCR).unwrap(), 0x03);
    }

    #[test]
    fn dlab_muxes_dll_dlm() {
        let (uart, buf) = uart_with_sink();
        // Set DLAB
        uart.write8(REG_LCR, LCR_DLAB).unwrap();
        // Write divisor latch registers
        uart.write8(REG_THR_RBR, 0x18).unwrap(); // DLL
        uart.write8(REG_IER, 0x00).unwrap();      // DLM
        // Read them back
        assert_eq!(uart.read8(REG_THR_RBR).unwrap(), 0x18); // DLL
        assert_eq!(uart.read8(REG_IER).unwrap(), 0x00);      // DLM
        // Nothing should have been sent to the sink
        assert!(buf.lock().unwrap().is_empty());
        // Clear DLAB — IER should still be 0 (not corrupted by DLM write)
        uart.write8(REG_LCR, 0x03).unwrap();
        assert_eq!(uart.read8(REG_IER).unwrap(), 0x00);
    }

    #[test]
    fn loopback_msr_reflects_mcr() {
        let (uart, _) = uart_with_sink();
        // Set MCR to loopback + OUT2 + RTS (what Linux autoconfig does: 0x1A)
        uart.write8(REG_MCR, MCR_LOOP | 0x0A).unwrap();
        let msr = uart.read8(REG_MSR).unwrap();
        // RTS(bit1) → CTS(0x10), OUT2(bit3) → DCD(0x80) = 0x90
        assert_eq!(msr & 0xF0, 0x90);
    }

    #[test]
    fn lsr_is_read_only() {
        let (uart, _) = uart_with_sink();
        uart.write8(REG_LSR, 0xFF).unwrap();
        assert_eq!(uart.read8(REG_LSR).unwrap(), LSR_DEFAULT);
    }
}
