use crate::color::Color;
use crate::graphics::Graphics;
use crate::prototype::abyss_scene::AbyssScene;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    Awakening,
    Crossing,
    Transition,
    Completed,
}

pub struct BootScreen {
    pub time: f32,
    pub stage: BootStage,
    pub stage_progress: f32,
    pub login_blend: f32,
    pub reduced_motion: bool,
    pub auto_loop: bool,
}

impl BootScreen {
    pub const AWAKENING_DURATION: f32 = 3.5;
    pub const CROSSING_DURATION: f32 = 6.0;
    pub const TRANSITION_DURATION: f32 = 2.5;

    pub fn new() -> Self {
        Self {
            time: 0.0,
            stage: BootStage::Awakening,
            stage_progress: 0.0,
            login_blend: 0.0,
            reduced_motion: false,
            auto_loop: false,
        }
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.stage = BootStage::Awakening;
        self.stage_progress = 0.0;
        self.login_blend = 0.0;
    }

    pub fn skip(&mut self) {
        self.time = Self::AWAKENING_DURATION + Self::CROSSING_DURATION + Self::TRANSITION_DURATION;
        self.stage = BootStage::Completed;
        self.stage_progress = 1.0;
        self.login_blend = 1.0;
    }

    pub fn is_completed(&self) -> bool {
        self.stage == BootStage::Completed
    }

    pub fn update(&mut self, dt: f32) {
        let step = if self.reduced_motion { dt * 0.7 } else { dt };
        self.time += step;

        let total_awakening = Self::AWAKENING_DURATION;
        let total_crossing = total_awakening + Self::CROSSING_DURATION;
        let total_boot = total_crossing + Self::TRANSITION_DURATION;

        if self.time < total_awakening {
            self.stage = BootStage::Awakening;
            self.stage_progress = (self.time / total_awakening).clamp(0.0, 1.0);
            self.login_blend = 0.0;
        } else if self.time < total_crossing {
            self.stage = BootStage::Crossing;
            self.stage_progress = ((self.time - total_awakening) / Self::CROSSING_DURATION).clamp(0.0, 1.0);
            self.login_blend = 0.0;
        } else if self.time < total_boot {
            self.stage = BootStage::Transition;
            self.stage_progress = ((self.time - total_crossing) / Self::TRANSITION_DURATION).clamp(0.0, 1.0);
            self.login_blend = self.stage_progress;
        } else {
            if self.auto_loop {
                self.reset();
            } else {
                self.stage = BootStage::Completed;
                self.stage_progress = 1.0;
                self.login_blend = 1.0;
            }
        }
    }

    pub fn render(&self, scene: &AbyssScene, gfx: &mut Graphics) {
        let w = scene.width as i32;
        let h = scene.height as i32;

        let (ocean_alpha, leviathan_alpha, text_alpha, levi_progress) = match self.stage {
            BootStage::Awakening => {
                let p = self.stage_progress;
                let ocean = (p * 1.2).clamp(0.0, 1.0);
                let levi = if p > 0.4 { ((p - 0.4) / 0.6).clamp(0.0, 0.45) } else { 0.0 };
                let txt = if p > 0.6 { ((p - 0.6) / 0.4).clamp(0.0, 0.7) } else { 0.0 };
                let pos = p * 0.22;
                (ocean, levi, txt, pos)
            }
            BootStage::Crossing => {
                let p = self.stage_progress;
                let ocean = 1.0;
                let levi = 0.85;
                let txt = 1.0;
                let pos = 0.22 + p * 0.58;
                (ocean, levi, txt, pos)
            }
            BootStage::Transition => {
                let p = self.stage_progress;
                let ocean = 1.0;
                let levi = ((1.0 - p * 1.2) * 0.85).clamp(0.0, 0.85);
                let txt = ((1.0 - p * 1.5)).clamp(0.0, 1.0);
                let pos = 0.80 + p * 0.25;
                (ocean, levi, txt, pos)
            }
            BootStage::Completed => {
                (1.0, 0.0, 0.0, 1.05)
            }
        };

        // 1. Abyss Background & Volumetric Light
        scene.render_background(gfx, self.time, ocean_alpha);

        // 2. Colossal Leviathan Silhouette
        scene.render_leviathan(gfx, self.time, leviathan_alpha, levi_progress);

        // 3. Floating Marine Snow and Bioluminescent Motes
        scene.render_particles(gfx, self.time, ocean_alpha);

        // 4. Logo and Typography
        if text_alpha > 0.01 {
            let cx = w / 2;
            let cy = h / 2 - (h / 14);

            // Subtle logo sizing based on screen resolution
            let logo_size = (h as f32 * 0.16).clamp(96.0, 180.0) as u32;
            scene.render_logo(gfx, cx, cy - 30, logo_size, logo_size, text_alpha);

            // Title: "LEVIATHAN OS"
            let title = "LEVIATHAN OS";
            let title_scale = if w >= 1600 { 3 } else { 2 };
            let title_char_w = 8 * (title_scale as i32);
            let title_w = (title.len() as i32) * title_char_w;
            let title_x = cx - title_w / 2;
            let title_y = cy + (logo_size as i32 / 2) - 10;

            let title_fg = Color::new_rgba(220, 235, 255, (text_alpha * 240.0) as u8);
            let title_glow = Color::new_rgba(20, 90, 170, (text_alpha * 120.0) as u8);
            gfx.draw_text_shadow(title_x, title_y, title, title_scale, title_fg, title_glow);

            // Tagline: "The ocean is vast. Cross it with Leviathan."
            let tagline = "The ocean is vast. Cross it with Leviathan.";
            let tag_scale = 1;
            let tag_char_w = 8 * (tag_scale as i32);
            let tag_w = (tagline.len() as i32) * tag_char_w;
            let tag_x = cx - tag_w / 2;
            let tag_y = title_y + 36;

            let tag_fg = Color::new_rgba(140, 175, 215, (text_alpha * 210.0) as u8);
            let tag_shadow = Color::new_rgba(4, 12, 30, (text_alpha * 200.0) as u8);
            gfx.draw_text_shadow(tag_x, tag_y, tagline, tag_scale, tag_fg, tag_shadow);

            // Minimalist Bioluminescent Boot Progress Bar
            let bar_w = (w / 4).clamp(180, 360);
            let bar_h = 2;
            let bar_x = cx - bar_w / 2;
            let bar_y = tag_y + 34;

            // Bar track
            let track_col = Color::new_rgba(15, 30, 60, (text_alpha * 160.0) as u8);
            gfx.fill_rect(bar_x, bar_y, bar_w, bar_h, track_col);

            // Bar fill (overall boot progress)
            let total_dur = Self::AWAKENING_DURATION + Self::CROSSING_DURATION + Self::TRANSITION_DURATION;
            let progress_ratio = (self.time / total_dur).clamp(0.0, 1.0);
            let fill_w = ((bar_w as f32) * progress_ratio) as i32;

            let fill_col = Color::new_rgba(0, 195, 240, (text_alpha * 230.0) as u8);
            gfx.fill_rect(bar_x, bar_y, fill_w, bar_h, fill_col);

            // Subtle leading head glow
            if fill_w > 0 && fill_w < bar_w {
                let glow_col = Color::new_rgba(160, 230, 255, (text_alpha * 255.0) as u8);
                gfx.fill_circle(bar_x + fill_w, bar_y + 1, 3, glow_col);
            }
        }

        // Stage indicator & controls hint (restrained in bottom corner)
        let hint_y = h - 28;
        let hint_col = Color::new_rgba(70, 95, 130, 160);
        let hint_text = match self.stage {
            BootStage::Awakening => "[Stage A: Awakening] Press [Space] to skip | [R] replay",
            BootStage::Crossing => "[Stage B: The Crossing] Press [Space] to skip | [R] replay",
            BootStage::Transition => "[Stage C: Transitioning into Abyss Login...]",
            BootStage::Completed => "[Boot Sequence Completed]",
        };
        gfx.draw_text(24, hint_y, hint_text, hint_col, None);
    }
}
