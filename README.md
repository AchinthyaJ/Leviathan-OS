# Leviathan OS — Pre-Release 1 (Alpha)

**Codename**: Resilient Dragon  
**Release**: Alpha Pre-Release 1 (`PR-1`)  
**Base Architecture**: Modern UsrMerge Linux userspace, custom static initramfs, Hyprland Wayland compositor, and Noctalia desktop shell.

Leviathan OS is an experimental, modern Linux distribution engineered from a lightweight live initramfs architecture. It delivers a fast, GPU-accelerated Wayland desktop session with an immersive abyssal visual aesthetic, native application packaging, and persistent storage overlays.

---

## Architecture and Key Components

- **Kernel and Boot Architecture**:
  - Direct UEFI boot via GRUB with high-resolution graphics support (`1920x1080@60`).
  - High-compression zstd initramfs architecture for near-instant boot times.
  - Multi-stage cinematic boot sequence with procedural abyssal visual rendering.
- **Display Server & Compositor**:
  - Hyprland Wayland compositor optimized specifically for VirtIO-GPU DRM hardware acceleration.
  - Custom display profile avoiding FP16/atomic-flip virtualization incompatibilities.
- **Desktop Environment**:
  - Custom Noctalia desktop shell featuring status bar indicators, workspace management, and launcher controls.
  - Obsidian glass login screen with secure SHA-512 authentication and power management controls.
- **Storage and Persistence**:
  - Automatic detection and mounting of ext4 data disks labeled `LEVIATHAN_DATA`.
  - Non-destructive persistent overlayfs layers across `/etc`, `/root`, `/home`, `/var`, and `/opt`.
- **Packaging and Userspace**:
  - Full native `apt` and `dpkg` support with Ubuntu 24.04 (Noble) package base.
  - UsrMerge architecture (`/bin -> usr/bin`, `/sbin -> usr/sbin`, `/lib -> usr/lib`, `/lib64 -> usr/lib64`).

---

## Default Credentials

Leviathan OS boots into a secure graphical authentication screen:

- **Username**: `leviathan`
- **Default Password**: `leviathan`
- **Privilege Model**: Configured as root administrator (`UID 0` / `GID 0`) with full passwordless `sudo` execution.

---

## Built-in Applications

- **Web Browser**: Firefox with native Wayland backend.
- **File Manager**: Dolphin (KDE Frameworks 5) configured for Wayland sessions.
- **Terminal Emulator**: Kitty GPU-accelerated terminal with custom Noctalia theme.
- **Core CLI Utilities**: `nano`, `htop`, `curl`, `wget`, `vim-tiny`, `iproute2`, `net-tools`, `less`, `udhcpc`.
- **Multimedia Stack**: PipeWire, WirePlumber, and PulseAudio emulation layer.

---

## Default Desktop Keybindings

| Keybinding | Action |
| :--- | :--- |
| `Super + Return` | Open Kitty Terminal |
| `Super + B` | Launch Firefox Web Browser |
| `Super + E` | Open Dolphin File Manager |
| `Super + Space` / `Super + D` | Toggle Application Launcher |
| `Super + S` | Open System Settings |
| `Super + L` | Lock Screen / Authentication Overlay |
| `Super + Q` | Close Active Window |
| `Super + F` | Toggle Fullscreen |
| `Super + Shift + Space` | Toggle Floating Window Mode |
| `Super + 1..5` | Switch Workspace 1 through 5 |
| `Super + Shift + 1..5` | Move Active Window to Workspace 1 through 5 |
| `Super + Arrow Keys` | Focus Directional Window |

---

## Quick Start

### 1. Prerequisites

Install the required virtualization, compilation, and ISO creation tools on a Debian/Ubuntu host:

```bash
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf gcc make cpio zstd grub-pc-bin grub-efi-amd64-bin xorriso mtools
```

### 2. Building the ISO

To compile init utilities, package the root filesystem into an initramfs, and create the bootable UEFI ISO:

```bash
make iso
```

The resulting ISO image will be generated at `build/leviathan.iso`.

#### Populating Rootfs from Release Asset

If working from a clean clone without a local `rootfs/usr` tree, download the official rootfs release tarball:

```bash
make fetch-rootfs
```

To package a clean userspace snapshot for distribution:

```bash
make pack-rootfs
```

### 3. Running in QEMU

Launch Leviathan OS using the dedicated launcher script with hardware virtualization and VirtIO-GPU OpenGL acceleration:

```bash
./run.sh
```

Or via Make:

```bash
make run
```

For serial console debugging:

```bash
make run-serial
```

---

## Project Structure

```
Leviathan/
├── assets/                  # Wallpapers, brand graphics, and typography
├── build/                   # Output directory for ISO, initramfs, and data disks
├── docs/                    # Technical documentation and prototype captures
├── image/                   # Bootable ISO directory tree (GRUB, EFI, kernel)
├── init/                    # C sources for static init binaries (poweroff, getty, bootsplash)
├── rootfs/                  # Root filesystem userspace and system configurations
├── tools/                   # Bootstrap scripts, automation, and test utilities
├── ui/                      # Rust sources for preview, login, and UI components
├── Makefile                 # Build orchestration for bins, initramfs, and ISO
└── run.sh                   # QEMU launcher with KVM, OVMF, and VirtIO-GPU
```

---

## Contributing and Operational Guidelines

We welcome community contributions. Please review the following guidelines before submitting pull requests:

- Consult [CONTRIBUTING.md](CONTRIBUTING.md) for contribution requirements and pull request standards.
- For AI agent contributors, strictly adhere to [Agents.md](Agents.md) to preserve system boot stability, user privileges, and UsrMerge symlink integrity.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
