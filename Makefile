CC = gcc
CFLAGS = -Wall -Wextra -O2 -static

# Executable sources (exclude library modules like graphics.c)
EXCLUDE_SRCS = init/graphics.c
SRCS = $(filter-out $(EXCLUDE_SRCS), $(wildcard init/*.c))
BINS = $(patsubst init/%.c,rootfs/bin/%,$(SRCS))

IMAGE_INITRAMFS = image/boot/initramfs.img
BUILD_INITRAMFS = build/initramfs.img
BUILD_ISO = build/leviathan.iso

OVMF_CODE = /usr/share/OVMF/OVMF_CODE_4M.fd
OVMF_VARS = build/OVMF_VARS.fd

.PHONY: all bins initramfs iso clean run run-serial run-headless

all: iso

RUST_TARGET = x86_64-unknown-linux-musl
RUST_BIN = ui/target/$(RUST_TARGET)/release/leviathan-ui

bins: $(BINS) rootfs/bin/leviathan-ui

rootfs/bin/leviathan-ui: $(wildcard ui/src/*.rs) ui/Cargo.toml
	@mkdir -p rootfs/bin
	cargo build --release --target $(RUST_TARGET) --manifest-path ui/Cargo.toml
	cp $(RUST_BIN) rootfs/bin/leviathan-ui
	@ln -sf leviathan-ui rootfs/bin/leviathan-art

rootfs/bin/leviathan_art: init/leviathan_art.c init/graphics.c init/graphics.h init/font8x8.h
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) init/leviathan_art.c init/graphics.c -lm -o $@

rootfs/bin/%: init/%.c
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) $< -o $@

initramfs: bins
	@mkdir -p build
	cd rootfs && find . -print0 | cpio --null -ov --format=newc | gzip -9 > ../$(BUILD_INITRAMFS)
	cp $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS)

iso: initramfs
	@mkdir -p build
	grub-mkrescue -o $(BUILD_ISO) image

clean:
	rm -f $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS) $(BUILD_ISO)

CLEAN_ENV = env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD
KVM_FLAGS = $(shell test -w /dev/kvm && echo "-enable-kvm -cpu host")

run: iso
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 2G \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO)

run-serial: iso
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 2G \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-serial stdio

run-headless: iso
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 2G \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-display vnc=:99 \
		-serial stdio

