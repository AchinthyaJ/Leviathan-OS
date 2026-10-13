#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Ensure build directory exists
mkdir -p build

# Detect OVMF firmware
OVMF_CODE=""
for f in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/edk2/x64/OVMF_CODE.4m.fd /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd; do
    if [ -f "$f" ]; then
        OVMF_CODE="$f"
        break
    fi
done

OVMF_VARS_SRC=""
for f in /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/edk2/x64/OVMF_VARS.4m.fd /usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd; do
    if [ -f "$f" ]; then
        OVMF_VARS_SRC="$f"
        break
    fi
done

if [ -z "$OVMF_CODE" ] || [ -z "$OVMF_VARS_SRC" ]; then
    echo "Error: OVMF UEFI firmware not found on system." >&2
    exit 1
fi

OVMF_VARS="build/OVMF_VARS.fd"
cp -f "$OVMF_VARS_SRC" "$OVMF_VARS"

# Check KVM availability
KVM_OPT=""
if [ -w /dev/kvm ]; then
    KVM_OPT="-enable-kvm -cpu host"
fi

# Optional data disk
DATA_DISK_OPT=""
if [ -f "build/leviathan-data.img" ]; then
    DATA_DISK_OPT="-drive file=build/leviathan-data.img,format=raw,if=virtio"
fi

# Hyprland / Wayland requires virtio-gpu with GL acceleration (gives /dev/dri/renderD128)
# Tablet is used exclusively to avoid relative mouse cursor lock/freeze
exec env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD SDL_VIDEODRIVER=wayland \
    qemu-system-x86_64 \
        $KVM_OPT \
        -machine q35 \
        -m 8G \
        -smp 2 \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive if=pflash,format=raw,file="$OVMF_VARS" \
        -cdrom build/leviathan.iso \
        $DATA_DISK_OPT \
        -boot d \
        -vga none \
        -device virtio-gpu-gl-pci \
        -display sdl,gl=on,show-cursor=off \
        -nic user,model=virtio-net-pci \
        -device virtio-tablet-pci \
        -audiodev sdl,id=snd0 \
        -device intel-hda \
        -device hda-duplex,audiodev=snd0 \
        "$@"