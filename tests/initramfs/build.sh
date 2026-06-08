#!/bin/bash
#
# Build a BusyBox-based initramfs for rvsim.
# Run this on a Linux machine with riscv32-unknown-linux-gnu-gcc installed.
#
# Usage:
#   cd tests/initramfs && ./build.sh
#
# Prerequisites:
#   - riscv32-unknown-linux-gnu- toolchain
#   - BusyBox submodule initialized (tests/busybox)
#   - Linux kernel source at tests/linux (for gen_init_cpio)
#
# Output:
#   tests/initramfs-bin/initramfs.cpio

set -euo pipefail

CROSS_COMPILE="${CROSS_COMPILE:-riscv32-unknown-linux-gnu-}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUSYBOX_SRC="$REPO_ROOT/tests/busybox"
BUILD_DIR="$SCRIPT_DIR/_build"
OUTPUT_DIR="$REPO_ROOT/tests/initramfs-bin"
LINUX_DIR="$REPO_ROOT/tests/linux"

echo "=== BusyBox initramfs builder for rvsim ==="
echo "Cross compiler:  $CROSS_COMPILE"
echo ""

# Check prerequisites
if ! command -v "${CROSS_COMPILE}gcc" &>/dev/null; then
    echo "ERROR: ${CROSS_COMPILE}gcc not found in PATH"
    echo "Install a riscv32 toolchain or set CROSS_COMPILE=..."
    exit 1
fi

if [ ! -f "$BUSYBOX_SRC/Makefile" ]; then
    echo "ERROR: BusyBox submodule not initialized at $BUSYBOX_SRC"
    echo "Run: git submodule update --init tests/busybox"
    exit 1
fi

# Step 1: Configure BusyBox (out-of-tree build)
echo "--- Configuring BusyBox ---"
if [ -f "$BUSYBOX_SRC/include/autoconf.h" ]; then
    make -C "$BUSYBOX_SRC" mrproper >/dev/null 2>&1
fi
mkdir -p "$BUILD_DIR"
yes "" | make -C "$BUSYBOX_SRC" O="$BUILD_DIR" ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" defconfig >/dev/null 2>&1 || true

# Apply our overrides via sed (flip existing =y to disabled, enable disabled)
sed -i \
    -e 's/^# CONFIG_STATIC is not set$/CONFIG_STATIC=y/' \
    -e 's/^# CONFIG_FEATURE_PREFER_APPLETS is not set$/CONFIG_FEATURE_PREFER_APPLETS=y/' \
    -e 's/^# CONFIG_FEATURE_SH_STANDALONE is not set$/CONFIG_FEATURE_SH_STANDALONE=y/' \
    -e 's/^CONFIG_SHA1_HWACCEL=y$/# CONFIG_SHA1_HWACCEL is not set/' \
    -e 's/^CONFIG_SHA256_HWACCEL=y$/# CONFIG_SHA256_HWACCEL is not set/' \
    -e 's/^CONFIG_TC=y$/# CONFIG_TC is not set/' \
    -e 's/^CONFIG_IFCONFIG=y$/# CONFIG_IFCONFIG is not set/' \
    -e 's/^CONFIG_ROUTE=y$/# CONFIG_ROUTE is not set/' \
    -e 's/^CONFIG_IP=y$/# CONFIG_IP is not set/' \
    -e 's/^CONFIG_PING=y$/# CONFIG_PING is not set/' \
    -e 's/^CONFIG_WGET=y$/# CONFIG_WGET is not set/' \
    -e 's/^CONFIG_NETSTAT=y$/# CONFIG_NETSTAT is not set/' \
    -e 's/^CONFIG_FEATURE_IPV6=y$/# CONFIG_FEATURE_IPV6 is not set/' \
    -e 's/^CONFIG_FEATURE_IFUPDOWN_IP=y$/# CONFIG_FEATURE_IFUPDOWN_IP is not set/' \
    -e 's/^CONFIG_INIT=y$/# CONFIG_INIT is not set/' \
    -e 's/^CONFIG_LINUXRC=y$/# CONFIG_LINUXRC is not set/' \
    -e 's/^CONFIG_FEATURE_USE_INITTAB=y$/# CONFIG_FEATURE_USE_INITTAB is not set/' \
    -e 's/^CONFIG_MODPROBE=y$/# CONFIG_MODPROBE is not set/' \
    -e 's/^CONFIG_RMMOD=y$/# CONFIG_RMMOD is not set/' \
    -e 's/^CONFIG_INSMOD=y$/# CONFIG_INSMOD is not set/' \
    -e 's/^CONFIG_LSMOD=y$/# CONFIG_LSMOD is not set/' \
    -e 's/^CONFIG_MODINFO=y$/# CONFIG_MODINFO is not set/' \
    -e 's/^CONFIG_DEPMOD=y$/# CONFIG_DEPMOD is not set/' \
    -e 's/^CONFIG_FEATURE_2_4_MODULES=y$/# CONFIG_FEATURE_2_4_MODULES is not set/' \
    "$BUILD_DIR/.config"
yes "" | make -C "$BUILD_DIR" ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" oldconfig >/dev/null 2>&1 || true

# Step 2: Build
echo "--- Building BusyBox (this may take a few minutes) ---"
make -C "$BUILD_DIR" ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" -j"$(nproc)" 2>&1 | tail -1

# Verify the binary
BUSYBOX_BIN="$BUILD_DIR/busybox"
if [ ! -f "$BUSYBOX_BIN" ]; then
    echo "ERROR: BusyBox build failed — no busybox binary produced"
    exit 1
fi
echo "Built: $(file "$BUSYBOX_BIN" | cut -d: -f2)"

# Step 3: Copy binary to initramfs source directory
cp "$BUSYBOX_BIN" "$SCRIPT_DIR/busybox"

# Step 4: Build cpio archive
echo "--- Building initramfs cpio ---"
mkdir -p "$OUTPUT_DIR"

GEN_INIT_CPIO="$LINUX_DIR/usr/gen_init_cpio"
if [ -x "$GEN_INIT_CPIO" ]; then
    echo "Using gen_init_cpio from kernel tree"
    cd "$SCRIPT_DIR"
    "$GEN_INIT_CPIO" initramfs_list.txt > "$OUTPUT_DIR/initramfs.cpio"
else
    echo "gen_init_cpio not found at $GEN_INIT_CPIO"
    echo "Building it from the kernel source..."
    if [ -f "$LINUX_DIR/usr/gen_init_cpio.c" ]; then
        gcc -o "$GEN_INIT_CPIO" "$LINUX_DIR/usr/gen_init_cpio.c"
        cd "$SCRIPT_DIR"
        "$GEN_INIT_CPIO" initramfs_list.txt > "$OUTPUT_DIR/initramfs.cpio"
    else
        echo "ERROR: Linux source tree not found at $LINUX_DIR"
        echo "Initialize the submodule: git submodule update --init tests/linux"
        echo ""
        echo "Alternatively, build the cpio manually:"
        echo "  cd tests/linux && make usr/gen_init_cpio"
        echo "  usr/gen_init_cpio ../initramfs/initramfs_list.txt > ../initramfs-bin/initramfs.cpio"
        exit 1
    fi
fi

# Clean up the busybox binary from source dir (it's in the cpio now)
rm -f "$SCRIPT_DIR/busybox"

CPIO_SIZE=$(du -h "$OUTPUT_DIR/initramfs.cpio" | cut -f1)
echo ""
echo "=== Done ==="
echo "Output: $OUTPUT_DIR/initramfs.cpio ($CPIO_SIZE)"
echo ""
echo "Next steps:"
echo "  1. Embed in kernel:"
echo "     cd tests/linux"
echo "     ./scripts/config --set-str CONFIG_INITRAMFS_SOURCE \"$OUTPUT_DIR/initramfs.cpio\""
echo "     make ARCH=riscv CROSS_COMPILE=riscv64-unknown-linux-gnu- -j\$(nproc)"
echo "     cp arch/riscv/boot/Image ../linux-bin/Image"
echo "  2. Boot with rvsim:"
echo "     cargo run --release -- tests/opensbi-bin/fw_jump.elf --dtb tests/device-tree-bin/rvsim.dtb --kernel tests/linux-bin/Image"
