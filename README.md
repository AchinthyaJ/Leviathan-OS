# Leviathan OS

Leviathan OS is an experimental, modern Linux distribution built on a lightweight live initramfs architecture featuring Hyprland (Wayland compositor) and the custom Noctalia desktop shell.

---

## Architecture and Key Components

- **Kernel and Boot**: Linux kernel booted via UEFI GRUB with zstd-compressed initramfs for instant startup.
- **Compositor**: Hyprland built and optimized for virtio-gpu DRM rendering with smooth Wayland client support.
- **Desktop Shell**: Noctalia desktop environment providing status bar, workspace indicators, and system controls.
- **Default User**: `leviathan` configured as primary root administrator (UID 0 / GID 0) with passwordless sudo.
- **Built-in Applications**:
  - Web Browser: Firefox with native Wayland backend.
  - File Manager: Dolphin (KDE Frameworks 5) configured for root Wayland session.
  - Terminal: Kitty with GPU acceleration.
  - Core Utilities: `nano`, `htop`, `curl`, `wget`, `vim-tiny`, `iproute2`, `net-tools`, `less`.
  - Package Management: Full native `apt` and `dpkg` support.

---

## Quick Start

### Prerequisites

Ensure you have QEMU, KVM, and OVMF UEFI installed on your Linux host:

```bash
# Debian / Ubuntu host:
sudo apt-get install qemu-system-x86 ovmf gcc make cpio zstd grub-pc-bin grub-efi-amd64-bin xorriso mtools
```

### Building the ISO

To compile init binaries, assemble the initramfs, and create the bootable ISO:

```bash
make iso
```

The resulting ISO will be generated at `build/leviathan.iso`.

### Running in QEMU

Launch Leviathan OS inside QEMU with hardware acceleration and VirtIO GPU support:

```bash
./run.sh
```

Or via Make:

```bash
make run
```

---

## Contributing

We welcome community contributions! Please read our [Contribution Guidelines](contribution.md) before submitting pull requests.

Important contribution requirements:
- Include visual proof (screenshots or recordings) for any UI, compositor, or application changes.
- Provide thorough documentation of your changes and why they were made.
- Review the special guidelines for AI assistants if using LLM/agent tooling to prevent low-level OS regressions.
- For AI agent contributors, refer to [Agents.md](Agents.md) for concrete operational rules and safety constraints.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.