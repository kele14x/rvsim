use std::env;
use std::fs;
use std::process;

use goblin::elf::Elf;
use rvsim_core::cpu::Hart;
use rvsim_core::mem::Memory;
use rvsim_mem::bus::Bus;
use rvsim_mem::clint::Clint;
use rvsim_mem::flat::FlatMemory;
use rvsim_mem::uart::Uart;

const RAM_BASE: u32 = 0x8000_0000;
const RAM_SIZE: usize = 128 * 1024 * 1024; // 128 MB

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: rvsim <elf-binary>");
        process::exit(1);
    }

    let elf_data = fs::read(&args[1]).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", args[1], e);
        process::exit(1);
    });

    let elf = Elf::parse(&elf_data).unwrap_or_else(|e| {
        eprintln!("Failed to parse ELF: {}", e);
        process::exit(1);
    });

    let ram = FlatMemory::new(RAM_SIZE, RAM_BASE);
    let mut bus = Bus::new(ram, Clint::new(), Uart::stdout());

    // Load segments
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let file_offset = ph.p_offset as usize;
            let file_size = ph.p_filesz as usize;
            let vaddr = ph.p_paddr as u32;
            if file_size > 0 {
                bus.ram.load(vaddr, &elf_data[file_offset..file_offset + file_size]);
            }
        }
    }

    // Find tohost symbol
    let tohost_addr = elf
        .syms
        .iter()
        .find(|sym| {
            elf.strtab.get_at(sym.st_name).map_or(false, |name| name == "tohost")
        })
        .map(|sym| sym.st_value as u32);

    if tohost_addr.is_none() {
        eprintln!("Warning: 'tohost' symbol not found in ELF");
    }

    let entry = elf.entry as u32;
    let mut hart = Hart::new(entry);

    let max_cycles: u64 = 10_000_000;
    for _ in 0..max_cycles {
        bus.tick(hart.cycle);
        let pending = bus.pending_interrupts();
        hart.step(&mut bus, pending);

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
    process::exit(1);
}
