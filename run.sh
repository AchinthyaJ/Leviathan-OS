#!/bin/sh
# Wrapper script to run Leviathan under QEMU with sanitized GTK and Snap environment

KVM_OPT=""
if [ -w /dev/kvm ]; then
    KVM_OPT="-enable-kvm -cpu host"
fi

exec env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD \
    qemu-system-x86_64 \
    $KVM_OPT \
    -machine q35 \
    -m 2G \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=build/OVMF_VARS.fd \
    -cdrom build/leviathan.iso \
    "$@"
