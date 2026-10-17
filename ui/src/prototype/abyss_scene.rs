use crate::color::Color;
use crate::graphics::Graphics;
use crate::image::Image;
use std::cell::{Cell, RefCell};

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub speed_y: f32,
    pub speed_x: f32,
    pub size: i32,
    pub alpha: f32,
    pub pulse_offset: f32,
    pub is_bioluminescent: bool,
}

pub struct AbyssScene {
    pub width: u32,
    pub height: u32,
    pub particles: Vec<Particle>,
    pub logo: Option<Image>,
    pub logo_attempted: bool,
    pub background: Option<Image>,
    pub background_attempted: bool,
    pub cached_bg: RefCell<Option<Vec<u8>>>,
    pub cached_bg_size: Cell<(u32, u32)>,
    pub reduced_motion: bool,
}

impl AbyssScene {
    pub fn new(width: u32, height: u32) -> Self {
        let mut particles = Vec::with_capacity(120);

        // Marine snow + rising bioluminescent motes
        for i in 0..120 {
            let is_bio = i % 4 == 0;
            let seed = (i as f32) * 17.31;
            particles.push(Particle {
                x: (seed.sin().abs() * (width as f32)).clamp(0.0, width as f32),
                y: ((seed * 2.3).cos().abs() * (height as f32)).clamp(0.0, height as f32),
                speed_y: if is_bio {
                    -0.25 - (seed % 0.4)
                } else {
                    0.15 + (seed % 0.35)
                },
                speed_x: ((seed * 1.7).sin() * 0.2),
                size: if is_bio { (i % 3) + 2 } else { (i % 2) + 1 },
                alpha: 0.2 + ((seed * 0.5).sin().abs() * 0.6),
                pulse_offset: seed % 6.28,
                is_bioluminescent: is_bio,
            });
        }

        let mut scene = Self {
            width,
            height,
            particles,
            logo: None,
            logo_attempted: false,
            background: None,
            background_attempted: false,
            cached_bg: RefCell::new(None),
            cached_bg_size: Cell::new((0, 0)),
            reduced_motion: false,
        };

        scene.try_load_logo();
        scene.try_load_background();
        scene
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        *self.cached_bg.borrow_mut() = None;
        self.cached_bg_size.set((0, 0));
    }

    fn try_load_logo(&mut self) {
        if self.logo_attempted {
            return;
        }
        self.logo_attempted = true;
        let candidates = [
            "assets/logo.png",
            "../assets/logo.png",
            "/home/achinthyaj/Documents/Projects/My Death/Leviathan/assets/logo.png",
            "/usr/share/leviathan/assets/logo.png",
            "/etc/leviathan/assets/logo.png",
        ];
        for path in &candidates {
            if let Ok(img) = Image::load_png(path) {
                self.logo = Some(img);
                break;
            }
        }
    }

    fn try_load_background(&mut self) {
        if self.background_attempted {
            return;
        }
        self.background_attempted = true;
        let candidates = [
            "assets/lock_background.png",
            "../assets/lock_background.png",
            "/home/achinthyaj/Documents/Projects/My Death/Leviathan/assets/lock_background.png",
            "/usr/share/leviathan/assets/lock_background.png",
            "/etc/leviathan/assets/lock_background.png",
        ];
        for path in &candidates {
            if let Ok(img) = Image::load_png(path) {
                self.background = Some(img);
                break;
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.reduced_motion {
            return;
        }
        let w = self.width as f32;
        let h = self.height as f32;

        for p in &mut self.particles {
            p.y += p.speed_y * dt * 60.0;
            p.x += p.speed_x * dt * 60.0;

            if p.y < -10.0 {
                p.y = h + 10.0;
                p.x = ((p.x * 1.3) % w).abs();
            } else if p.y > h + 10.0 {
                p.y = -10.0;
                p.x = ((p.x * 1.3) % w).abs();
            }

            if p.x < 0.0 {
                p.x += w;
            } else if p.x > w {
                p.x -= w;
            }
        }
    }

    pub fn render_background(&self, gfx: &mut Graphics, time: f32, opacity: f32) {
        if opacity <= 0.0 {
            gfx.clear(Color::new(0, 0, 0));
            return;
        }

        let w = self.width as i32;
        let h = self.height as i32;

        // Try rendering lock_background.png with instant cached blit
        if let Some(bg_img) = &self.background {
            let gw = gfx.width();
            let gh = gfx.height();

            let mut cache = self.cached_bg.borrow_mut();
            if cache.is_none() || self.cached_bg_size.get() != (gw, gh) {
                // Pre-render the scaled wallpaper + darkness overlay + vignette ONCE
                let mut temp_gfx = Graphics::new_for_wayland(gw, gh);
                let gpitch = temp_gfx.pitch();
                bg_img.draw_to_buffer_scaled(
                    temp_gfx.backbuffer_mut(),
                    gw,
                    gh,
                    gpitch,
                    0,
                    0,
                    gw,
                    gh,
                    1.0,
                );

                // Dark overlay to keep UI components prominent, glassy, and cast a deep purplish ambient depth
                let overlay_alpha = 0.52;
                temp_gfx.fill_rect(0, 0, gw as i32, gh as i32, Color::new_rgba(16, 6, 26, (overlay_alpha * 255.0) as u8));

                // Subtle smooth vignette along screen edges
                let vig_steps = 12;
                for step in 0..vig_steps {
                    let t = step as f32 / vig_steps as f32;
                    let border = (t * 140.0) as i32;
                    let vig_alpha = (1.0 - t) * 0.28;
                    if vig_alpha > 0.005 {
                        let col = Color::new_rgba(6, 0, 10, (vig_alpha * 255.0) as u8);
                        temp_gfx.fill_rect(0, 0, gw as i32, border.max(1), col);
                        temp_gfx.fill_rect(0, gh as i32 - border.max(1), gw as i32, border.max(1), col);
                        temp_gfx.fill_rect(0, 0, border.max(1), gh as i32, col);
                        temp_gfx.fill_rect(gw as i32 - border.max(1), 0, border.max(1), gh as i32, col);
                    }
                }

                *cache = Some(temp_gfx.get_backbuffer().to_vec());
                self.cached_bg_size.set((gw, gh));
            }

            if let Some(ref bg_bytes) = *cache {
                let dst = gfx.backbuffer_mut();
                let len = dst.len().min(bg_bytes.len());
                dst[..len].copy_from_slice(&bg_bytes[..len]);
                gfx.mark_dirty(0, 0, w, h);
                return;
            }
        }

        // --- Fallback: Procedural abyssal purplish gradient ---
        let step = (h / 32).max(1);
        for y in (0..h).step_by(step as usize) {
            let t = (y as f32 / h as f32).clamp(0.0, 1.0);
            let r = (6.0 + t * 14.0) * opacity;
            let g = (2.0 + t * 5.0) * opacity;
            let b = (16.0 + t * 32.0) * opacity;
            gfx.fill_rect(0, y, w, step + 1, Color::new(r as u8, g as u8, b as u8));
        }

        // Soft volumetric caustics / shafts of deep water light from upper-left
        let light_bands = 4;
        for i in 0..light_bands {
            let band_offset = (i as f32) * 220.0 + (time * 12.0 * (1.0 + i as f32 * 0.2));
            let start_x = ((band_offset) % (w as f32 * 1.4) - 200.0) as i32;
            let end_x = start_x + 320;
            let ray_alpha = ((0.02 + 0.015 * ((time * 0.8 + i as f32).sin())) * opacity).clamp(0.0, 0.06);

            for dy in (0..h).step_by(8) {
                let shift = (dy as f32 * 0.35) as i32;
                let cur_alpha = ray_alpha * (1.0 - (dy as f32 / h as f32) * 0.75);
                if cur_alpha > 0.005 {
                    gfx.fill_rect(
                        start_x + shift,
                        dy,
                        end_x - start_x,
                        8,
                        Color::new_rgba(85, 45, 140, (cur_alpha * 255.0) as u8),
                    );
                }
            }
        }
    }

    pub fn render_particles(&self, gfx: &mut Graphics, time: f32, opacity: f32) {
        if opacity <= 0.0 {
            return;
        }

        for p in &self.particles {
            let mut alpha = p.alpha * opacity;
            if p.is_bioluminescent {
                let pulse = ((time * 3.0 + p.pulse_offset).sin() + 1.0) * 0.5;
                alpha *= 0.6 + pulse * 0.4;
                let color = Color::new_rgba(175, 105, 255, (alpha.clamp(0.0, 1.0) * 255.0) as u8);
                gfx.fill_circle(p.x as i32, p.y as i32, p.size, color);
            } else {
                let color = Color::new_rgba(125, 95, 165, (alpha.clamp(0.0, 1.0) * 255.0) as u8);
                gfx.fill_circle(p.x as i32, p.y as i32, p.size, color);
            }
        }
    }

    /// Renders the colossal silhouette of the Leviathan creature gliding through the water.
    ///
    /// It uses a serpentine multi-segment spine model with variable body radii,
    /// dorsal fins, pectoral fins, a flowing caudal tail fluke, glowing bioluminescent
    /// photophores along the spine, and an ocean-mist atmospheric fog pass.
    pub fn render_leviathan(&self, gfx: &mut Graphics, time: f32, global_alpha: f32, progress: f32) {
        if global_alpha <= 0.0 {
            return;
        }

        let w = self.width as f32;
        let h = self.height as f32;

        // Base horizontal movement: emerges from left and glides towards right
        // progress: 0.0 (far left offscreen) -> 1.0 (far right exiting into mist)
        let creature_total_len = w * 0.85;
        let head_x = -creature_total_len * 0.3 + progress * (w + creature_total_len * 0.6);
        let base_y = h * 0.42 + (time * 0.4).sin() * 28.0;

        let num_segments = 55;
        let segment_spacing = creature_total_len / (num_segments as f32);

        let mut spine_points: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(num_segments);

        for i in 0..num_segments {
            let seg_idx = i as f32;
            let t_seg = seg_idx / (num_segments as f32);

            // Sinusoidal undulating body wave travelling backwards along the spine
            let wave_phase = time * 2.2 - seg_idx * 0.16;
            let undulation_amp = 18.0 + t_seg * 38.0;
            let seg_y = base_y + (wave_phase).sin() * undulation_amp;
            let seg_x = head_x - seg_idx * segment_spacing;

            // Creature body radius profile:
            // Tapered snout -> massive muscular chest/mid-body -> elegant tapering tail
            let radius = if t_seg < 0.15 {
                18.0 + (t_seg / 0.15) * 38.0 // head to pectoral chest
            } else if t_seg < 0.55 {
                56.0 - ((t_seg - 0.15) / 0.40) * 16.0 // massive torso
            } else if t_seg < 0.85 {
                40.0 - ((t_seg - 0.55) / 0.30) * 26.0 // tail stalk
            } else {
                14.0 - ((t_seg - 0.85) / 0.15) * 8.0 // caudal fluke base
            };

            // Calculate tangent angle for dorsal fin placement
            let tangent = if i > 0 {
                let prev = spine_points[i - 1];
                (seg_y - prev.1).atan2(seg_x - prev.0)
            } else {
                0.0
            };

            spine_points.push((seg_x, seg_y, radius, tangent));
        }

        // 1. Draw Tail Flukes / Fins (under body layer)
        if spine_points.len() > 10 {
            // Caudal tail fluke at the very back (last segment)
            let tail_tip = spine_points[spine_points.len() - 1];
            let tail_fin_span = 48.0;
            let fin_wave = (time * 2.2 - (num_segments as f32) * 0.16).cos();

            let fluke_alpha = (0.28 * global_alpha).clamp(0.0, 1.0);
            let fluke_color = Color::new_rgba(4, 14, 30, (fluke_alpha * 255.0) as u8);

            gfx.fill_circle((tail_tip.0 - 12.0) as i32, (tail_tip.1 - tail_fin_span * 0.7 * fin_wave) as i32, 16, fluke_color);
            gfx.fill_circle((tail_tip.0 - 12.0) as i32, (tail_tip.1 + tail_fin_span * 0.7 * fin_wave) as i32, 16, fluke_color);

            // Dorsal crests along mid-spine
            for idx in [12, 20, 28, 36] {
                if idx < spine_points.len() {
                    let pt = spine_points[idx];
                    let crest_height = pt.2 * 1.3;
                    let crest_x = (pt.0 + 4.0) as i32;
                    let crest_y = (pt.1 - crest_height) as i32;
                    let crest_col = Color::new_rgba(6, 18, 38, (0.35 * global_alpha * 255.0) as u8);
                    gfx.fill_circle(crest_x, crest_y, (pt.2 * 0.45) as i32, crest_col);
                }
            }
        }

        // 2. Draw Body Segments (Massive Deep Silhouette)
        for i in (0..spine_points.len()).rev() {
            let (sx, sy, rad, _) = spine_points[i];
            let t_seg = i as f32 / (num_segments as f32);

            // Body color: mysterious near-black abyssal silhouette
            // Distance fog blends it into background navy
            let fog_t = ((sx / w).clamp(0.0, 1.0) - 0.5).abs() * 0.25;
            let body_r = (2.0 + fog_t * 8.0) as u8;
            let body_g = (6.0 + fog_t * 14.0) as u8;
            let body_b = (16.0 + fog_t * 26.0) as u8;
            let body_alpha = (0.65 * global_alpha).clamp(0.0, 1.0);

            let body_color = Color::new_rgba(body_r, body_g, body_b, (body_alpha * 255.0) as u8);
            gfx.fill_circle(sx as i32, sy as i32, rad as i32, body_color);

            // Bioluminescent nodes along lateral spine line (subtle cyan/violet photophores)
            if i % 3 == 0 && t_seg > 0.08 && t_seg < 0.88 {
                let bio_pulse = ((time * 2.8 + i as f32 * 0.4).sin() + 1.0) * 0.5;
                let bio_alpha = (0.35 + bio_pulse * 0.45) * global_alpha;

                // Alternate between ethereal cyan and violet bioluminescence
                let bio_col = if (i / 3) % 2 == 0 {
                    Color::new_rgba(0, 190, 230, (bio_alpha * 255.0) as u8)
                } else {
                    Color::new_rgba(140, 80, 220, (bio_alpha * 255.0) as u8)
                };

                let dot_x = sx as i32;
                let dot_y = (sy - rad * 0.15) as i32;
                gfx.fill_circle(dot_x, dot_y, 3, bio_col);
            }
        }

        // 3. Head details: Glowing deep eye in the darkness
        if !spine_points.is_empty() {
            let head = spine_points[0];
            let eye_x = (head.0 + head.2 * 0.45) as i32;
            let eye_y = (head.1 - head.2 * 0.2) as i32;

            let eye_pulse = ((time * 1.8).sin() + 1.0) * 0.5;
            let eye_alpha = (0.55 + eye_pulse * 0.4) * global_alpha;

            // Outer ethereal glow
            gfx.fill_circle(eye_x, eye_y, 6, Color::new_rgba(0, 180, 220, (eye_alpha * 0.35 * 255.0) as u8));
            // Inner eye core
            gfx.fill_circle(eye_x, eye_y, 3, Color::new_rgba(180, 240, 255, (eye_alpha * 255.0) as u8));
        }

        // 4. Atmospheric Haze Overlay (Soft ocean mist in front of the creature)
        let mist_alpha = (0.12 * global_alpha).clamp(0.0, 0.25);
        let mist_col = Color::new_rgba(5, 15, 35, (mist_alpha * 255.0) as u8);
        gfx.fill_rect(0, (base_y - 120.0) as i32, w as i32, 240, mist_col);
    }

    /// Renders the official Leviathan logo from assets/logo.png or sleek fallback emblem
    pub fn render_logo(&self, gfx: &mut Graphics, cx: i32, cy: i32, target_w: u32, target_h: u32, alpha: f32) {
        if alpha <= 0.0 {
            return;
        }

        let dst_x = cx - (target_w as i32) / 2;
        let dst_y = cy - (target_h as i32) / 2;

        if let Some(logo_img) = &self.logo {
            let gw = gfx.width();
            let gh = gfx.height();
            let gpitch = gfx.pitch();
            logo_img.draw_to_buffer_scaled(
                gfx.backbuffer_mut(),
                gw,
                gh,
                gpitch,
                dst_x,
                dst_y,
                target_w,
                target_h,
                alpha,
            );
            gfx.mark_dirty(dst_x, dst_y, target_w as i32, target_h as i32);
        } else {
            // Elegant procedural geometric emblem fallback
            let r = (target_w / 2) as i32;
            gfx.draw_circle(cx, cy, r, Color::new_rgba(185, 115, 255, (alpha * 220.0) as u8));
            gfx.draw_circle(cx, cy, r - 6, Color::new_rgba(150, 90, 230, (alpha * 180.0) as u8));
            gfx.fill_circle(cx, cy, 8, Color::new_rgba(255, 255, 255, (alpha * 240.0) as u8));
        }
    }
}
