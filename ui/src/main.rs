mod color;
mod font;
mod graphics;
mod scene;

use graphics::Graphics;
use scene::Scene;

use std::env;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

struct RawTerminal {
    orig: libc::termios,
    active: bool,
}

impl RawTerminal {
    fn enter() -> Self {
        unsafe {
            let mut orig: libc::termios = std::mem::zeroed();
            if libc::isatty(libc::STDIN_FILENO) == 1 && libc::tcgetattr(libc::STDIN_FILENO, &mut orig) == 0 {
                let mut raw = orig;
                raw.c_lflag &= !(libc::ICANON | libc::ECHO);
                raw.c_cc[libc::VMIN] = 0;
                raw.c_cc[libc::VTIME] = 0;
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
                Self { orig, active: true }
            } else {
                Self { orig, active: false }
            }
        }
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.orig);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut max_frames: Option<u64> = None;

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
        }
        i += 1;
    }

    println!("leviathan-ui: Initializing Rust graphics subsystem on /dev/fb0...");
    let mut gfx = match Graphics::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("leviathan-ui: Graphics initialization failed: {}", e);
            std::process::exit(1);
        }
    };

    let width = gfx.width();
    let height = gfx.height();
    println!("leviathan-ui: Framebuffer initialized ({}x{} @ {} bpp)", width, height, gfx.bpp());

    // Enter raw terminal mode
    let _raw_term = RawTerminal::enter();

    // Create scene and pre-render background cache
    let mut scene = Scene::new(&mut gfx);

    println!("leviathan-ui: Starting 60 FPS swimming scenery on {}x{}...", width, height);

    let start_time = Instant::now();
    let target_frame_duration = Duration::from_nanos(16_666_666); // 60 FPS

    let mut frame_count: u64 = 0;
    let mut current_fps: f32 = 60.0;
    let mut last_fps_time = Instant::now();
    let mut last_fps_frame: u64 = 0;

    let mut stdin = std::io::stdin();
    let mut key_buf = [0u8; 1];

    loop {
        let frame_start = Instant::now();
        let time_sec = start_time.elapsed().as_secs_f32();

        // 1. Update simulation
        scene.update(width, height, time_sec);

        // 2. Render frame to backbuffer
        scene.render(&mut gfx, time_sec, current_fps, frame_count);

        // 3. Push complete frame to screen via double buffering
        gfx.swap_buffers();

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
                break;
            }
        }

        // Non-blocking keyboard check
        if let Ok(n) = stdin.read(&mut key_buf) {
            if n > 0 {
                let key = key_buf[0];
                if key == b'q' || key == b'Q' || key == 27 || key == b'\n' || key == 3 {
                    break;
                }
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
        "leviathan-ui: Exiting graphics scenery ({} frames rendered at avg {:.1} FPS)",
        frame_count, current_fps
    );
    // Dropping gfx automatically restores KD_TEXT and unmaps framebuffer
}
