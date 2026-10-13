CC = gcc
CFLAGS = -Wall -Wextra -O2 -static

# Executable sources (exclude library modules like graphics.c)
EXCLUDE_SRCS = init/graphics.c
SRCS = $(filter-out $(EXCLUDE_SRCS), $(wildcard init/*.c))
BINS = $(patsubst init/%.c,rootfs/bin/%,$(SRCS))

IMAGE_INITRAMFS = image/boot/initramfs.img
BUILD_INITRAMFS = build/initramfs.img
BUILD_ISO = build/leviathan.iso

OVMF_CODE ?= $(shell for f in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/edk2/x64/OVMF_CODE.4m.fd /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd; do test -f "$$f" && echo "$$f" && break; done)
OVMF_VARS_SRC ?= $(shell for f in /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/edk2/x64/OVMF_VARS.4m.fd /usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd; do test -f "$$f" && echo "$$f" && break; done)
OVMF_VARS = build/OVMF_VARS.fd

.PHONY: all bins initramfs iso clean run run-serial run-headless

all: iso

bins: $(BINS)

rootfs/bin/%: init/%.c
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) $< -o $@

ROOTFS_RELEASE_URL ?= https://github.com/AchinthyaJ/Leviathan-OS/releases/latest/download/rootfs.tar.zst
ROOTFS_ARCHIVE ?= build/rootfs.tar.zst

pack-rootfs:
	@mkdir -p build
	@echo "Packing rootfs into $(ROOTFS_ARCHIVE)..."
	tar --zstd -cf $(ROOTFS_ARCHIVE) -C rootfs .
	@echo "Created $(ROOTFS_ARCHIVE) (Upload this asset to GitHub Releases)"

fetch-rootfs:
	@mkdir -p build rootfs
	@echo "Fetching Leviathan rootfs from $(ROOTFS_RELEASE_URL)..."
	@if command -v curl >/dev/null 2>&1; then \
		curl -fsSL -o $(ROOTFS_ARCHIVE) $(ROOTFS_RELEASE_URL); \
	else \
		wget -q -O $(ROOTFS_ARCHIVE) $(ROOTFS_RELEASE_URL); \
	fi
	@echo "Extracting rootfs archive..."
	tar --zstd -xf $(ROOTFS_ARCHIVE) -C rootfs
	@echo "Rootfs populated successfully."

initramfs:
	@mkdir -p build image/boot
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	@if [ ! -d rootfs/usr ] || [ ! -f rootfs/init ]; then \
		echo "rootfs is missing. Run 'make fetch-rootfs' first."; \
		exit 1; \
	fi
	cd rootfs && find . -print0 | cpio --null -ov --format=newc | zstd -3 -T0 > ../$(BUILD_INITRAMFS)
	cp $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS)

iso: initramfs
	@mkdir -p build
	grub-mkrescue -o $(BUILD_ISO) image

clean:
	rm -f $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS) $(BUILD_ISO)

CLEAN_ENV = env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD SDL_VIDEODRIVER=wayland
KVM_FLAGS = $(shell test -w /dev/kvm && echo "-enable-kvm -cpu host")

run: iso
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display sdl,gl=on,show-cursor=off \
		-nic user,model=virtio-net-pci \
		-device virtio-tablet-pci \
		-audiodev sdl,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0

run-serial: iso
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display sdl,gl=on,show-cursor=off \
		-nic user,model=virtio-net-pci \
		-device virtio-tablet-pci \
		-audiodev sdl,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0 \
		-serial stdio

run-headless: iso
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display egl-headless \
		-vnc :99 \
		-nic user,model=virtio-net-pci \
		-device virtio-tablet-pci \
		-audiodev none,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0 \
		-serial stdio

