# rvsim

A RISC-V simulator (RV32GC + Sv32 MMU) in Rust that boots Linux.

## Quick Start

```bash
cargo build
cargo test
cargo run -- tests/rvsim-tests-bin/rv32ui-p-add    # run a single riscv-test
```

## Booting Linux

rvsim boots Linux via OpenSBI (fw_jump) → Linux kernel → userspace init.
You need three binaries plus a device-tree blob:

| Component | File | How to get |
|-----------|------|------------|
| OpenSBI firmware | `tests/opensbi-bin/fw_jump.elf` | Checked into repo |
| Device tree blob | `tests/opensbi-bin/rvsim.dtb` | Checked into repo (compiled from `rvsim.dts`) |
| Linux kernel Image | `tests/opensbi-bin/Image` | Build from source (see below) |
| initramfs (cpio) | your `initramfs.cpio` | Build from source (see below) |

### Run

```bash
cargo run -- \
  tests/opensbi-bin/fw_jump.elf \
  --dtb tests/opensbi-bin/rvsim.dtb \
  --kernel tests/opensbi-bin/Image \
  --max-cycles 2000000000
```

Kernel output appears on stdout. Use `--max-cycles` to control the
simulation length (default 10 billion with `--kernel`).

### Debug environment variables

| Variable | Effect |
|----------|--------|
| `RVSIM_TRACE=1` | Print PC, privilege mode, and key CSRs every cycle (very verbose) |
| `RVSIM_UART_TRACE=1` | Log every UART register read/write |
| `RVSIM_SBI_LOG=1` | Log SBI ecalls from S-mode to M-mode |

---

## Building the Linux Kernel

### Prerequisites

Install a RISC-V 32-bit cross-compiler toolchain. The exact package name
depends on your distro:

```bash
# Debian / Ubuntu
sudo apt install gcc-riscv64-linux-gnu

# Fedora
sudo dnf install gcc-riscv64-linux-gnu

# macOS (Homebrew) — use the riscv64 toolchain; it supports rv32 via -march
brew install riscv64-elf-gcc

# Or build from source: https://github.com/riscv-collab/riscv-gnu-toolchain
# Configure with: ./configure --prefix=/opt/riscv --with-arch=rv32gc --with-abi=ilp32d
```

> **Note:** Most distros only ship a `riscv64-linux-gnu-` toolchain. That
> toolchain works fine for building an RV32 kernel — set `ARCH=riscv` and
> the kernel's Kconfig/Makefile handles the 32-bit configuration.

You also need `dtc` (device-tree compiler) and standard build tools:

```bash
# Debian / Ubuntu
sudo apt install device-tree-compiler flex bison bc libssl-dev

# macOS
brew install dtc
```

### Clone and configure

```bash
git clone --depth 1 https://github.com/torvalds/linux.git
cd linux
```

Start from the default RISC-V 32-bit defconfig, then apply the options
rvsim needs. A minimal `.config` can be produced with:

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- rv32_defconfig
```

Then tweak the config — open `.config` in an editor or use `menuconfig`:

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- menuconfig
```

The essential settings for rvsim:

```
# Already set by rv32_defconfig:
CONFIG_ARCH_RV32I=y
CONFIG_32BIT=y
CONFIG_MMU=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_SERIAL_OF_PLATFORM=y
CONFIG_TTY=y

# Make sure these are enabled:
CONFIG_RISCV_SBI=y              # SBI support (OpenSBI interface)
CONFIG_RISCV_SBI_V01=y          # SBI v0.1 legacy extensions
CONFIG_BLK_DEV_INITRD=y         # initramfs support

# Disable things rvsim doesn't have (speeds up boot):
# CONFIG_SMP is not set          # single hart
# CONFIG_NET is not set           # no NIC
# CONFIG_SOUND is not set         # no soundcard
# CONFIG_USB_SUPPORT is not set
# CONFIG_WLAN is not set
# CONFIG_WIRELESS is not set
```

### Build

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- -j$(nproc)
```

The kernel Image will be at:

```
arch/riscv/boot/Image
```

Copy it into the repo:

```bash
cp arch/riscv/boot/Image /path/to/rvsim/tests/opensbi-bin/Image
```

---

## Building the initramfs

The kernel needs a root filesystem. The simplest approach is a cpio
archive containing a statically linked `/init` program.

### 1. Write a minimal init program

```c
// init.c
#include <stdio.h>
#include <unistd.h>

int main(void) {
    printf("Hello from Linux on rvsim!\n");
    // Keep init alive — the kernel panics if init exits.
    for (;;)
        sleep(1);
    return 0;
}
```

Cross-compile it **statically** for RV32:

```bash
riscv64-linux-gnu-gcc -march=rv32gc -mabi=ilp32d -static -o init init.c
```

Verify it is a static RV32 binary:

```bash
file init
# Expected: ELF 32-bit LSB executable, UCB RISC-V, ... statically linked
```

### 2. Create the cpio archive

The archive must include `/dev/console` as a character device node
(major 5, minor 1). There are two methods:

#### Method A: using the kernel's gen_init_cpio (no root required)

```bash
cat > initramfs_list.txt << 'EOF'
dir  /dev        0755 0 0
nod  /dev/console 0600 0 0 c 5 1
file /init       init 0755 0 0
EOF

# Build gen_init_cpio from the kernel source tree (one-time):
cd /path/to/linux
make usr/gen_init_cpio

# Generate the archive:
usr/gen_init_cpio /path/to/initramfs_list.txt > /path/to/initramfs.cpio
```

#### Method B: using cpio directly (requires root for mknod)

```bash
mkdir -p /tmp/initramfs/dev
sudo mknod /tmp/initramfs/dev/console c 5 1
cp init /tmp/initramfs/init
chmod +x /tmp/initramfs/init

cd /tmp/initramfs
find . | cpio -o -H newc > /path/to/initramfs.cpio
```

### 3. Embed in the kernel or load separately

The simplest approach is to embed the initramfs into the kernel Image at
build time. Add this to your kernel `.config` before building:

```
CONFIG_INITRAMFS_SOURCE="/absolute/path/to/initramfs.cpio"
```

Then rebuild the kernel — the Image will contain the initramfs.

---

## Building OpenSBI

A pre-built `fw_jump.elf` is checked into `tests/opensbi-bin/`. If you
need to rebuild it:

```bash
git clone --depth 1 https://github.com/riscv-software-src/opensbi.git
cd opensbi

make CROSS_COMPILE=riscv64-linux-gnu- \
     PLATFORM=generic \
     PLATFORM_RISCV_XLEN=32 \
     PLATFORM_RISCV_ISA=rv32gc \
     FW_JUMP_ADDR=0x80400000 \
     FW_JUMP_FDT_ADDR=0x82200000 \
     -j$(nproc)

cp build/platform/generic/firmware/fw_jump.elf /path/to/rvsim/tests/opensbi-bin/
```

## Recompiling the Device Tree

The DTB is compiled from `tests/opensbi-bin/rvsim.dts`:

```bash
dtc -I dts -O dtb -o tests/opensbi-bin/rvsim.dtb tests/opensbi-bin/rvsim.dts
```
