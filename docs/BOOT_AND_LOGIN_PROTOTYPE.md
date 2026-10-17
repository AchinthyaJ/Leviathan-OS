# Leviathan OS — Cinematic Boot and Login Prototype (Phase 1)

This document describes the Phase 1 standalone prototype for the Leviathan OS cinematic boot and login experience.

## Mandatory Safety and Isolation Statement

This prototype is completely isolated from the host operating system and Leviathan's production boot sequence:
- Real authentication, PAM, `/etc/shadow`, and credentials are never invoked or stored.
- Host startup configurations, boot scripts, `/init`, GRUB, and systemd units remain completely untouched.
- Hyprland, Noctalia, Kitty, network configurations, and active user sessions are never modified or destabilized.
- The prototype is delivered as a standalone preview binary (`leviathan-preview`) that can be executed and closed at will.

---

## 1. Creative Direction: Entering the Abyss

The visual theme embodies Leviathan's "Resilient Dragon" identity:
- **Atmosphere**: Near-black midnight navy, abyssal violet, subtle bioluminescence, and volumetric ocean rays.
- **Colossal Leviathan**: A procedural kinematic creature silhouette undulating through the deep ocean with lateral photophore light nodes and an ethereal eye.
- **Atmospheric Depth**: Multi-layer marine snow and rising bioluminescent motes.
- **Typography & Brand**: Restrained letter-spaced typography, subtle drop shadows, official logo integration (`assets/logo.png`), and the tagline:
  *"The ocean is vast. Cross it with Leviathan."*

---

## 2. Architecture and Implementation

The implementation reuses Leviathan's existing Rust rendering stack in `ui/` without introducing external UI frameworks or heavyweight 3D dependencies.

### Directory Structure
```
ui/
├── Cargo.toml
└── src/
    ├── bin/
    │   └── leviathan_preview.rs      # Standalone preview executable and test harness
    └── prototype/
        ├── mod.rs                    # State machine and transition orchestrator
        ├── abyss_scene.rs            # Abyssal environment, caustics, motes, and Leviathan
        ├── boot_screen.rs            # Stages A (Awakening), B (Crossing), C (Transition)
        └── login_screen.rs           # Upward user selector, masked password, demo auth
```

### Core Components
1. **`AbyssScene` (`ui/src/prototype/abyss_scene.rs`)**:
   - Computes vertical abyssal gradient and moving volumetric light rays.
   - Updates and renders ambient marine snow and bioluminescent motes.
   - Generates an undulating multi-segment serpentine silhouette for the colossal Leviathan with dorsal crests and caudal tail flukes.
   - Renders the official Leviathan logo (`assets/logo.png`) with scaled alpha-blending.

2. **`BootScreen` (`ui/src/prototype/boot_screen.rs`)**:
   - **Stage A — Awakening (0.0s - 3.5s)**: Begins in pitch black, gradually reveals abyssal depth, particles, and distant Leviathan silhouette.
   - **Stage B — The Crossing (3.5s - 9.5s)**: Creature swims smoothly across the screen; displays official logo, system wordmark, tagline, and minimalist progress bar.
   - **Stage C — Transition (9.5s - 12.0s)**: Creature disappears into deep mist; smoothly crossfades into the login interface.

3. **`LoginScreen` (`ui/src/prototype/login_screen.rs`)**:
   - Zero unnecessary text or debug clutter: pure, minimal, modern aesthetic.
   - **Leviathan Brand Logo**: Positioned at top-left above the account switcher (`logo_x`, `logo_y`).
   - **Account Switcher**: Positioned on the left column directly above the password box (`user_x`, `user_y`), displaying the user badge, display name, handle, and sleek dropdown overlay.
   - **Full-Left Password Input Box**: Sleek obsidian glass box (`password_x`, `password_y`) with procedural lock icon, masked dots, and submit arrow.
   - **Desktop Environment Selector**: Positioned on the left column directly below the password box (`desktop_env_x`, `desktop_env_y`), allowing switching between environments (`Hyprland`, `Noctalia`, `Sway`, `Leviathan UI`).
   - **Middle Clock & Date**: Large digital clock centered in the middle of the screen (`time_x`, `time_y`).
   - **Geometric Power Controls**: Sleek circular icon buttons (Sleep, Restart, Shut Down) with zero text clutter.
   - **Full .conf Configuration (`login.conf`)**: Every component's position is fully customizable along both X and Y pixel axes via `login.conf`. Supports `-1` for automatic centering and corner anchoring.

4. **`LeviathanExperience` (`ui/src/prototype/mod.rs`)**:
   - Coordinates state progression: `BootSequence` -> `Transitioning` -> `LoginSession`.
   - Supports instantaneous replay (`R` key), skip (`Space` / `Enter`), and reduced-motion toggle (`M` key).
   - Provides `save_graphics_to_png` for generating automated high-resolution screenshots.

---

## 3. How to Launch and Test

### Automated Verification Suite
Run the automated test suite to verify all visual stages, upward menu behavior, user switching, masked input, and resolution scaling:
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview -- --verify
```
All verification screenshots are saved to `target/prototype_verification/` and copied to `docs/prototype_screenshots/`.

### Interactive Preview Modes
Launch full flow (boot sequence followed by smooth transition to login screen):
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview
```

Launch standalone login screen directly:
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview -- --login-only
```

Launch standalone boot animation only:
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview -- --boot-only
```

Run at specific viewport resolution (e.g., 1280x720):
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview -- --resolution 1280x720
```

Enable reduced-motion mode:
```bash
cargo run --manifest-path ui/Cargo.toml --bin leviathan-preview -- --reduced-motion
```

---

## 4. Interactive Keyboard Controls

| Key | Action |
| :--- | :--- |
| **R** | Replay boot cinematic sequence from Stage A (Awakening) |
| **Space** | Skip boot sequence directly into Login Screen |
| **Up / Down** | Open and navigate upward user selection menu |
| **Tab** | Cycle focus between password input, user dropdown, and power menu |
| **Enter** | Submit demonstration password / activate selection |
| **Backspace** | Delete password character |
| **M** | Toggle reduced-motion mode |
| **Esc / Q** | Cleanly exit prototype (restores terminal state, leaves host intact) |

---

## 5. Verification Results Summary

The verification suite tested all requirements across 1920x1080 and 1280x720 viewports:
1. **Boot Lifecycle**: Stage A (Awakening), Stage B (The Crossing), and Stage C (Transition) render with appropriate timings and alpha transitions.
2. **Replay Support**: Triggering replay (`R`) resets time and returns to Stage A without restarting the process.
3. **Upward-Opening Dropdown**: Verified that opening the dropdown expands upward above the username card.
4. **Layout Stability**: Switching users between `leviathan`, `operator`, and `explorer` leaves the password field at the exact same vertical coordinate.
5. **Demonstrational Password**: Typing `levi-123` correctly masks input; pressing Enter produces a simulated demo session notification and never calls PAM.
6. **Graceful Downscaling**: Renders cleanly at 1280x720 without text truncation or clipping.
7. **Clean Exit**: Pressing Escape or Q exits immediately without affecting host desktop or services.

All visual verification screenshots can be found in `docs/prototype_screenshots/`.
