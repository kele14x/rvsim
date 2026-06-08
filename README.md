# RVSIM

A **R**ISC-**V** **sim**ulator (RV32GC + Sv32 MMU) in Rust that boots Linux.

## Run ISA Tests

### Prerequisite

Install the RISC-V cross-compiler toolchain.

#### Linux

On Linux the easiest way is to download the release binaries from <https://github.com/riscv-collab/riscv-gnu-toolchain>. Ensure you download the following 4 files:

- riscv32-elf-ubuntu-24.04-gcc.tar.xz
- riscv32-glibc-ubuntu-24.04-gcc.tar.xz
- riscv64-elf-ubuntu-24.04-gcc.tar.xz
- riscv64-glibc-ubuntu-24.04-gcc.tar.xz

Many distros ship a `riscv64-linux-gnu-` toolchain, but they often have issues.

Unzip them to a proper folder (for example `/opt/riscv`):

``` bash
tar xvf riscv32-elf-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv32-glibc-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv64-elf-ubuntu-24.04-gcc.tar.xz -C /opt
tar xvf riscv64-glibc-ubuntu-24.04-gcc.tar.xz -C /opt
```

Setup your environment variables:

``` bash
export PATH="/opt/riscv/bin:$PATH"
export RISCV="/opt/riscv"
```

You can add the above two lines to your `~/.bashrc` or `~/.zshrc`.

#### MacOS

On MacOS, there is a [brew package](https://github.com/riscv-software-src/homebrew-riscv) that ships the toolchain. However this toolchain was not able to compile OpenSBI. And I did not manage to compile it from source. Suggest you use a VM to run the Linux toolchain.

#### Windows

On Windows the easiest way is to use WSL and follow the Linux instructions.

### Build riscv-tests

Check out the **riscv-tests** repo:

``` bash
git submodule update --init --recursive --depth 1 tests/riscv-tests
cd tests/riscv-tests
```

Build it:

``` bash
autoconf
./configure --with-xlen=32 --prefix=$RISCV/target
make
```

Then copy the RISC-V ISA test binaries to folder **riscv-tests-bin**.

``` bash
cp isa/rv32ui-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32um-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32ua-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32uc-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32uf-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32ud-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32mi-* /path/to/rvsim/tests/riscv-tests-bin
cp isa/rv32si-* /path/to/rvsim/tests/riscv-tests-bin
```

### Run tests

From the project root, run:

``` bash
cargo build
cargo test
cargo run -- tests/riscv-tests-bin/rv32ui-p-add    # run a single riscv-test
make riscv-tests   # run all riscv-tests
```

---

## Compiling the Device Tree

You also need `dtc` (device-tree compiler):

```bash
# Debian / Ubuntu
sudo apt install device-tree-compiler
```

The DTB is compiled from `tests/device-tree/rvsim.dts`:

```bash
dtc -I dts -O dtb -o tests/device-tree-bin/rvsim.dtb tests/device-tree/rvsim.dts
```

## Building OpenSBI

Check out the **opensbi** submodule:

``` bash
git submodule update --init --depth 1 tests/opensbi
cd tests/opensbi
```

Build it:

``` bash
make CROSS_COMPILE=riscv64-unknown-linux-gnu- \
     PLATFORM=generic \
     PLATFORM_RISCV_XLEN=32 \
     PLATFORM_RISCV_ISA=rv32gc \
     FW_JUMP_ADDR=0x80400000 \
     FW_JUMP_FDT_ADDR=0x82200000 \
     -j$(nproc)
```

Copy the binary to **opensbi-bin** folder:

``` bash
cp build/platform/generic/firmware/fw_jump.elf /path/to/rvsim/tests/opensbi-bin/
```

## Building the Linux Kernel

### Prerequisites

You also need some standard build tools:

```bash
# Debian / Ubuntu
sudo apt install flex bison bc libssl-dev
```

### Get the Kernel source

The project includes a git submodule pointing to the **linux-6.12.y**
stable branch:

```bash
git submodule update --init --depth 1 tests/linux
cd tests/linux
```

> **Tip:** The Linux kernel repo is large — even a shallow clone can be
> slow on a poor connection. As an alternative you can download a tarball
> from [The Linux Kernel Archives](https://www.kernel.org/) (e.g.
> **linux-6.12.92.tar.xz**) and extract it into `tests/linux/`.

### Configure the Kernel

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

## Building the initramfs

The kernel needs a root filesystem. We provide two options: a minimal
hello-world init (for quick smoke tests) and a BusyBox-based initramfs
(for an interactive shell with real utilities).

### Option A: Minimal init (hello world)

`tests/initramfs/init.c` is a tiny C program that prints a message and
sleeps forever. Useful for testing that Linux boots to userspace.

```bash
cd tests/initramfs
riscv32-unknown-linux-gnu-gcc -march=rv32gc -mabi=ilp32d -static -o init init.c
```

Create a cpio archive using the kernel's `gen_init_cpio` and the
manifest file `tests/initramfs/minimal_list.txt`:

```bash
# Build gen_init_cpio (one-time):
cd tests/linux && make usr/gen_init_cpio && cd -

cd tests/initramfs
../../tests/linux/usr/gen_init_cpio minimal_list.txt > ../initramfs-bin/initramfs.cpio
```

### Option B: BusyBox initramfs (interactive shell)

This gives you a real shell (`/bin/sh`) and standard utilities (`ls`,
`cat`, `grep`, `vi`, `ps`, `dmesg`, etc.) after boot.

#### Automated build

Run the build script on a Linux machine with the RV32 toolchain:

```bash
git submodule update --init tests/busybox
cd tests/initramfs
./build.sh
```

This builds BusyBox from the `tests/busybox` submodule (1.37, statically
linked for RV32GC) and produces `tests/initramfs-bin/initramfs.cpio`.
You can override the compiler:

```bash
CROSS_COMPILE=riscv32-unknown-linux-gnu- ./build.sh
```

#### Manual build

```bash
# 1. Build BusyBox from the submodule
git submodule update --init tests/busybox
cd tests/busybox

make ARCH=riscv CROSS_COMPILE=riscv32-unknown-linux-gnu- defconfig
sed -i \
    -e 's/^# CONFIG_STATIC is not set$/CONFIG_STATIC=y/' \
    -e 's/^CONFIG_SHA1_HWACCEL=y$/# CONFIG_SHA1_HWACCEL is not set/' \
    -e 's/^CONFIG_SHA256_HWACCEL=y$/# CONFIG_SHA256_HWACCEL is not set/' \
    -e 's/^CONFIG_TC=y$/# CONFIG_TC is not set/' \
    .config
yes "" | make ARCH=riscv CROSS_COMPILE=riscv32-unknown-linux-gnu- oldconfig
make ARCH=riscv CROSS_COMPILE=riscv32-unknown-linux-gnu- -j$(nproc)

# 2. Copy binary and build cpio
cp busybox ../initramfs/busybox
cd ../initramfs

# Build gen_init_cpio if needed:
cd ../linux && make usr/gen_init_cpio && cd -

../linux/usr/gen_init_cpio initramfs_list.txt > ../initramfs-bin/initramfs.cpio
```

### Embed in the kernel

Both options above produce `tests/initramfs-bin/initramfs.cpio`. Update
the kernel `.config` to embed it and rebuild:

```bash
cd tests/linux
./scripts/config --set-str CONFIG_INITRAMFS_SOURCE \
    "$(realpath ../initramfs-bin/initramfs.cpio)"
make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j$(nproc)
cp arch/riscv/boot/Image ../linux-bin/Image
```

## Booting Linux

rvsim boots Linux via OpenSBI (fw_jump) → Linux kernel → userspace init.
You need three binaries plus a device-tree blob:

| Component          | File                                 | How to get                                    |
|--------------------|--------------------------------------|-----------------------------------------------|
| OpenSBI firmware   | `tests/opensbi-bin/fw_jump.elf`      | Build from source (see above)                 |
| Device tree blob   | `tests/device-tree-bin/rvsim.dtb`    | Compiled from `rvsim.dts` (see above)         |
| Linux kernel Image | `tests/linux-bin/Image`              | Build from source (see above)                 |
| initramfs (cpio)   | `tests/initramfs-bin/initramfs.cpio` | Build from source (see below)                 |

### Run

```bash
cargo run -- \
  tests/opensbi-bin/fw_jump.elf \
  --dtb tests/device-tree-bin/rvsim.dtb \
  --kernel tests/linux-bin/Image \
  --max-cycles 1000000000
```

Kernel output appears on stdout. Use `--max-cycles` to control the
simulation length (default 10 billion with `--kernel`).

### Debug environment variables

| Variable             | Effect                                                            |
|----------------------|-------------------------------------------------------------------|
| `RVSIM_TRACE=1`      | Print PC, privilege mode, and key CSRs every cycle (very verbose) |
| `RVSIM_UART_TRACE=1` | Log every UART register read/write                                |
| `RVSIM_SBI_LOG=1`    | Log SBI ecalls from S-mode to M-mode                              |
