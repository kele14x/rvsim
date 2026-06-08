use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::Read;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

use crossterm::terminal;
use goblin::elf::{header, Elf};
use rvsim_core::cpu::Hart;
use rvsim_core::mem::Memory;
use rvsim_mem::bus::Bus;
use rvsim_mem::clint::Clint;
use rvsim_mem::flat::FlatMemory;
use rvsim_mem::plic::Plic;
use rvsim_mem::uart::Uart;

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

const RAM_BASE: u32 = 0x8000_0000;
const RAM_SIZE: usize = 256 * 1024 * 1024; // 256 MB
/// Default DTB load address: 32 MiB into RAM. Sits comfortably above any
/// reasonable kernel image and below the rest of usable RAM, matching how
/// real boards typically lay out the previous-stage handoff.
const DEFAULT_DTB_ADDR: u32 = 0x8220_0000;

/// Default load base for static-PIE firmware (ELF type ET_DYN). OpenSBI's
/// generic platform expects to be placed here (FW_TEXT_START), then self-
/// relocates to wherever the previous stage put it.
const DEFAULT_PIE_BASE: u32 = 0x8000_0000;

/// Default kernel load address — OpenSBI fw_jump expects the payload here.
const DEFAULT_KERNEL_ADDR: u32 = 0x8040_0000;

struct CliArgs {
    elf_path: String,
    dtb_path: Option<String>,
    dtb_addr: u32,
    hartid: u32,
    load_base: Option<u32>,
    kernel_path: Option<String>,
    kernel_addr: u32,
    max_cycles: Option<u64>,
}

fn parse_args() -> CliArgs {
    let mut elf_path: Option<String> = None;
    let mut dtb_path: Option<String> = None;
    let mut dtb_addr: u32 = DEFAULT_DTB_ADDR;
    let mut hartid: u32 = 0;
    let mut load_base: Option<u32> = None;
    let mut kernel_path: Option<String> = None;
    let mut kernel_addr: u32 = DEFAULT_KERNEL_ADDR;
    let mut max_cycles: Option<u64> = None;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dtb" => {
                dtb_path = Some(iter.next().unwrap_or_else(|| {
                    eprintln!("--dtb requires a path");
                    process::exit(2);
                }));
            }
            "--dtb-addr" => {
                let v = iter.next().unwrap_or_else(|| {
                    eprintln!("--dtb-addr requires a hex/decimal address");
                    process::exit(2);
                });
                dtb_addr = parse_u32(&v).unwrap_or_else(|| {
                    eprintln!("--dtb-addr: cannot parse '{}' as u32", v);
                    process::exit(2);
                });
            }
            "--hartid" => {
                let v = iter.next().unwrap_or_else(|| {
                    eprintln!("--hartid requires a number");
                    process::exit(2);
                });
                hartid = parse_u32(&v).unwrap_or_else(|| {
                    eprintln!("--hartid: cannot parse '{}' as u32", v);
                    process::exit(2);
                });
            }
            "--load-base" => {
                let v = iter.next().unwrap_or_else(|| {
                    eprintln!("--load-base requires an address");
                    process::exit(2);
                });
                load_base = Some(parse_u32(&v).unwrap_or_else(|| {
                    eprintln!("--load-base: cannot parse '{}' as u32", v);
                    process::exit(2);
                }));
            }
            "--kernel" => {
                kernel_path = Some(iter.next().unwrap_or_else(|| {
                    eprintln!("--kernel requires a path");
                    process::exit(2);
                }));
            }
            "--kernel-addr" => {
                let v = iter.next().unwrap_or_else(|| {
                    eprintln!("--kernel-addr requires an address");
                    process::exit(2);
                });
                kernel_addr = parse_u32(&v).unwrap_or_else(|| {
                    eprintln!("--kernel-addr: cannot parse '{}' as u32", v);
                    process::exit(2);
                });
            }
            "--max-cycles" => {
                let v = iter.next().unwrap_or_else(|| {
                    eprintln!("--max-cycles requires a number");
                    process::exit(2);
                });
                max_cycles = Some(v.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!("--max-cycles: cannot parse '{}' as u64", v);
                    process::exit(2);
                }));
            }
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            other if other.starts_with('-') => {
                eprintln!("unknown flag: {}", other);
                print_usage();
                process::exit(2);
            }
            _ => {
                if elf_path.is_some() {
                    eprintln!("multiple ELF arguments not supported");
                    process::exit(2);
                }
                elf_path = Some(arg);
            }
        }
    }

    let elf_path = elf_path.unwrap_or_else(|| {
        print_usage();
        process::exit(1);
    });

    CliArgs { elf_path, dtb_path, dtb_addr, hartid, load_base, kernel_path, kernel_addr, max_cycles }
}

fn parse_u32(s: &str) -> Option<u32> {
    if let Some(stripped) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(stripped, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

fn print_usage() {
    eprintln!("Usage: rvsim [options] <elf-binary>");
    eprintln!();
    eprintln!("  --dtb <path>          Load a Flattened Device Tree blob and pass its");
    eprintln!("                        address in a1 at boot (a0 = hartid).");
    eprintln!("  --dtb-addr <addr>     Where to place the DTB in RAM (default 0x{:08x}).",
        DEFAULT_DTB_ADDR);
    eprintln!("  --hartid <n>          Value passed in a0 at boot (default 0).");
    eprintln!("  --load-base <addr>    Override load base for PIE/DYN ELFs");
    eprintln!("                        (default 0x{:08x}; ignored for fixed-address ELFs).",
        DEFAULT_PIE_BASE);
    eprintln!("  --kernel <path>       Load a raw kernel image (e.g. Linux Image) at");
    eprintln!("                        --kernel-addr for OpenSBI fw_jump to boot.");
    eprintln!("  --kernel-addr <addr>  Where to load the kernel (default 0x{:08x}).",
        DEFAULT_KERNEL_ADDR);
    eprintln!("  --max-cycles <n>      Maximum simulation cycles (default 10M, 10B with --kernel).");
}

fn main() {
    let args = parse_args();

    let elf_data = fs::read(&args.elf_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", args.elf_path, e);
        process::exit(1);
    });

    let elf = Elf::parse(&elf_data).unwrap_or_else(|e| {
        eprintln!("Failed to parse ELF: {}", e);
        process::exit(1);
    });

    let ram = FlatMemory::new(RAM_SIZE, RAM_BASE);
    let rx_queue = Arc::new(Mutex::new(VecDeque::new()));
    let uart = Uart::new(Box::new(std::io::stdout()), rx_queue.clone());
    let mut bus = Bus::new(ram, Clint::new(), Plic::new(), uart);

    let _raw_guard = if args.kernel_path.is_some() {
        terminal::enable_raw_mode().ok();
        let rq = rx_queue.clone();
        thread::spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let mut buf = [0u8; 1];
            while stdin.read(&mut buf).unwrap_or(0) > 0 {
                if buf[0] == 0x03 {
                    // Ctrl-C: restore terminal and exit
                    let _ = terminal::disable_raw_mode();
                    process::exit(0);
                }
                rq.lock().unwrap().push_back(buf[0]);
            }
        });
        Some(RawModeGuard)
    } else {
        None
    };

    // For PIE firmware (ET_DYN, e.g. OpenSBI fw_jump.elf), p_paddr is a
    // *relative* offset — the previous stage chooses the load base. Default
    // to FW_TEXT_START. Fixed-address ELFs (ET_EXEC, e.g. riscv-tests) are
    // loaded verbatim.
    let is_pie = elf.header.e_type == header::ET_DYN;
    let load_base: u32 = if is_pie {
        args.load_base.unwrap_or(DEFAULT_PIE_BASE)
    } else {
        if let Some(b) = args.load_base {
            eprintln!("Warning: --load-base 0x{:08x} ignored (ELF is not PIE)", b);
        }
        0
    };

    // Load segments
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let file_offset = ph.p_offset as usize;
            let file_size = ph.p_filesz as usize;
            let vaddr = (ph.p_paddr as u32).wrapping_add(load_base);
            if file_size > 0 {
                bus.ram.load(vaddr, &elf_data[file_offset..file_offset + file_size]);
            }
        }
    }

    // Load raw kernel image first (before DTB), so the DTB can overwrite
    // the tail of the kernel if they overlap in memory.
    if let Some(path) = &args.kernel_path {
        let kernel = fs::read(path).unwrap_or_else(|e| {
            eprintln!("Failed to read kernel {}: {}", path, e);
            process::exit(1);
        });
        let end = (args.kernel_addr as u64).saturating_add(kernel.len() as u64);
        if args.kernel_addr < RAM_BASE || end > RAM_BASE as u64 + RAM_SIZE as u64 {
            eprintln!(
                "Kernel does not fit in RAM: addr=0x{:08x} len=0x{:x} ram=[0x{:08x},0x{:08x})",
                args.kernel_addr, kernel.len(), RAM_BASE, RAM_BASE as u64 + RAM_SIZE as u64
            );
            process::exit(1);
        }
        bus.ram.load(args.kernel_addr, &kernel);
        eprintln!("Loaded kernel ({} bytes) at 0x{:08x}", kernel.len(), args.kernel_addr);
    }

    // DTB loaded after kernel so it wins if they overlap.
    let dtb_loaded_at: Option<u32> = if let Some(path) = &args.dtb_path {
        let dtb = fs::read(path).unwrap_or_else(|e| {
            eprintln!("Failed to read DTB {}: {}", path, e);
            process::exit(1);
        });
        let end = (args.dtb_addr as u64).saturating_add(dtb.len() as u64);
        if args.dtb_addr < RAM_BASE || end > RAM_BASE as u64 + RAM_SIZE as u64 {
            eprintln!(
                "DTB does not fit in RAM: addr=0x{:08x} len=0x{:x} ram=[0x{:08x},0x{:08x})",
                args.dtb_addr, dtb.len(), RAM_BASE, RAM_BASE as u64 + RAM_SIZE as u64
            );
            process::exit(1);
        }
        bus.ram.load(args.dtb_addr, &dtb);
        Some(args.dtb_addr)
    } else {
        None
    };

    // Find tohost symbol
    let tohost_addr = elf
        .syms
        .iter()
        .find(|sym| {
            elf.strtab.get_at(sym.st_name) == Some("tohost")
        })
        .map(|sym| sym.st_value as u32);

    if tohost_addr.is_none() && dtb_loaded_at.is_none() {
        // Only warn for riscv-tests-style binaries; a real kernel boot won't have tohost.
        eprintln!("Warning: 'tohost' symbol not found in ELF");
    }

    let entry = (elf.entry as u32).wrapping_add(load_base);
    let mut hart = Hart::new(entry);

    // Standard RISC-V boot handoff: a0 = hartid, a1 = dtb pointer (or 0 if absent).
    // The "previous stage" here is the simulator itself; we set the regs that
    // OpenSBI / a Linux kernel head expects.
    hart.regs.set(10, args.hartid);
    hart.regs.set(11, dtb_loaded_at.unwrap_or(0));

    let max_cycles: u64 = args.max_cycles.unwrap_or(if args.kernel_path.is_some() {
        10_000_000_000
    } else {
        10_000_000
    });
    let trace = std::env::var("RVSIM_TRACE").is_ok();
    let sbi_log = std::env::var("RVSIM_SBI_LOG").is_ok();
    let mut cycle_count: u64 = 0;
    let mut last_pcs: [u32; 8] = [0; 8];
    let mut pc_idx: usize = 0;
    for _ in 0..max_cycles {
        bus.tick(cycle_count);
        hart.mtime = cycle_count;
        let pending = bus.pending_interrupts();
        if trace {
            eprintln!("pc=0x{:08x} priv={} mstatus=0x{:08x} mip=0x{:x} mie=0x{:x}",
                hart.pc, hart.priv_mode,
                hart.csrs.read_raw(rvsim_core::csr::CSR_MSTATUS),
                hart.csrs.read_raw(rvsim_core::csr::CSR_MIP),
                hart.csrs.read_raw(rvsim_core::csr::CSR_MIE));
        }
        let prev_priv = hart.priv_mode;
        let a6 = hart.regs.get(6);
        let a7 = hart.regs.get(7);
        let a0 = hart.regs.get(10);
        let a1 = hart.regs.get(11);
        let ecall_pc = hart.pc;
        last_pcs[pc_idx] = hart.pc;
        pc_idx = (pc_idx + 1) & 7;
        hart.step(&mut bus, pending);
        cycle_count += 1;
        if sbi_log && prev_priv == 1 && hart.priv_mode == 3 {
            eprintln!("[{}] SBI ecall @ 0x{:08x}: eid=0x{:08x} fid=0x{:08x} a0=0x{:08x} a1=0x{:08x}",
                cycle_count, ecall_pc, a7, a6, a0, a1);
        }

        // Check tohost after each step
        if let Some(addr) = tohost_addr {
            let val = bus.read32(addr).unwrap_or(0);
            if val != 0 {
                if val == 1 {
                    println!("PASS");
                    process::exit(0);
                } else {
                    let test_num = val >> 1;
                    eprintln!("FAIL: test case {} (tohost=0x{:08x})", test_num, val);
                    process::exit(1);
                }
            }
        }
    }

    eprintln!("TIMEOUT: exceeded {} cycles", max_cycles);
    eprintln!("  pc=0x{:08x}  priv={}  cycle={}",
        hart.pc, hart.priv_mode, hart.cycle);
    eprintln!("  mstatus=0x{:08x}  mip=0x{:08x}  mie=0x{:08x}",
        hart.csrs.read_raw(rvsim_core::csr::CSR_MSTATUS),
        hart.csrs.read_raw(rvsim_core::csr::CSR_MIP),
        hart.csrs.read_raw(rvsim_core::csr::CSR_MIE));
    eprintln!("  mideleg=0x{:08x}  sepc=0x{:08x}  scause=0x{:08x}",
        hart.csrs.read_raw(rvsim_core::csr::CSR_MIDELEG),
        hart.csrs.read_raw(rvsim_core::csr::CSR_SEPC),
        hart.csrs.read_raw(rvsim_core::csr::CSR_SCAUSE));
    eprint!("  last PCs:");
    for i in 0..8 {
        eprint!(" 0x{:08x}", last_pcs[(pc_idx + i) & 7]);
    }
    eprintln!();
    let dump_start = hart.pc & !0xF;
    eprint!("  bytes at 0x{:08x}:", dump_start);
    for off in 0..32u32 {
        let va = dump_start.wrapping_add(off);
        match hart.translate(&bus, va, rvsim_core::mmu::AccessType::Fetch) {
            Ok(pa) => match bus.read8(pa) {
                Ok(b) => eprint!(" {:02x}", b),
                Err(_) => eprint!(" ??"),
            },
            Err(_) => eprint!(" ??"),
        }
    }
    eprintln!();
    process::exit(1);
}
