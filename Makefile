CC = gcc
CFLAGS = -Wall -Wextra -O2 -static

# Executable sources (exclude library modules like graphics.c, and multi-file binaries)
# changepass needs -lcrypt so it has its own rule below
EXCLUDE_SRCS = init/graphics.c init/changepass.c init/bootsplash.c init/leviathan_art.c
SRCS = $(filter-out $(EXCLUDE_SRCS), $(wildcard init/*.c))
BINS = $(patsubst init/%.c,rootfs/bin/%,$(SRCS))

IMAGE_INITRAMFS = image/boot/initramfs.img
BUILD_INITRAMFS = build/initramfs.img
BUILD_ISO = build/leviathan.iso

OVMF_CODE ?= $(shell for f in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/edk2/x64/OVMF_CODE.4m.fd /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd; do test -f "$$f" && echo "$$f" && break; done)
OVMF_VARS_SRC ?= $(shell for f in /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/edk2/x64/OVMF_VARS.4m.fd /usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd; do test -f "$$f" && echo "$$f" && break; done)
OVMF_VARS = build/OVMF_VARS.fd

.PHONY: all bins initramfs iso clean clean-data run run-serial run-headless pack-rootfs fetch-rootfs

all: iso

bins: $(BINS) rootfs/bin/bootsplash rootfs/bin/leviathan_art rootfs/bin/changepass
	@mkdir -p rootfs/usr/sbin rootfs/usr/bin
	cp -f rootfs/bin/poweroff rootfs/usr/sbin/poweroff
	ln -sfn poweroff rootfs/bin/reboot
	ln -sfn poweroff rootfs/usr/sbin/reboot

rootfs/bin/bootsplash: init/bootsplash.c init/graphics.c
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) init/bootsplash.c init/graphics.c -o $@ -lm

rootfs/bin/leviathan_art: init/leviathan_art.c init/graphics.c
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) init/leviathan_art.c init/graphics.c -o $@ -lm

rootfs/bin/%: init/%.c
	@mkdir -p rootfs/bin
	$(CC) $(CFLAGS) $< -o $@

# changepass needs -lcrypt for crypt_r (not available in full static on all systems,
# so we use dynamic linking here — it runs inside the rootfs which has glibc).
rootfs/bin/changepass: init/changepass.c
	@mkdir -p rootfs/bin
	$(CC) -Wall -Wextra -O2 init/changepass.c -o $@ -lcrypt

rootfs/usr/bin/changepass: rootfs/bin/changepass
	@mkdir -p rootfs/usr/bin
	cp -f rootfs/bin/changepass rootfs/usr/bin/changepass

ROOTFS_RELEASE_URL ?= https://github.com/AchinthyaJ/Leviathan-OS/releases/latest/download/rootfs.tar.zst
ROOTFS_ARCHIVE ?= build/rootfs.tar.zst

# Excludes for clean packaging (caches, temp files, logs, docs)
ROOTFS_EXCLUDES = \
	--exclude='./tmp/*' \
	--exclude='./var/tmp/*' \
	--exclude='./var/log/*' \
	--exclude='./var/cache/apt/archives/*.deb' \
	--exclude='./var/cache/apt/*.bin' \
	--exclude='./var/lib/apt/lists/*' \
	--exclude='./root/.cache/*' \
	--exclude='./usr/share/doc/*' \
	--exclude='./usr/share/man/*' \
	--exclude='./mnt/*' \
	--exclude='./run/*' \
	--exclude='./proc/*' \
	--exclude='./sys/*' \
	--exclude='./dev/*' \
	--exclude='*.old' \
	--exclude='*.bak' \
	--exclude='*.tmp' \
	--exclude='.dockerenv'

pack-rootfs:
	@mkdir -p build
	@echo "Packing clean rootfs with ultra compression (-19) into $(ROOTFS_ARCHIVE)..."
	tar $(ROOTFS_EXCLUDES) -cf - -C rootfs . | zstd -19 -T0 > $(ROOTFS_ARCHIVE)
	@echo "Created $(ROOTFS_ARCHIVE) ($$(ls -lh $(ROOTFS_ARCHIVE) | awk '{print $$5}'))"

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

initramfs: bins
	@mkdir -p build image/boot
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	@if [ ! -d rootfs/usr ] || [ ! -f rootfs/init ]; then \
		echo "rootfs is missing. Run 'make fetch-rootfs' first."; \
		exit 1; \
	fi
	cd rootfs && find . \
		-path './tmp' -prune -o \
		-path './var/tmp' -prune -o \
		-path './var/log' -prune -o \
		-path './var/cache/apt/archives' -prune -o \
		-path './var/lib/apt/lists' -prune -o \
		-path './root/.cache' -prune -o \
		-path './mnt' -prune -o \
		-path './run' -prune -o \
		-name '*.old' -prune -o \
		-name '*.bak' -prune -o \
		-name '.dockerenv' -prune -o \
		-print0 | cpio -R 0:0 --null -ov --format=newc | zstd -6 -T0 > ../$(BUILD_INITRAMFS)


	cp $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS)

iso: initramfs
	@mkdir -p build
	grub-mkrescue -o $(BUILD_ISO) image

DATA_DISK ?= build/leviathan_data.img
DATA_DISK_SIZE ?= 10G

$(DATA_DISK):
	@mkdir -p build
	@echo "Creating persistent Leviathan data disk ($(DATA_DISK_SIZE))..."
	truncate -s $(DATA_DISK_SIZE) $(DATA_DISK)
	mkfs.ext4 -F -L LEVIATHAN_DATA $(DATA_DISK)

clean:
	rm -f $(BUILD_INITRAMFS) $(IMAGE_INITRAMFS) $(BUILD_ISO)

clean-data:
	rm -f $(DATA_DISK)

CLEAN_ENV = env -u GTK_PATH -u GTK_EXE_PREFIX -u GIO_MODULE_DIR -u GTK_IM_MODULE_FILE -u GSETTINGS_SCHEMA_DIR -u LD_LIBRARY_PATH -u LD_PRELOAD SDL_VIDEODRIVER=wayland
KVM_FLAGS = $(shell test -w /dev/kvm && echo "-enable-kvm -cpu host")

run: iso $(DATA_DISK)
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-drive file=$(DATA_DISK),format=raw,if=virtio \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display sdl,gl=on,show-cursor=off \
		-nic user,model=virtio-net-pci \
		-virtfs local,path=.,mount_tag=hostshare,security_model=none \
		-device virtio-tablet-pci \
		-audiodev sdl,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0

run-serial: iso $(DATA_DISK)
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-drive file=$(DATA_DISK),format=raw,if=virtio \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display sdl,gl=on,show-cursor=off \
		-nic user,model=virtio-net-pci \
		-virtfs local,path=.,mount_tag=hostshare,security_model=none \
		-device virtio-tablet-pci \
		-audiodev sdl,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0 \
		-serial stdio

run-headless: iso $(DATA_DISK)
	@test -f $(OVMF_VARS) || cp -f $(OVMF_VARS_SRC) $(OVMF_VARS)
	$(CLEAN_ENV) qemu-system-x86_64 \
		$(KVM_FLAGS) \
		-machine q35 \
		-m 8G \
		-smp 2 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-cdrom $(BUILD_ISO) \
		-drive file=$(DATA_DISK),format=raw,if=virtio \
		-boot d \
		-vga none \
		-device virtio-gpu-gl-pci \
		-display egl-headless \
		-vnc :99 \
		-nic user,model=virtio-net-pci \
		-virtfs local,path=.,mount_tag=hostshare,security_model=none \
		-device virtio-tablet-pci \
		-audiodev none,id=snd0 \
		-device intel-hda \
		-device hda-duplex,audiodev=snd0 \
		-serial stdio

