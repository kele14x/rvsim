# rvsim

A RISC-V simulator (RV32GC + Sv32 MMU) in Rust that boots Linux.

## Run ISA Tests

### Install the necessary RISC-V toolchain

Install the RISC-V cross-compiler toolchain.

On linux the easiest way is download the release binary from <https://github.com/riscv-collab/riscv-gnu-toolchain>. Ensure you downloaded the following 4 files:

- riscv32-elf-ubuntu-24.04-gcc.tar.xz
- riscv32-glibc-ubuntu-24.04-gcc.tar.xz
- riscv64-elf-ubuntu-24.04-gcc.tar.xz
- riscv64-glibc-ubuntu-24.04-gcc.tar.xz

Many distros ship a `riscv64-linux-gnu-` toolchain, but more or less with some issue.

Unzip them to a proper folder (for example `/opt/riscv`):

``` bash
tar xvf riscv32-elf-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv32-glibc-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv64-elf-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv64-glibc-ubuntu-24.04-gcc.tar.xz -C /opt
```

Setup your path:

``` bash
export PATH="/opt/riscv/bin:$PATH"
```

### Build riscv-tests

Checkout the **riscv-tests** repo.

``` bash
git submodule update --init --recursive --depth 1 tests/riscv-tests
```



## Run individual tests

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

You also need `dtc` (device-tree compiler) and standard build tools:

```bash
# Debian / Ubuntu
sudo apt install device-tree-compiler flex bison bc libssl-dev
```

### Get and configure Kernel

Download a tarball from [The Linux Kernel Archives](https://www.kernel.org/). For example **linux-6.12.92.tar.xz**.

Start from the default RISC-V 32-bit defconfig, then apply the options
rvsim needs. A minimal `.config` can be produced with:

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- rv32_defconfig
```

Optional:

Tweak the config — open `.config` in an editor or use `menuconfig`:

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- menuconfig
```

### Build

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
```

The kernel Image will be at:

```plaintext
arch/riscv/boot/Image
```

We does not ship a initramfs with it currently. But it's ok for now.

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
riscv32-unknown-linux-gnu-gcc -march=rv32gc -mabi=ilp32d -static -o init init.c
```

Verify it is a static RV32 binary:

```bash
file init
# Expected: ELF 32-bit LSB executable, UCB RISC-V, ... statically linked ...
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

```bash
make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
```

Copy it into the repo:

```bash
cp arch/riscv/boot/Image /path/to/rvsim/tests/opensbi-bin/Image
```

## Building OpenSBI

A pre-built `fw_jump.elf` is checked into `tests/opensbi-bin/`. If you
need to rebuild it:

```bash
git clone --depth 1 https://github.com/riscv-software-src/opensbi.git
cd opensbi

make CROSS_COMPILE=riscv64-unknown-linux-gnu- \
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
