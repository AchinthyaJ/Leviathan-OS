use crate::color::Color;
use crate::graphics::Graphics;
use std::f32::consts::PI;

pub const NUM_BODY_SEGMENTS: usize = 115;
pub const NUM_BUBBLES: usize = 45;
pub const NUM_MOTES: usize = 60;

#[derive(Clone, Copy)]
pub struct Bubble {
    pub x: f32,
    pub y: f32,
    pub speed_y: f32,
    pub wobble_speed: f32,
    pub radius: i32,
    pub seed: f32,
}

#[derive(Clone, Copy)]
pub struct Mote {
    pub x: f32,
    pub y: f32,
    pub speed_y: f32,
    pub speed_x: f32,
    pub brightness: u8,
}

#[derive(Clone, Copy)]
pub struct BodySegment {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub radius: f32,
}

pub struct Scene {
    pub bg_cache: Vec<u8>,
    pub bubbles: [Bubble; NUM_BUBBLES],
    pub motes: [Mote; NUM_MOTES],
}

impl Scene {
    pub fn new(gfx: &mut Graphics) -> Self {
        let width = gfx.width();
        let height = gfx.height();

        // 1. Pre-render deep ocean background into cache
        println!("leviathan-ui: Pre-rendering deep ocean background cache...");

        // Vertical depth gradient
        for y in 0..height {
            let ty = y as f32 / height as f32;
            let r = (10.0 * (1.0 - ty) * (1.0 - ty) + 4.0 * (1.0 - ty) * ty * 2.0 + 1.0 * ty) as u8;
            let g = (45.0 * (1.0 - ty) + 12.0 * ty) as u8;
            let b = (80.0 * (1.0 - ty) + 20.0 * ty) as u8;
            let row_col = Color::rgb(r, g, b);
            for x in 0..width {
                gfx.put_pixel(x as i32, y as i32, row_col);
            }
        }

        // Light shafts / sunbeams
        let rays = [
            (160.0f32, 65.0f32,  0.34f32, 0.12f32),
            (360.0f32, 90.0f32,  0.40f32, 0.18f32),
            (570.0f32, 115.0f32, 0.36f32, 0.15f32),
            (810.0f32, 80.0f32,  0.43f32, 0.12f32),
            (1050.0f32, 95.0f32, 0.38f32, 0.14f32),
        ];

        for &(start_x, w_top, slant, intensity) in &rays {
            let max_y = (height as f32 * 0.72) as i32;
            for y in 0..max_y {
                let y_ratio = y as f32 / max_y as f32;
                let curr_center_x = start_x + y as f32 * slant;
                let curr_width = w_top * (1.0 + y_ratio * 1.6);
                let beam_falloff = (1.0 - y_ratio) * intensity;

                let x_min = (curr_center_x - curr_width * 0.5) as i32;
                let x_max = (curr_center_x + curr_width * 0.5) as i32;

                for x in x_min..=x_max {
                    if x < 0 || (x as u32) >= width { continue; }
                    let dist = ((x as f32 - curr_center_x) / (curr_width * 0.5)).abs();
                    if dist > 1.0 { continue; }

                    let alpha = (1.0 - dist * dist) * beam_falloff;
                    let cur = gfx.get_pixel(x, y);
                    let ray_c = Color::rgb(80, 190, 230);
                    gfx.put_pixel(x, y, Color::blend(cur, ray_c, alpha));
                }
            }
        }

        // Seafloor ridges & volcanic basalt
        for x in 0..width as i32 {
            let fx = x as f32 * 0.008;
            let hill_y = (height as f32 - 110.0 + (fx * 3.1).sin() * 35.0 + (fx * 1.7).cos() * 25.0) as i32;
            for y in hill_y..height as i32 {
                let cur = gfx.get_pixel(x, y);
                let rock = Color::rgb(3, 14, 26);
                gfx.put_pixel(x, y, Color::blend(cur, rock, 0.85));
            }
        }

        for x in 0..width as i32 {
            let fx = x as f32 * 0.015;
            let rock_y = (height as f32 - 65.0 + (fx * 4.2).sin() * 22.0 + (fx * 7.5).cos() * 12.0) as i32;
            for y in rock_y..height as i32 {
                gfx.put_pixel(x, y, Color::rgb(2, 8, 16));
            }
        }

        let bg_cache = gfx.get_backbuffer().to_vec();

        // 2. Initialize Particles
        let mut bubbles = [Bubble {
            x: 0.0, y: 0.0, speed_y: 1.5, wobble_speed: 2.0, radius: 4, seed: 0.0,
        }; NUM_BUBBLES];

        for (i, b) in bubbles.iter_mut().enumerate() {
            let pseudo_rand = (i * 7919 + 1013) % 1000;
            b.x = ((pseudo_rand * 13) % width as usize) as f32;
            b.y = ((pseudo_rand * 37) % height as usize) as f32;
            b.speed_y = 1.2 + (pseudo_rand % 100) as f32 / 50.0;
            b.wobble_speed = 2.0 + (pseudo_rand % 100) as f32 / 33.0;
            b.radius = 2 + (pseudo_rand % 5) as i32;
            b.seed = (pseudo_rand % 100) as f32;
        }

        let mut motes = [Mote {
            x: 0.0, y: 0.0, speed_y: 0.4, speed_x: 0.1, brightness: 120,
        }; NUM_MOTES];

        for (i, m) in motes.iter_mut().enumerate() {
            let pr = (i * 6271 + 3307) % 1000;
            m.x = ((pr * 19) % width as usize) as f32;
            m.y = ((pr * 41) % height as usize) as f32;
            m.speed_y = 0.3 + (pr % 100) as f32 / 200.0;
            m.speed_x = -0.2 + (pr % 100) as f32 / 250.0;
            m.brightness = (70 + (pr % 140)) as u8;
        }

        Self { bg_cache, bubbles, motes }
    }

    pub fn update(&mut self, width: u32, height: u32, time_sec: f32) {
        for (i, b) in self.bubbles.iter_mut().enumerate() {
            b.y -= b.speed_y;
            b.x += (time_sec * b.wobble_speed + b.seed).sin() * 0.6;
            if b.y < 45.0 {
                b.y = (height - 40 - (i as u32 * 13 % 60)) as f32;
                b.x = (i as u32 * 79 % width) as f32;
            }
        }

        for (i, m) in self.motes.iter_mut().enumerate() {
            m.y += m.speed_y;
            m.x += m.speed_x + (time_sec * 0.5 + i as f32).sin() * 0.2;
            if m.y >= (height - 10) as f32 {
                m.y = 50.0;
                m.x = (i as u32 * 97 % width) as f32;
            }
            if m.x < 0.0 { m.x += width as f32; }
            if m.x >= width as f32 { m.x -= width as f32; }
        }
    }

    pub fn render(
        &self,
        gfx: &mut Graphics,
        time_sec: f32,
        fps: f32,
        frame_count: u64,
    ) {
        let _width = gfx.width();
        let height = gfx.height();

        // Fast-blit pre-rendered deep ocean background into backbuffer (~0.3 ms)
        gfx.copy_to_backbuffer(&self.bg_cache);

        // Hydrothermal vent pulses on seabed
        let vents = [220, 680, 1080];
        for (i, &vx) in vents.iter().enumerate() {
            let vy = height as i32 - 50;
            gfx.fill_rect(vx - 8, vy - 15, 16, 25, Color::rgb(12, 16, 24));
            let pulse = 0.7 + 0.3 * (time_sec * 3.0 + i as f32 * 2.0).sin();
            let glow_r = (12.0 * pulse) as i32;
            gfx.fill_circle(vx, vy - 15, glow_r, Color::rgb(0, (180.0 * pulse) as u8, (150.0 * pulse) as u8));
            gfx.fill_circle(vx, vy - 15, 5, Color::rgb(180, 255, 235));
        }

        // Render Kinematic Swimming Leviathan
        self.render_leviathan(gfx, time_sec);

        // Render Bubbles & Marine Snow
        for b in &self.bubbles {
            gfx.draw_circle(b.x as i32, b.y as i32, b.radius, Color::rgb(120, 220, 240));
            gfx.put_pixel((b.x as i32) - b.radius / 3, (b.y as i32) - b.radius / 3, Color::rgb(240, 255, 255));
        }

        for m in &self.motes {
            let c = Color::rgb(m.brightness / 2, m.brightness, m.brightness);
            gfx.put_pixel(m.x as i32, m.y as i32, c);
            if m.brightness > 180 {
                gfx.put_pixel(m.x as i32 + 1, m.y as i32, c);
            }
        }

        // Render HUD Overlay
        self.render_hud(gfx, fps, frame_count);
    }

    fn render_leviathan(&self, gfx: &mut Graphics, time_sec: f32) {
        let swim_speed = 0.55f32;
        let t_swim = time_sec * swim_speed;

        let head_x = 640.0 + 350.0 * (t_swim * 1.1).sin();
        let head_y = 380.0 + 120.0 * (t_swim * 1.7).sin();

        let vx = 350.0 * 1.1 * (t_swim * 1.1).cos();
        let vy = 120.0 * 1.7 * (t_swim * 1.7).cos();
        let head_angle = vy.atan2(vx);

        let mut segs = [BodySegment { x: 0.0, y: 0.0, angle: 0.0, radius: 10.0 }; NUM_BODY_SEGMENTS];
        segs[0] = BodySegment { x: head_x, y: head_y, angle: head_angle, radius: 28.0 };

        let segment_dist = 8.5f32;

        for i in 1..NUM_BODY_SEGMENTS {
            let progress = i as f32 / NUM_BODY_SEGMENTS as f32;
            let wave_amp = 0.035 + 0.38 * progress;
            let wave_offset = wave_amp * (i as f32 * 0.12 - time_sec * 4.8).sin();
            let cur_angle = segs[i - 1].angle + wave_offset;

            segs[i].x = segs[i - 1].x - cur_angle.cos() * segment_dist;
            segs[i].y = segs[i - 1].y - cur_angle.sin() * segment_dist;
            segs[i].angle = cur_angle;

            let body_envelope = (progress * PI).sin();
            if progress < 0.15 {
                segs[i].radius = 28.0 + (progress / 0.15) * 10.0;
            } else {
                segs[i].radius = 8.0 + 30.0 * body_envelope.powf(0.7);
            }
        }

        // A. Dorsal Spines & Webbed Fins
        for i in (12..NUM_BODY_SEGMENTS - 15).step_by(3) {
            let r = segs[i].radius;
            let a = segs[i].angle;
            let mut nx = -a.sin();
            let mut ny =  a.cos();
            if ny > 0.0 { nx = -nx; ny = -ny; }

            let spine_ripple = (i as f32 * 0.2 - time_sec * 5.0).sin();
            let spine_len = (r * 1.7 + 16.0 * spine_ripple).max(r + 4.0);

            let sx0 = (segs[i].x + nx * r) as i32;
            let sy0 = (segs[i].y + ny * r) as i32;
            let sx1 = (segs[i].x + nx * spine_len) as i32;
            let sy1 = (segs[i].y + ny * spine_len) as i32;

            if i > 15 {
                let prev_sx1 = (segs[i - 3].x + nx * (spine_len * 0.8)) as i32;
                let prev_sy1 = (segs[i - 3].y + ny * (spine_len * 0.8)) as i32;
                gfx.draw_line(sx1, sy1, prev_sx1, prev_sy1, Color::rgb(0, 180, 215));
            }

            gfx.draw_line(sx0, sy0, sx1, sy1, Color::rgb(10, 95, 125));
            gfx.fill_circle(sx1, sy1, 2, Color::rgb(90, 255, 240));
        }

        // B. Caudal Tail Fin
        let last = NUM_BODY_SEGMENTS - 1;
        let tx = segs[last].x;
        let ty = segs[last].y;
        let ta = segs[last].angle;
        let spread = 45.0 * PI / 180.0;

        for fan in -4..=4 {
            let fan_a = ta + (fan as f32 * spread / 4.0) + PI;
            let fin_len = 80.0 - (fan as f32).abs() * 8.0;
            let fx = (tx + fan_a.cos() * fin_len) as i32;
            let fy = (ty + fan_a.sin() * fin_len) as i32;
            gfx.draw_line(tx as i32, ty as i32, fx, fy, Color::rgb(0, 175, 220));
            gfx.fill_circle(fx, fy, 2, Color::rgb(130, 255, 245));
        }

        // C. Serpentine Armored Body Segments
        for seg in segs.iter().rev() {
            let cx = seg.x as i32;
            let cy = seg.y as i32;
            let cr = seg.radius as i32;

            gfx.fill_circle(cx, cy, cr, Color::rgb(6, 32, 44));
            if cr > 4 {
                gfx.fill_circle(cx, cy, cr - 4, Color::rgb(12, 68, 76));
            }
            if cr > 8 {
                let a = seg.angle;
                let under_x = cx + (a.sin() * (cr as f32 * 0.25)) as i32;
                let under_y = cy - (a.cos() * (cr as f32 * 0.25)) as i32;
                gfx.fill_circle(under_x, under_y, cr / 2, Color::rgb(24, 110, 105));
            }
        }

        // D. Bioluminescent Runes
        for i in (8..NUM_BODY_SEGMENTS - 8).step_by(3) {
            let bx = segs[i].x as i32;
            let by = segs[i].y as i32;

            let glow_phase = (time_sec * 3.0 + i as f32 * 0.12).sin();
            let glow_g = (210.0 + 45.0 * glow_phase) as u8;
            let glow_b = (190.0 + 65.0 * glow_phase) as u8;

            gfx.fill_circle(bx, by, 3, Color::rgb(0, glow_g, glow_b));
            gfx.fill_circle(bx, by, 1, Color::rgb(255, 255, 255));

            if i > 8 {
                let pbx = segs[i - 3].x as i32;
                let pby = segs[i - 3].y as i32;
                gfx.draw_line(pbx, pby, bx, by, Color::rgb(0, 190, 200));
            }
        }

        // E. Sea Dragon Head
        let hx = segs[0].x;
        let hy = segs[0].y;
        let ha = segs[0].angle;

        let forward_x = ha.cos();
        let forward_y = ha.sin();
        let right_x = -ha.sin();
        let right_y =  ha.cos();

        gfx.fill_circle(hx as i32, hy as i32, 30, Color::rgb(8, 38, 50));
        gfx.fill_circle(hx as i32, hy as i32, 25, Color::rgb(14, 75, 84));

        let snout_x = hx + forward_x * 42.0;
        let snout_y = hy + forward_y * 42.0;
        gfx.fill_circle((hx + forward_x * 20.0) as i32, (hy + forward_y * 20.0) as i32, 20, Color::rgb(10, 52, 65));
        gfx.fill_circle(snout_x as i32, snout_y as i32, 14, Color::rgb(12, 65, 78));

        let jaw_x = hx + forward_x * 32.0 + right_x * 12.0;
        let jaw_y = hy + forward_y * 32.0 + right_y * 12.0;
        gfx.fill_circle(jaw_x as i32, jaw_y as i32, 11, Color::rgb(8, 42, 54));

        for f in 0..3 {
            let fx0 = snout_x - forward_x * (8.0 + f as f32 * 10.0);
            let fy0 = snout_y - forward_y * (8.0 + f as f32 * 10.0);
            let fx1 = fx0 + right_x * 9.0;
            let fy1 = fy0 + right_y * 9.0;
            gfx.draw_line(fx0 as i32, fy0 as i32, fx1 as i32, fy1 as i32, Color::rgb(240, 255, 255));
        }

        // Blinking Golden Eye
        let eye_x = hx + forward_x * 12.0 - right_x * 12.0;
        let eye_y = hy + forward_y * 12.0 - right_y * 12.0;
        let blinking = (time_sec % 4.0) > 3.85;

        if blinking {
            gfx.draw_line((eye_x - forward_x * 6.0) as i32, (eye_y - forward_y * 6.0) as i32,
                          (eye_x + forward_x * 6.0) as i32, (eye_y + forward_y * 6.0) as i32,
                          Color::rgb(0, 180, 160));
        } else {
            gfx.fill_circle(eye_x as i32, eye_y as i32, 9, Color::rgb(0, 180, 160));
            gfx.fill_circle(eye_x as i32, eye_y as i32, 6, Color::rgb(255, 185, 0));
            gfx.fill_circle(eye_x as i32, eye_y as i32, 4, Color::rgb(255, 225, 60));
            gfx.draw_line((eye_x - right_x * 5.0) as i32, (eye_y - right_y * 5.0) as i32,
                          (eye_x + right_x * 5.0) as i32, (eye_y + right_y * 5.0) as i32,
                          Color::rgb(10, 10, 10));
        }

        // Swept Aquatic Horns
        for h in 0..2 {
            let horn_side = if h == 0 { -1.0f32 } else { 1.0f32 };
            let mut px = (hx - right_x * (horn_side * 10.0)) as i32;
            let mut py = (hy - right_y * (horn_side * 10.0)) as i32;

            for seg in 1..=12 {
                let st = seg as f32 / 12.0;
                let cur_hx = hx - forward_x * (st * 65.0) + right_x * (horn_side * (10.0 + st * 18.0)) - right_y * (st * st * 14.0);
                let cur_hy = hy - forward_y * (st * 65.0) + right_y * (horn_side * (10.0 + st * 18.0)) + right_x * (st * st * 14.0);
                gfx.draw_line(px, py, cur_hx as i32, cur_hy as i32, Color::rgb(18, 90 + seg * 9, 110 + seg * 10));
                if seg == 12 {
                    gfx.fill_circle(cur_hx as i32, cur_hy as i32, 3, Color::rgb(0, 255, 230));
                }
                px = cur_hx as i32;
                py = cur_hy as i32;
            }
        }

        // Bioluminescent Flowing Whiskers
        for w in 0..2 {
            let w_side = if w == 0 { -1.0f32 } else { 1.0f32 };
            let mut prev_wx = snout_x as i32;
            let mut prev_wy = snout_y as i32;

            for seg in 1..=15 {
                let st = seg as f32 / 15.0;
                let drift = (time_sec * 6.0 + st * 3.0 + w as f32).sin() * 12.0;
                let cur_wx = (snout_x - forward_x * (st * 70.0) + right_x * (w_side * (8.0 + st * 15.0) + drift)) as i32;
                let cur_wy = (snout_y - forward_y * (st * 70.0) + right_y * (w_side * (8.0 + st * 15.0) + drift)) as i32;
                gfx.draw_line(prev_wx, prev_wy, cur_wx, cur_wy, Color::rgb(0, 210, 200));
                if seg == 15 {
                    gfx.fill_circle(cur_wx, cur_wy, 3, Color::rgb(80, 255, 240));
                    gfx.fill_circle(cur_wx, cur_wy, 1, Color::rgb(255, 255, 255));
                }
                prev_wx = cur_wx;
                prev_wy = cur_wy;
            }
        }
    }

    fn render_hud(&self, gfx: &mut Graphics, fps: f32, frame_count: u64) {
        let width = gfx.width();
        let height = gfx.height();

        // Top title bar
        gfx.fill_rect(20, 12, width as i32 - 40, 32, Color::rgba(5, 18, 30, 200));
        gfx.draw_rect(20, 12, width as i32 - 40, 32, Color::rgb(0, 180, 210));

        let title = "L E V I A T H A N   O S   -   R U S T   U I   /   6 0   F P S   S C E N E R Y";
        let text_x = (width as i32 - (title.len() as i32 * 8)) / 2;
        gfx.draw_text(text_x, 23, title, Color::rgb(0, 255, 230), None);

        // Bottom-left telemetry card
        let card_w = 440;
        let card_h = 76;
        let card_x = 25;
        let card_y = height as i32 - card_h - 18;

        gfx.fill_rect(card_x, card_y, card_w, card_h, Color::rgba(3, 14, 25, 220));
        gfx.draw_rect(card_x, card_y, card_w, card_h, Color::rgb(0, 160, 190));

        let perf_str = format!("PERFORMANCE: {:.1} FPS | FRAME: {} (Rust 60Hz Engine)", fps, frame_count);
        gfx.draw_text(card_x + 12, card_y + 10, &perf_str, Color::rgb(80, 255, 140), None);

        let hw_str = format!("HARDWARE:    SimpleDRM Framebuffer ({} x {} @ {} bpp)", gfx.width(), gfx.height(), gfx.bpp());
        gfx.draw_text(card_x + 12, card_y + 26, &hw_str, Color::rgb(140, 210, 230), None);

        gfx.draw_text(card_x + 12, card_y + 42, "CONSOLE:     VT Text Muted (KD_GRAPHICS Active)", Color::rgb(140, 210, 230), None);
        gfx.draw_text(card_x + 12, card_y + 58, "ENGINE:      Pure Safe Rust Userspace UI Subsystem", Color::rgb(0, 240, 220), None);

        // Bottom-right interactive exit prompt
        let exit_msg = "Press [Q], [ESC], or [ENTER] to exit to shell";
        let exit_x = width as i32 - (exit_msg.len() as i32 * 8) - 35;
        gfx.fill_rect(exit_x - 10, height as i32 - 36, exit_msg.len() as i32 * 8 + 20, 22, Color::rgba(3, 14, 25, 220));
        gfx.draw_rect(exit_x - 10, height as i32 - 36, exit_msg.len() as i32 * 8 + 20, 22, Color::rgb(0, 140, 180));
        gfx.draw_text(exit_x, height as i32 - 29, exit_msg, Color::rgb(180, 230, 245), None);
    }
}
