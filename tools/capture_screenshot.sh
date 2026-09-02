#!/bin/bash
set -e

LOG="/tmp/leviathan_art.log"
PPM="/tmp/leviathan_art.ppm"
SOCK="/tmp/qemu-art-mon.sock"
OUT_IMG="/home/inchara/.gemini/antigravity-cli/brain/41035a9c-c632-4422-9829-924c8e88de92/leviathan_underwater_60fps.png"

rm -f "$LOG" "$PPM" "$SOCK"

KVM_OPT=""
if [ -w /dev/kvm ]; then
    KVM_OPT="-enable-kvm -cpu host"
fi

env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD \
qemu-system-x86_64 \
    $KVM_OPT \
    -machine q35 \
    -m 2G \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=build/OVMF_VARS.fd \
    -cdrom build/leviathan.iso \
    -display vnc=:99 \
    -monitor unix:"$SOCK",server,nowait \
    -serial file:"$LOG" \
    -daemonize

echo "Waiting for Leviathan 60 FPS swimming scenery to start..."
for i in {1..50}; do
    if [ -f "$LOG" ] && grep -q "Starting 60 FPS swimming scenery" "$LOG"; then
        echo "60 FPS scenery started after $((i * 500)) ms!"
        break
    fi
    sleep 0.5
done

# Allow the Leviathan to swim for 3 seconds (approx 180 frames)
sleep 3.0

echo "screendump $PPM" | socat - UNIX-CONNECT:"$SOCK"
sleep 0.5
echo "quit" | socat - UNIX-CONNECT:"$SOCK" || true

if [ -f "$PPM" ]; then
    convert "$PPM" "$OUT_IMG"
    cp "$OUT_IMG" ./leviathan_underwater.png
    echo "Screenshot saved to $OUT_IMG and ./leviathan_underwater.png"
else
    echo "Error: PPM not generated"
    exit 1
fi
