use crate::animation3d::Leviathan3DScene;
use crate::camera::Camera;
use crate::color::Color;
use crate::graphics::Graphics;
use crate::render3d::Renderer3D;

pub struct Scene {
    pub bg_cache: Vec<u8>,
    pub renderer3d: Renderer3D,
    pub camera: Camera,
    pub animation3d: Leviathan3DScene,
}

impl Scene {
    pub fn new(gfx: &mut Graphics) -> Self {
        let width = gfx.width();
        let height = gfx.height();

        // 1. Pre-render deep ocean atmospheric background into backbuffer cache
        println!("leviathan-ui: Pre-rendering deep ocean background cache...");

        // Vertical abyssal gradient
        for y in 0..height {
            let ty = y as f32 / height as f32;
            let r = (8.0 * (1.0 - ty) * (1.0 - ty) + 3.0 * (1.0 - ty) * ty * 2.0 + 1.0 * ty) as u8;
            let g = (38.0 * (1.0 - ty) + 10.0 * ty) as u8;
            let b = (72.0 * (1.0 - ty) + 18.0 * ty) as u8;
            let row_col = Color::rgb(r, g, b);
            for x in 0..width {
                gfx.put_pixel(x as i32, y as i32, row_col);
            }
        }

        // Volumetric Light shafts / abyssal sunbeams filtering from surface
        let rays = [
            (160.0f32, 65.0f32,  0.34f32, 0.12f32),
            (360.0f32, 90.0f32,  0.40f32, 0.18f32),
            (570.0f32, 115.0f32, 0.36f32, 0.15f32),
            (810.0f32, 80.0f32,  0.43f32, 0.12f32),
            (1050.0f32, 95.0f32, 0.38f32, 0.14f32),
        ];

        for &(start_x, w_top, slant, intensity) in &rays {
            let max_y = (height as f32 * 0.75) as i32;
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
                    let ray_c = Color::rgb(70, 180, 225);
                    gfx.put_pixel(x, y, Color::blend(cur, ray_c, alpha));
                }
            }
        }

        // Seafloor oceanic ridge silhouettes
        for x in 0..width as i32 {
            let fx = x as f32 * 0.007;
            let hill_y = (height as f32 - 100.0 + (fx * 3.1).sin() * 30.0 + (fx * 1.7).cos() * 22.0) as i32;
            for y in hill_y..height as i32 {
                let cur = gfx.get_pixel(x, y);
                let rock = Color::rgb(2, 12, 22);
                gfx.put_pixel(x, y, Color::blend(cur, rock, 0.88));
            }
        }

        let bg_cache = gfx.get_backbuffer().to_vec();

        // 2. Initialize 3D Engine subsystems
        println!("leviathan-ui: Generating 3D procedural meshes & skeletal matrices...");
        let renderer3d = Renderer3D::new(gfx);
        let camera = Camera::new();
        let animation3d = Leviathan3DScene::new();

        Self {
            bg_cache,
            renderer3d,
            camera,
            animation3d,
        }
    }

    pub fn update(&mut self, _width: u32, _height: u32, time_sec: f32) {
        // 1. Update 3D animation simulation
        self.animation3d.update(time_sec);

        // 2. Update dynamic 3D camera
        let head_pos = self.animation3d.segments[0].pos;
        let head_forward = self.animation3d.segments[0].forward;
        self.camera.update(time_sec, head_pos, head_forward);
    }

    pub fn render(
        &mut self,
        gfx: &mut Graphics,
        time_sec: f32,
        fps: f32,
        frame_count: u64,
    ) {
        let width = gfx.width();
        let height = gfx.height();

        // 1. Fast-blit background cache (~0.3 ms)
        gfx.copy_to_backbuffer(&self.bg_cache);

        // 2. Clear 3D Z-buffer (~0.2 ms)
        self.renderer3d.clear_depth();

        // 3. Render complete 3D animated scene
        self.animation3d.render(gfx, &mut self.renderer3d, &self.camera, time_sec);

        // 4. Render sleek HUD Overlay with 3D telemetry
        self.render_hud(gfx, width, height, fps, frame_count);
    }

    fn render_hud(
        &self,
        gfx: &mut Graphics,
        width: u32,
        height: u32,
        fps: f32,
        frame_count: u64,
    ) {
        // Top Title Bar
        gfx.fill_rect(20, 10, width as i32 - 40, 30, Color::rgba(3, 14, 25, 215));
        gfx.draw_rect(20, 10, width as i32 - 40, 30, Color::rgb(0, 190, 220));

        let title = "L E V I A T H A N   O S   -   3 D   A N I M A T I O N   R E N D E R E R   ( 6 0   F P S )";
        let text_x = (width as i32 - (title.len() as i32 * 8)) / 2;
        gfx.draw_text(text_x, 20, title, Color::rgb(0, 255, 235), None);

        // Bottom-Left Telemetry Card
        let card_w = 460;
        let card_h = 92;
        let card_x = 25;
        let card_y = height as i32 - card_h - 18;

        gfx.fill_rect(card_x, card_y, card_w, card_h, Color::rgba(2, 12, 22, 225));
        gfx.draw_rect(card_x, card_y, card_w, card_h, Color::rgb(0, 170, 200));

        let perf_str = format!("PERFORMANCE: {:.1} FPS | FRAME: {} (Z-Buffer Active)", fps, frame_count);
        gfx.draw_text(card_x + 12, card_y + 10, &perf_str, Color::rgb(80, 255, 140), None);

        let cam_str = format!("CAMERA [C]:  {}", self.camera.mode.name());
        gfx.draw_text(card_x + 12, card_y + 26, &cam_str, Color::rgb(140, 225, 245), None);

        let shade_str = format!("SHADING [L]: {}", self.renderer3d.shading_mode.name());
        gfx.draw_text(card_x + 12, card_y + 42, &shade_str, Color::rgb(140, 225, 245), None);

        let mode_name = match self.animation3d.scene_mode {
            1 => "3D Abyssal Leviathan Kinematics",
            2 => "3D Geometric Polyhedra Showcase",
            3 => "3D Holographic Cyber Grid",
            _ => "Custom",
        };
        let scene_str = format!("SCENE [1/2/3]: {}", mode_name);
        gfx.draw_text(card_x + 12, card_y + 58, &scene_str, Color::rgb(0, 245, 225), None);

        let pause_str = if self.animation3d.is_paused { "[PAUSED]" } else { "[RUNNING 60Hz]" };
        let engine_str = format!("PIPELINE:    Software 3D Rasterizer + Fog {}", pause_str);
        gfx.draw_text(card_x + 12, card_y + 74, &engine_str, Color::rgb(255, 210, 80), None);

        // Bottom-Right Controls Prompt
        let ctrl_msg = "[C] Cam | [L] Shading | [1-3] Scene | [SPACE] Pause | [Q] Exit";
        let ctrl_x = width as i32 - (ctrl_msg.len() as i32 * 8) - 35;
        gfx.fill_rect(ctrl_x - 10, height as i32 - 36, ctrl_msg.len() as i32 * 8 + 20, 22, Color::rgba(2, 12, 22, 225));
        gfx.draw_rect(ctrl_x - 10, height as i32 - 36, ctrl_msg.len() as i32 * 8 + 20, 22, Color::rgb(0, 150, 190));
        gfx.draw_text(ctrl_x, height as i32 - 29, ctrl_msg, Color::rgb(180, 235, 250), None);
    }
}
