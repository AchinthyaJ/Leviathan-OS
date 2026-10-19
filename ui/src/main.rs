mod animation3d;
mod app_files;
mod app_packages;
mod app_settings;
mod app_terminal;
mod camera;
mod clipboard;
mod color;
mod config;
mod desktop;
mod desktop_entry;
mod dock;
mod font;
mod graphics;
mod icons;
mod image;
mod image_loader;
mod input;
mod layer;
mod layer_editor;
mod layer_manager;
mod math3d;
mod mesh;
mod mouse;
mod package;
mod render3d;
mod scene;
mod squircle;
mod wayland_backend;
mod widget;
mod window_manager;

use color::Color;
use desktop::Desktop;
use graphics::Graphics;
use input::{Action, InputManager};
use scene::Scene;
use wayland_backend::WaylandBackend;

use std::env;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut max_frames: Option<u64> = None;
    let mut start_in_3d = false;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--frames" && i + 1 < args.len() {
            max_frames = args[i + 1].parse().ok();
            i += 1;
        } else if args[i] == "--timeout" && i + 1 < args.len() {
            if let Ok(sec) = args[i + 1].parse::<u64>() {
                max_frames = Some(sec * 60);
            }
            i += 1;
        } else if args[i] == "--3d" {
            start_in_3d = true;
        }
        i += 1;
    }

    // ─── Backend selection ────────────────────────────────────────────────────
    //
    // When WAYLAND_DISPLAY is set we are running under a Wayland compositor
    // (Hyprland in production).  We create a full-screen wlr-layer-shell
    // OVERLAY surface and use the same CPU backbuffer for rendering — only
    // the presentation path changes (wl_shm + surface.commit instead of fb0
    // mmap copy).
    //
    // If WAYLAND_DISPLAY is absent, we fall back to the original /dev/fb0
    // path so the binary continues to work stand-alone.

    let wayland_display = env::var("WAYLAND_DISPLAY").ok();
    let using_wayland = wayland_display.is_some();

    let mut wayland: Option<WaylandBackend> = None;
    let mut gfx: Graphics;

    if using_wayland {
        println!("leviathan-ui: WAYLAND_DISPLAY is set — connecting to Hyprland via wlr-layer-shell...");
        match WaylandBackend::new(1920, 1080) {
            Ok(wb) => {
                let w = wb.width();
                let h = wb.height();
                println!("leviathan-ui: Wayland layer surface configured ({}x{})", w, h);
                gfx = Graphics::new_for_wayland(w, h);
                wayland = Some(wb);
            }
            Err(e) => {
                eprintln!("leviathan-ui: Wayland init failed ({}), falling back to /dev/fb0", e);
                gfx = Graphics::new().unwrap_or_else(|err| {
                    eprintln!("leviathan-ui: fb0 init also failed: {}", err);
                    std::process::exit(1);
                });
            }
        }
    } else {
        println!("leviathan-ui: Initializing Leviathan Desktop GUI on /dev/fb0...");
        gfx = match Graphics::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("leviathan-ui: Graphics initialization failed: {}", e);
                std::process::exit(1);
            }
        };
        println!(
            "leviathan-ui: Framebuffer initialized ({}x{} @ {} bpp)",
            gfx.width(), gfx.height(), gfx.bpp()
        );
    }

    let width = gfx.width();
    let height = gfx.height();

    // Suppress the kernel VT text cursor only in framebuffer mode.
    // In Wayland mode the compositor owns the VT and we must not touch it.
    if !using_wayland {
        for path in &["/dev/tty1", "/dev/tty0", "/dev/console"] {
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                unsafe {
                    libc::ioctl(fd, 0x4B3A /* KDSETMODE */, 1 /* KD_GRAPHICS */);
                    let hide_cursor = b"\x1b[?25l";
                    libc::write(fd, hide_cursor.as_ptr() as *const libc::c_void, hide_cursor.len());
                }
                break;
            }
        }
    }

    // Initialize system clipboard
    clipboard::Clipboard::init();

    // Initialize unified input subsystem (Linux evdev + virtual terminals + stdin).
    // In Wayland mode we still create the InputManager but it will find no
    // evdev devices (the compositor has grabbed them).  Keyboard/mouse events
    // then come exclusively from the Wayland backend.
    let input_manager = InputManager::new();

    // Initialize Desktop Environment (Dock, Windows, Terminal, Settings, Mouse)
    let mut desktop = Desktop::new(width, height);
    if start_in_3d {
        desktop.show_3d_viewport = true;
    }

    // Initialize 3D Engine for 3D Abyss app
    let mut scene = Scene::new(&mut gfx);

    println!("leviathan-ui: Starting Leviathan Desktop Environment on {}x{}...", width, height);

    let start_time = Instant::now();
    let target_frame_duration = Duration::from_nanos(16_666_666); // 60 FPS

    let mut frame_count: u64 = 0;
    let mut current_fps: f32 = 60.0;
    let mut last_fps_time = Instant::now();
    let mut last_fps_frame: u64 = 0;

    'main_loop: loop {
        let frame_start = Instant::now();
        let time_sec = start_time.elapsed().as_secs_f32();

        // ── 0. Wayland event dispatch (before any input processing) ───────────
        if let Some(ref mut wb) = wayland {
            if !wb.poll_events() {
                eprintln!("leviathan-ui: Wayland connection closed — exiting.");
                break 'main_loop;
            }

            // Feed Wayland pointer events into the desktop mouse state.
            {
                let p = &wb.state.pointer;
                desktop.mouse.inject_wayland_event(
                    p.x as i32,
                    p.y as i32,
                    p.left_down,
                    p.right_down,
                    p.left_clicked,
                    p.left_released,
                    p.right_clicked,
                    p.scroll_delta,
                );
            }

            // Feed Wayland keyboard events into the action queue.
            let wl_actions = drain_wayland_keys(wb);
            for action in wl_actions {
                dispatch_action(
                    action,
                    &mut desktop,
                    &mut scene,
                    width,
                    height,
                    &mut frame_count,
                    &mut wayland,
                    // We use a closure trick: break cannot cross function boundaries,
                    // so we signal exit via a flag instead.
                );
            }
        }

        // ── 1. Process Input Actions (evdev keyboard — used in fb0 mode) ──────
        let actions = input_manager.poll_actions();
        let mut should_exit = false;
        for action in actions {
            match action {
                Action::Exit | Action::Escape => {
                    if desktop.show_3d_viewport {
                        desktop.show_3d_viewport = false;
                    } else if desktop.dock.is_all_apps_open {
                        desktop.dock.is_all_apps_open = false;
                    } else if desktop.wm.has_any_focused()
                        || (desktop.terminal.window.is_open && desktop.terminal.window.is_focused)
                        || (desktop.files.window.is_open && desktop.files.window.is_focused)
                        || (desktop.settings.window.is_open && desktop.settings.window.is_focused)
                        || (desktop.packages.window.is_open && desktop.packages.window.is_focused)
                    {
                        desktop.close_focused_window();
                    } else if matches!(action, Action::Exit) {
                        should_exit = true;
                    } else {
                        desktop.handle_action(action, width as i32, height as i32);
                    }
                }
                Action::Char(b) if desktop.show_3d_viewport => {
                    match b {
                        b'c' | b'C' => scene.camera.cycle_mode(),
                        b'l' | b'L' => scene.renderer3d.cycle_shading_mode(),
                        b'1' => scene.animation3d.scene_mode = 1,
                        b'2' => scene.animation3d.scene_mode = 2,
                        b'3' => scene.animation3d.scene_mode = 3,
                        b' ' => scene.animation3d.is_paused = !scene.animation3d.is_paused,
                        27 | b'q' | b'Q' => desktop.show_3d_viewport = false,
                        _ => {}
                    }
                }
                Action::CycleCamera if desktop.show_3d_viewport => scene.camera.cycle_mode(),
                Action::CycleShading if desktop.show_3d_viewport => scene.renderer3d.cycle_shading_mode(),
                Action::Scene1 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 1,
                Action::Scene2 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 2,
                Action::Scene3 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 3,
                Action::TogglePause if desktop.show_3d_viewport => {
                    scene.animation3d.is_paused = !scene.animation3d.is_paused;
                }
                _ => {
                    desktop.handle_action(action, width as i32, height as i32);
                }
            }
        }
        if should_exit {
            break 'main_loop;
        }

        // ── 2. Update & Render Active Mode ────────────────────────────────────
        if desktop.show_3d_viewport {
            scene.update(width, height, time_sec);
            scene.render(&mut gfx, time_sec, current_fps, frame_count);

            gfx.fill_rect(0, 0, width as i32, 24, Color::new(38, 38, 40));
            gfx.draw_string(14, 5, "[✦ 3D ABYSS VIEWPORT] Press ESC to return to Desktop", Color::MACOS_TEXT);
            desktop.mouse.draw_cursor(&mut gfx);
        } else {
            desktop.update(width, height);
            desktop.draw(&mut gfx, current_fps);
        }

        // ── 3. Push frame to screen ───────────────────────────────────────────
        if let Some(ref mut wb) = wayland {
            // Wayland mode: copy backbuffer to wl_shm and commit the surface.
            if let Err(e) = wb.present(gfx.get_backbuffer()) {
                eprintln!("leviathan-ui: Wayland present failed: {} — exiting.", e);
                break 'main_loop;
            }
            // Mark gfx clean so the next frame starts fresh without the
            // damage-skip optimisation interfering with the shm copy.
            gfx.mark_all_dirty();
        } else {
            // Framebuffer mode: the existing double-buffer copy to fb0 mmap.
            gfx.swap_buffers();
        }

        frame_count += 1;

        // Compute measured FPS every 500 ms
        let fps_elapsed = last_fps_time.elapsed().as_secs_f32();
        if fps_elapsed >= 0.5 {
            current_fps = (frame_count - last_fps_frame) as f32 / fps_elapsed;
            last_fps_time = Instant::now();
            last_fps_frame = frame_count;
        }

        // Check frame limit
        if let Some(limit) = max_frames {
            if frame_count >= limit {
                break 'main_loop;
            }
        }

        // Pacing for 60 FPS
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_duration {
            let sleep_time = target_frame_duration - elapsed;
            if sleep_time > Duration::from_micros(300) {
                thread::sleep(sleep_time);
            }
        }
    }

    println!(
        "leviathan-ui: Exiting Desktop Environment ({} frames rendered at avg {:.1} FPS)...",
        frame_count, current_fps
    );

    // Restore VT text mode only when we owned the framebuffer.
    if !using_wayland {
        for path in &["/dev/tty1", "/dev/tty0", "/dev/console"] {
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                unsafe {
                    libc::ioctl(fd, 0x4B3A /* KDSETMODE */, 0 /* KD_TEXT */);
                    libc::ioctl(fd, 0x5606 /* VT_ACTIVATE */, 1);
                    let reset_seq = b"\x1b[?25h\x1b[2J\x1b[H";
                    libc::write(fd, reset_seq.as_ptr() as *const libc::c_void, reset_seq.len());
                }
                break;
            }
        }
    }
}

// ─── Wayland key → Action conversion ─────────────────────────────────────────

/// Drain the Wayland key queue and convert to `Action` values.
///
/// The `code` field in [`WaylandKeyEvent`] is the raw Linux evdev keycode,
/// identical to the constants in `input.rs`.  We replicate the same mapping
/// here so Wayland keyboard input is processed exactly like evdev input.
fn drain_wayland_keys(wb: &mut WaylandBackend) -> Vec<Action> {
    use input::{KEY_ESC, KEY_ENTER};
    // Additional key constants matching Linux input-event-codes
    const KEY_BACKSPACE: u16 = 14;
    const KEY_TAB: u16 = 15;
    const KEY_LEFTSHIFT: u16 = 42;
    const KEY_RIGHTSHIFT: u16 = 54;
    const KEY_LEFTCTRL: u16 = 29;
    const KEY_RIGHTCTRL: u16 = 97;
    const KEY_LEFTALT: u16 = 56;
    const KEY_RIGHTALT: u16 = 100;
    const KEY_LEFTMETA: u16 = 125;
    const KEY_RIGHTMETA: u16 = 126;
    const KEY_UP: u16 = 103;
    const KEY_DOWN: u16 = 108;
    const KEY_LEFT: u16 = 105;
    const KEY_RIGHT: u16 = 106;
    const KEY_PAGEUP: u16 = 104;
    const KEY_PAGEDOWN: u16 = 109;
    const KEY_HOME: u16 = 102;
    const KEY_END: u16 = 107;
    const KEY_DELETE: u16 = 111;
    const KEY_F4: u16 = 62;
    const KEY_T: u16 = 20;
    const KEY_E: u16 = 18;
    const KEY_M: u16 = 50;

    let mut shift = false;
    let mut ctrl = false;
    let mut alt = false;
    let mut meta = false;
    let mut actions = Vec::new();

    // Snapshot the queue
    let events: Vec<_> = wb.state.keys.drain(..).collect();

    // First pass: track modifier state
    for ev in &events {
        match ev.code as u16 {
            KEY_LEFTSHIFT | KEY_RIGHTSHIFT => shift = ev.pressed,
            KEY_LEFTCTRL  | KEY_RIGHTCTRL  => ctrl  = ev.pressed,
            KEY_LEFTALT   | KEY_RIGHTALT   => alt   = ev.pressed,
            KEY_LEFTMETA  | KEY_RIGHTMETA  => meta  = ev.pressed,
            _ => {}
        }
    }

    // Second pass: convert pressed keys to actions (matches input.rs logic)
    for ev in events {
        if !ev.pressed { continue; }
        let code = ev.code as u16;
        let action = match code {
            KEY_ESC => Some(Action::Escape),
            KEY_ENTER => Some(Action::Enter),
            KEY_BACKSPACE => Some(Action::Backspace),
            KEY_UP => Some(Action::ArrowUp),
            KEY_DOWN => Some(Action::ArrowDown),
            KEY_LEFT => Some(Action::ArrowLeft),
            KEY_RIGHT => Some(Action::ArrowRight),
            KEY_PAGEUP => Some(Action::PageUp),
            KEY_PAGEDOWN => Some(Action::PageDown),
            KEY_HOME => Some(Action::Home),
            KEY_END => Some(Action::End),
            KEY_DELETE => Some(Action::Delete),
            KEY_F4 if alt => Some(Action::AltF4),
            KEY_TAB if alt => Some(Action::AltTab),
            KEY_T if meta => Some(Action::SuperT),
            KEY_E if meta => Some(Action::SuperE),
            KEY_M if meta => Some(Action::SuperM),
            _ if meta => None, // other Super combos: ignored for now
            _ => {
                // Printable character: map evdev scancode to ASCII
                if let Some(ch) = evdev_to_char(ev.code, shift) {
                    Some(Action::Char(ch as u8))
                } else {
                    None
                }
            }
        };
        if let Some(a) = action {
            actions.push(a);
        }
    }
    actions
}

/// Dispatch a single `Action` in a way that mirrors the main loop logic
/// but returns a flag instead of breaking (since we cannot break across
/// function call boundaries in Rust without a closure trick).
fn dispatch_action(
    action: Action,
    desktop: &mut Desktop,
    scene: &mut Scene,
    width: u32,
    height: u32,
    _frame_count: &mut u64,
    _wayland: &mut Option<WaylandBackend>,
) {
    match action {
        Action::Exit | Action::Escape => {
            if desktop.show_3d_viewport {
                desktop.show_3d_viewport = false;
            } else if desktop.dock.is_all_apps_open {
                desktop.dock.is_all_apps_open = false;
            } else if desktop.wm.has_any_focused()
                || (desktop.terminal.window.is_open && desktop.terminal.window.is_focused)
                || (desktop.files.window.is_open && desktop.files.window.is_focused)
                || (desktop.settings.window.is_open && desktop.settings.window.is_focused)
                || (desktop.packages.window.is_open && desktop.packages.window.is_focused)
            {
                desktop.close_focused_window();
            } else {
                desktop.handle_action(action, width as i32, height as i32);
            }
        }
        Action::Char(b) if desktop.show_3d_viewport => match b {
            b'c' | b'C' => scene.camera.cycle_mode(),
            b'l' | b'L' => scene.renderer3d.cycle_shading_mode(),
            b'1' => scene.animation3d.scene_mode = 1,
            b'2' => scene.animation3d.scene_mode = 2,
            b'3' => scene.animation3d.scene_mode = 3,
            b' ' => scene.animation3d.is_paused = !scene.animation3d.is_paused,
            27 | b'q' | b'Q' => desktop.show_3d_viewport = false,
            _ => {}
        },
        Action::CycleCamera if desktop.show_3d_viewport => scene.camera.cycle_mode(),
        Action::CycleShading if desktop.show_3d_viewport => scene.renderer3d.cycle_shading_mode(),
        Action::Scene1 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 1,
        Action::Scene2 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 2,
        Action::Scene3 if desktop.show_3d_viewport => scene.animation3d.scene_mode = 3,
        Action::TogglePause if desktop.show_3d_viewport => {
            scene.animation3d.is_paused = !scene.animation3d.is_paused;
        }
        _ => {
            desktop.handle_action(action, width as i32, height as i32);
        }
    }
}

/// Convert a Linux evdev scancode to a printable ASCII character, respecting
/// the shift modifier.  Covers the basic US layout keys used in the desktop.
fn evdev_to_char(code: u32, shift: bool) -> Option<char> {
    // Map (evdev_code, shift) → char
    let normal = match code {
        2  => '1', 3  => '2', 4  => '3', 5  => '4', 6  => '5',
        7  => '6', 8  => '7', 9  => '8', 10 => '9', 11 => '0',
        12 => '-', 13 => '=', 16 => 'q', 17 => 'w', 18 => 'e',
        19 => 'r', 20 => 't', 21 => 'y', 22 => 'u', 23 => 'i',
        24 => 'o', 25 => 'p', 26 => '[', 27 => ']', 30 => 'a',
        31 => 's', 32 => 'd', 33 => 'f', 34 => 'g', 35 => 'h',
        36 => 'j', 37 => 'k', 38 => 'l', 39 => ';', 40 => '\'',
        41 => '`', 43 => '\\',44 => 'z', 45 => 'x', 46 => 'c',
        47 => 'v', 48 => 'b', 49 => 'n', 50 => 'm', 51 => ',',
        52 => '.', 53 => '/', 57 => ' ',
        _ => return None,
    };
    let shifted = match code {
        2  => '!', 3  => '@', 4  => '#', 5  => '$', 6  => '%',
        7  => '^', 8  => '&', 9  => '*', 10 => '(', 11 => ')',
        12 => '_', 13 => '+', 16 => 'Q', 17 => 'W', 18 => 'E',
        19 => 'R', 20 => 'T', 21 => 'Y', 22 => 'U', 23 => 'I',
        24 => 'O', 25 => 'P', 26 => '{', 27 => '}', 30 => 'A',
        31 => 'S', 32 => 'D', 33 => 'F', 34 => 'G', 35 => 'H',
        36 => 'J', 37 => 'K', 38 => 'L', 39 => ':', 40 => '"',
        41 => '~', 43 => '|', 44 => 'Z', 45 => 'X', 46 => 'C',
        47 => 'V', 48 => 'B', 49 => 'N', 50 => 'M', 51 => '<',
        52 => '>', 53 => '?', 57 => ' ',
        _ => normal,
    };
    Some(if shift { shifted } else { normal })
}
