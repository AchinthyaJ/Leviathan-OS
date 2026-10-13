# Agent Guidelines for Leviathan OS

This document provides explicit instructions for AI coding agents (Claude, Antigravity, Cursor, Codex, ChatGPT, Copilot, etc.) operating on this repository.

Read and follow these constraints strictly. Failure to follow these rules breaks system boot and userspace components.

---

## 1. Repository Architecture Overview

- `rootfs/`: The root filesystem tree packed into the initramfs. It contains a complete Ubuntu/Debian-derived userspace with custom Hyprland compositor binaries, Noctalia desktop shell, and system drivers.
- `rootfs/init`: The PID 1 startup script executed by the kernel. It mounts devtmpfs/proc/sysfs, discovers the data disk, configures networking, launches system daemons (udev, dbus, polkit, pipewire), and starts Hyprland.
- `rootfs/etc/hypr/hyprland.conf`: Hyprland compositor configuration tailored specifically for virtio-gpu DRM rendering.
- `image/boot/grub/grub.cfg`: GRUB bootloader configuration for UEFI ISO boot.
- `image/boot/vmlinuz`: Linux kernel image.
- `Makefile`: Build scripts for `bins`, `initramfs`, and `iso`.
- `run.sh`: QEMU test launcher using OVMF UEFI, KVM, VirtIO-GPU with OpenGL acceleration, and VirtIO tablet input.

---

## 2. Hard Constraints (What NOT to Do)

### Never Break Pseudo-Filesystems and Device Nodes
- Do not create static files in `rootfs/dev`, `rootfs/proc`, or `rootfs/sys`.
- Essential static devices (`/dev/null`, `/dev/zero`, `/dev/urandom`, `/dev/random`) must be proper character device nodes (`mknod -m 666 ... c 1 3`, etc.), NEVER plain text or binary files. Plain files under `/dev` will crash tools like `aspell` and `systemd-udevd`.

### Never Overwrite Standard Architecture Symlinks
- The root filesystem follows the modern UsrMerge layout:
  - `/bin -> usr/bin`
  - `/sbin -> usr/sbin`
  - `/lib -> usr/lib`
  - `/lib64 -> usr/lib64`
- Never delete or convert these directory symlinks to static directories, as it breaks glibc dynamic loader lookup (`ld-linux-x86-64.so.2`) and causes `dpkg-divert` fatal errors.

### Never Change User Privilege Model
- The primary user is `leviathan`, configured as root (UID 0 / GID 0).
- Home directory `/root` is canonical, with `/home/leviathan` symlinked to `/root`.
- Do not revert `leviathan` to UID 1000 or revoke passwordless sudo in `/etc/sudoers.d/99-leviathan`.

### Never Introduce Blocking Delays in `rootfs/init`
- The boot process must complete in under 5 seconds.
- Peripheral drivers, Wi-Fi hardware scanning, and network lease requests (`udhcpc`) must run in the background.
- Do not add unconditional `sleep` statements in the primary boot path.

### Never Re-enable virtio-gpu Incompatible Hyprland Features
In `rootfs/etc/hypr/hyprland.conf`:
- Keep `render:use_fp16 = 0` (integer 0, not false). virgl does not support FP16 textures and throws `TRANSFER3D: 22` / `GL_INVALID_FRAMEBUFFER_OPERATION`.
- Keep `debug:damage_tracking = 0`. Partial damage tracking in virtio-gpu leaves white bars and unpainted scanlines.
- Keep `AQ_FORCE_LINEAR_BLIT=0` and `AQ_NO_ATOMIC=1` in environment settings to prevent DRM atomic flip glitches.
- Do not set `cursor_trail = 1` in Kitty configurations (`/etc/xdg/kitty/kitty.conf`), as trailing shaders render as white flicker bands under virtualized OpenGL.

### Never Use Emojis in Documentation
- Do not output emojis in any Markdown (`.md`) file, shell comment, or commit message.

---

## 3. Prescribed Workflow for Operating on `rootfs`

When installing packages, modifying dpkg states, or running scripts inside `rootfs`:

1. **Use Docker or Bubblewrap with Proper Mounts**:
   Do not run chroot directly on the host without proper root mappings and dev mounts:
   ```bash
   docker run --rm -v "$PWD/rootfs":/chroot -w / ubuntu:24.04 bash -c '
     export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
     chroot /chroot <command>
   '
   ```
2. **Preserve PATH in All Shell Environments**:
   `dpkg` and `apt` strictly require `/usr/sbin` and `/sbin` in `PATH`. Always export:
   `PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"`
3. **Verify Packaging Status**:
   Always run `dpkg --audit` and `apt-get check` inside the chroot before finalizing changes to ensure no broken or unconfigured packages remain.
4. **Clean up Scratch and Temporary Artifacts**:
   Do not leave temporary files, intermediate build artifacts, or personal log files in the repository.

---

## 4. Verification Checklist Before Reporting Task Complete

Before declaring any change ready:
1. Run `make iso` and verify it produces `build/leviathan.iso` without errors.
2. Verify that `rootfs/init` passes shell syntax check: `sh -n rootfs/init`.
3. Check `rootfs/etc/environment` and `rootfs/etc/profile` to ensure consistent paths.
4. Provide comprehensive written documentation in your response describing what was changed and why.
