# Contributing to Leviathan OS

Thank you for your interest in contributing to Leviathan OS. This guide outlines everything you need to know about setting up your environment, making modifications, submitting pull requests, and verifying system behavior.

---

## 1. Prerequisites and Build Environment

To build and run Leviathan OS, ensure you have the following host tools installed:

- Linux host system (x86_64) with KVM enabled (`/dev/kvm` accessible)
- QEMU (`qemu-system-x86_64`)
- OVMF UEFI firmware packages (`ovmf`, `edk2-ovmf`)
- GCC and Make (`gcc`, `make`)
- CPIO and Zstandard (`cpio`, `zstd`)
- GRUB rescue tools and xorriso (`grub-pc-bin`, `grub-efi-amd64-bin`, `xorriso`, `mtools`)
- Docker or Bubblewrap (recommended for running maintenance commands inside `rootfs` chroot)

---

### Fetching the Rootfs Base

If cloning the repository fresh, download the official rootfs release asset to populate `rootfs/`:

```bash
make fetch-rootfs
```

### Building the OS

Build the bootable ISO using:

```bash
make iso
```

This will:
1. Compile custom init binaries in `init/`.
2. Pack the `rootfs/` directory into a fast `zstd`-compressed initramfs at `build/initramfs.img` and copy it to `image/boot/initramfs.img`.
3. Invoke `grub-mkrescue` to create `build/leviathan.iso`.

### Packaging the Rootfs

To package an updated `rootfs/` for GitHub Releases (automatically excluding temporary caches, logs, and docs):

```bash
make pack-rootfs
```

### Running in QEMU

Run the built image using the launcher script:

```bash
./run.sh
```

Or run via Make:

```bash
make run
```

### Fast Prototyping

If you only modify files inside `rootfs/`, you do not need to recompile binaries:
- Run `make initramfs` to refresh the initramfs.
- Run `make iso` to update the ISO.

---

## 3. Contribution Requirements

When submitting pull requests or proposing changes:

### Detailed Documentation of Changes
Every contribution must include detailed documentation explaining:
- The exact rationale for each modification.
- Files added, changed, or removed.
- Side effects on userspace, device drivers, or startup services.
- If dependencies were added, list their package source, version, and licensing.

### Visual Proof with Screenshots
For any changes that affect:
- Noctalia desktop shell or Wayland UI
- Hyprland compositor rendering or animations
- Application launches (Firefox, Dolphin, Kitty, etc.)
- Graphical boot or text console output

You **must** include screenshots or screen recordings showing:
1. Clean boot into the desktop environment without glitching.
2. The modified components operating normally under standard desktop resolution (1920x1080).
3. The terminal with relevant status or verification commands.

Pull requests affecting the graphical environment without visual proof will not be accepted.

---

## 4. Special Guidelines for AI Agents and Automated Tools

If you are using AI coding assistants (such as Claude, Antigravity, Cursor, ChatGPT, Codex, etc.) to generate or review code:

> **CRITICAL WARNING REGARDING AI ASSISTANTS ON OPERATING SYSTEM REPOSITORIES:**
> 
> Operating system trees and root filesystems are fragile. AI agents frequently make subtle, critical mistakes on low-level OS infrastructure:
> - Clobbering Linux pseudo-filesystems (`/dev`, `/proc`, `/sys`) with static files or improper mount points.
> - Breaking critical system symlinks (`/bin -> usr/bin`, `/lib -> usr/lib`, `/lib64 -> usr/lib64`).
> - Introducing glibc mismatch issues or breaking `ld.so` loader paths.
> - Corrupting packaging state databases in `/var/lib/dpkg` or `/var/lib/apt`.
> - Misunderstanding Wayland, DRM/KMS, and virtio-gpu driver constraints (such as FP16 framebuffer format support or atomic page-flip capabilities).
> 
> **You are personally responsible for everything your AI agent produces.**
> 
> Before submitting any AI-assisted pull request:
> 1. Thoroughly review every single file diff line-by-line.
> 2. Ensure the agent did not add extraneous scratch files, backup files, or unnecessary modifications.
> 3. Verify that all system services, permissions, and setuid binaries remain intact.
> 4. Boot the resulting ISO in QEMU and test interactive behavior directly.
> 5. Clearly disclose in your PR description that AI tools were used, specify the model, and explain how you verified the changes.

---

## 5. Coding and Architecture Rules

1. **Keep Init Fast and Deterministic**:
   Do not introduce blocking sleeps or slow network loops into `rootfs/init`. Peripheral drivers and non-critical daemons must be run asynchronously.
2. **Preserve User Privileges**:
   The primary desktop account is `leviathan` configured as root (UID 0 / GID 0) with a home directory at `/root` (and `/home/leviathan` linked to `/root`). Do not lock the root user out or alter essential sudoers rules in `/etc/sudoers.d/`.
3. **Wayland and GPU Compatibility**:
   Hyprland runs on VirtIO-GPU inside QEMU. Keep `render:use_fp16 = 0` and `debug:damage_tracking = 0` in `/etc/hypr/hyprland.conf` to prevent virtual GPU scanout tearing and transfer crashes.
4. **No Emojis in Documentation**:
   To preserve clarity, accessibility, and terminal readability, do not include emojis anywhere in documentation or commit messages.

---

## 6. Submitting Your Contribution

1. Fork the repository and create a feature branch (`git checkout -b feature/my-feature`).
2. Implement your changes following all guidelines above.
3. Test locally using `./run.sh` and capture visual proof (screenshots).
4. Commit your changes with clear, descriptive commit messages.
5. Push to your branch and open a Pull Request.
6. Fill out the PR template completely with your documentation, verification logs, and screenshots.
