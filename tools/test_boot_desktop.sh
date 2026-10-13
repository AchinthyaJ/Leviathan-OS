#!/bin/bash
set -e

LOG="/tmp/leviathan_boot.log"
PPM="/tmp/leviathan_boot.ppm"
SOCK="/tmp/qemu_test_mon.sock"
OUT_IMG="/home/inchara/.gemini/antigravity-cli/brain/5679184d-3ac5-4c63-85b3-e0e9b23f56c0/desktop_screenshot.png"

rm -f "$LOG" "$PPM" "$SOCK"
mkdir -p build
cp -f /usr/share/OVMF/OVMF_VARS_4M.fd build/OVMF_VARS.fd

KVM_OPT=""
if [ -w /dev/kvm ]; then
    KVM_OPT="-enable-kvm -cpu host"
fi

echo "Launching QEMU..."
env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD \
qemu-system-x86_64 \
    $KVM_OPT \
    -machine q35 \
    -m 4G \
    -boot d \
    -vga std \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=build/OVMF_VARS.fd \
    -cdrom build/leviathan.iso \
    -display vnc=:98 \
    -monitor unix:"$SOCK",server,nowait \
    -serial file:"$LOG" \
    -daemonize

echo "Waiting for Leviathan UI Desktop to start..."
for i in $(seq 1 40); do
    if [ -f "$LOG" ] && grep -q "Starting Leviathan Desktop Environment" "$LOG"; then
        echo "Leviathan Desktop GUI started after $((i * 500)) ms!"
        break
    fi
    sleep 0.5
done

# Wait 2 seconds for desktop and test window to render
sleep 2.0

python3 -c "
import socket, time
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect('$SOCK')
s.sendall(b'screendump $PPM\n')
time.sleep(0.5)
s.sendall(b'quit\n')
s.close()
" || true

if [ -f "$PPM" ]; then
    convert "$PPM" "$OUT_IMG"
    echo "Screenshot successfully saved to $OUT_IMG"
    ls -lh "$OUT_IMG"
else
    echo "PPM not found, log contents:"
    cat "$LOG" | tail -n 30
    exit 1
fi
