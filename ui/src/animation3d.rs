use crate::camera::Camera;
use crate::color::Color;
use crate::graphics::Graphics;
use crate::math3d::{Mat4, Vec3};
use crate::mesh::Mesh;
use crate::render3d::{Renderer3D, ShadingMode};
use std::f32::consts::PI;

pub const NUM_3D_SEGMENTS: usize = 34;
pub const NUM_3D_BUBBLES: usize = 50;
pub const NUM_3D_SNOW: usize = 70;

#[derive(Clone, Copy)]
pub struct Segment3D {
    pub pos: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub radius_x: f32,
    pub radius_y: f32,
}

#[derive(Clone, Copy)]
pub struct Particle3D {
    pub pos: Vec3,
    pub vel: Vec3,
    pub radius: f32,
    pub color: Color,
    pub wobble_phase: f32,
}

pub struct Leviathan3DScene {
    // 3D Meshes
    pub head_mesh: Mesh,
    pub vertebra_meshes: Vec<Mesh>,
    pub fin_mesh: Mesh,
    pub crystal_mesh: Mesh,
    pub torus_mesh: Mesh,
    pub sphere_mesh: Mesh,
    pub cube_mesh: Mesh,

    // Animation state
    pub segments: [Segment3D; NUM_3D_SEGMENTS],
    pub bubbles: [Particle3D; NUM_3D_BUBBLES],
    pub snow: [Particle3D; NUM_3D_SNOW],

    // Monoliths on seabed
    pub monolith_positions: [Vec3; 4],

    // Scene mode: 1 = Leviathan Abyss, 2 = 3D Geometry Showcase, 3 = Holographic Grid
    pub scene_mode: u32,
    pub is_paused: bool,
}

impl Leviathan3DScene {
    pub fn new() -> Self {
        println!("leviathan-ui: Generating 3D procedural meshes & skeletal matrices...");

        let primary_color = Color::rgb(8, 42, 58);
        let belly_color = Color::rgb(18, 98, 104);
        let rune_color = Color::rgb(0, 240, 215);

        let head_mesh = Mesh::create_dragon_head(1.9, primary_color, rune_color);
        let fin_mesh = Mesh::create_fin(6.0, 3.0, Color::rgb(0, 195, 215));
        let crystal_mesh = Mesh::create_crystal(10.0, 2.6, Color::rgb(0, 235, 210));
        let torus_mesh = Mesh::create_torus(10.0, 3.0, 18, 9, Color::rgb(0, 185, 225));
        let sphere_mesh = Mesh::create_sphere(6.5, 10, 14, Color::rgb(20, 115, 145));
        let cube_mesh = Mesh::create_cube(5.0, Color::rgb(10, 50, 68));

        // Generate tapered body segments
        let mut vertebra_meshes = Vec::with_capacity(NUM_3D_SEGMENTS);
        for i in 0..NUM_3D_SEGMENTS {
            let progress = i as f32 / NUM_3D_SEGMENTS as f32;
            let envelope = (progress * PI).sin();

            let rx = if progress < 0.12 {
                2.8 + (progress / 0.12) * 1.1
            } else {
                1.1 + 3.2 * envelope.powf(0.7)
            };
            let ry = rx * 0.95;
            let dorsal_ridge = rx * 0.45;

            vertebra_meshes.push(Mesh::create_vertebra(
                rx,
                ry,
                2.3,
                dorsal_ridge,
                primary_color,
                belly_color,
            ));
        }

        let default_seg = Segment3D {
            pos: Vec3::ZERO,
            forward: Vec3::FORWARD,
            up: Vec3::UP,
            right: Vec3::X,
            radius_x: 2.5,
            radius_y: 2.5,
        };
        let segments = [default_seg; NUM_3D_SEGMENTS];

        // 3D Bubbles
        let mut bubbles = [Particle3D {
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            radius: 0.35,
            color: Color::rgb(110, 230, 250),
            wobble_phase: 0.0,
        }; NUM_3D_BUBBLES];

        for (i, b) in bubbles.iter_mut().enumerate() {
            let pr = (i * 7919 + 1013) % 1000;
            b.pos = Vec3::new(
                -35.0 + ((pr * 13) % 70) as f32,
                -16.0 + ((pr * 37) % 32) as f32,
                -25.0 + ((pr * 29) % 60) as f32,
            );
            b.vel = Vec3::new(
                -0.03 + (pr % 10) as f32 / 150.0,
                0.22 + (pr % 30) as f32 / 120.0,
                -0.03 + (pr % 10) as f32 / 150.0,
            );
            b.radius = 0.22 + (pr % 4) as f32 * 0.10;
            b.wobble_phase = (pr % 100) as f32 * 0.1;
        }

        // 3D Marine Snow
        let mut snow = [Particle3D {
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            radius: 0.12,
            color: Color::rgb(140, 210, 230),
            wobble_phase: 0.0,
        }; NUM_3D_SNOW];

        for (i, m) in snow.iter_mut().enumerate() {
            let pr = (i * 6271 + 3307) % 1000;
            m.pos = Vec3::new(
                -40.0 + ((pr * 17) % 80) as f32,
                -18.0 + ((pr * 41) % 36) as f32,
                -35.0 + ((pr * 23) % 75) as f32,
            );
            m.vel = Vec3::new(
                -0.03 + (pr % 8) as f32 / 150.0,
                -0.07 - (pr % 12) as f32 / 120.0,
                -0.03 + (pr % 8) as f32 / 150.0,
            );
            let b = 100 + (pr % 120) as u8;
            m.color = Color::rgb(b / 2, b, b);
            m.radius = 0.10 + (pr % 3) as f32 * 0.05;
            m.wobble_phase = (pr % 100) as f32 * 0.05;
        }

        let monolith_positions = [
            Vec3::new(-28.0, -18.0, 12.0),
            Vec3::new( 28.0, -18.0, -12.0),
            Vec3::new(-10.0, -18.0, -28.0),
            Vec3::new( 12.0, -18.0, 28.0),
        ];

        Self {
            head_mesh,
            vertebra_meshes,
            fin_mesh,
            crystal_mesh,
            torus_mesh,
            sphere_mesh,
            cube_mesh,
            segments,
            bubbles,
            snow,
            monolith_positions,
            scene_mode: 1,
            is_paused: false,
        }
    }

    pub fn update(&mut self, time_sec: f32) {
        if self.is_paused {
            return;
        }

        // 1. Update 3D Swimming Trajectory (smooth circular / lemniscate depth path)
        let swim_speed = 0.40;
        let t = time_sec * swim_speed;

        let head_x = 24.0 * (t * 0.9).sin();
        let head_y = 5.5 * (t * 1.5).sin();
        let head_z = 18.0 * (t * 0.9).cos();

        // Velocity derivatives for 3D heading
        let vx = 24.0 * 0.9 * (t * 0.9).cos();
        let vy = 5.5 * 1.5 * (t * 1.5).cos();
        let vz = -18.0 * 0.9 * (t * 0.9).sin();

        let forward = Vec3::new(vx, vy, vz).normalize();
        let mut right = Vec3::UP.cross(forward).normalize();
        let mut up = forward.cross(right);

        // Bank / Roll based on horizontal turning
        let roll = -vx * 0.03;
        let roll_mat = Mat4::rotation_axis(forward, roll);
        right = roll_mat.transform_dir(right);
        up = roll_mat.transform_dir(up);

        self.segments[0].pos = Vec3::new(head_x, head_y, head_z);
        self.segments[0].forward = forward;
        self.segments[0].up = up;
        self.segments[0].right = right;

        let seg_dist = 2.4;

        // Propagate 3D sinusoidal undulating wave backwards through spine
        for i in 1..NUM_3D_SEGMENTS {
            let progress = i as f32 / NUM_3D_SEGMENTS as f32;
            let wave_amp = 0.045 + 0.38 * progress;

            // Lateral wave (horizontal undulation)
            let yaw_wave = wave_amp * (i as f32 * 0.25 - time_sec * 3.8).sin();
            // Vertical pitch wave (gentle dorsal heave)
            let pitch_wave = (wave_amp * 0.30) * (i as f32 * 0.25 - time_sec * 3.8 + PI * 0.5).sin();

            let prev_fwd = self.segments[i - 1].forward;
            let prev_up = self.segments[i - 1].up;
            let prev_right = self.segments[i - 1].right;

            let cur_fwd = (prev_fwd + prev_right * yaw_wave + prev_up * pitch_wave).normalize();
            let cur_right = prev_up.cross(cur_fwd).normalize();
            let cur_up = cur_fwd.cross(cur_right);

            self.segments[i].pos = self.segments[i - 1].pos - cur_fwd * seg_dist;
            self.segments[i].forward = cur_fwd;
            self.segments[i].up = cur_up;
            self.segments[i].right = cur_right;
        }

        // 2. Update 3D Particles
        for (i, b) in self.bubbles.iter_mut().enumerate() {
            b.pos += b.vel;
            b.pos.x += (time_sec * 2.2 + b.wobble_phase).sin() * 0.03;
            b.pos.z += (time_sec * 1.8 + b.wobble_phase).cos() * 0.03;

            if b.pos.y > 18.0 {
                b.pos.y = -18.0;
                b.pos.x = -35.0 + ((i * 13) % 70) as f32;
                b.pos.z = -25.0 + ((i * 29) % 60) as f32;
            }
        }

        for (i, m) in self.snow.iter_mut().enumerate() {
            m.pos += m.vel;
            m.pos.x += (time_sec * 0.7 + m.wobble_phase).sin() * 0.02;

            if m.pos.y < -19.0 {
                m.pos.y = 19.0;
                m.pos.x = -40.0 + ((i * 17) % 80) as f32;
                m.pos.z = -35.0 + ((i * 23) % 75) as f32;
            }
        }
    }

    pub fn render(
        &self,
        gfx: &mut Graphics,
        r3d: &mut Renderer3D,
        camera: &Camera,
        time_sec: f32,
    ) {
        let aspect = gfx.width() as f32 / gfx.height() as f32;
        let view = camera.get_view_matrix();
        let proj = camera.get_projection_matrix(aspect);

        match self.scene_mode {
            1 => self.render_abyss_scene(gfx, r3d, &view, &proj, time_sec),
            2 => self.render_geometry_showcase(gfx, r3d, &view, &proj, time_sec),
            3 => self.render_hologram_grid(gfx, r3d, &view, &proj, time_sec),
            _ => {}
        }
    }

    /// Mode 1: The 3D Swimming Leviathan in the Abyssal Ocean
    fn render_abyss_scene(
        &self,
        gfx: &mut Graphics,
        r3d: &mut Renderer3D,
        view: &Mat4,
        proj: &Mat4,
        time_sec: f32,
    ) {
        // A. Render 3D Seabed Ancient Monoliths & Floating Power Crystals
        for (i, &pos) in self.monolith_positions.iter().enumerate() {
            let pedestal_scale = Mat4::scale(2.2, 5.0, 2.2);
            let pedestal_pos = Mat4::translation(pos.x, pos.y, pos.z);
            let pedestal_model = pedestal_pos.mul(&pedestal_scale);
            r3d.render_mesh(gfx, &self.cube_mesh, &pedestal_model, view, proj);

            let rot_crystal = Mat4::rotation_y(time_sec * 1.2 + i as f32 * 1.5)
                .mul(&Mat4::rotation_z(0.2));
            let hover_y = pos.y + 7.5 + (time_sec * 2.0 + i as f32).sin() * 0.8;
            let crystal_model = Mat4::translation(pos.x, hover_y, pos.z).mul(&rot_crystal);
            r3d.render_mesh(gfx, &self.crystal_mesh, &crystal_model, view, proj);

            let shaft_top = Vec3::new(pos.x, hover_y + 14.0, pos.z);
            let shaft_col = Color::rgb(0, 210, 230);
            r3d.draw_line_3d(gfx, Vec3::new(pos.x, hover_y, pos.z), shaft_top, shaft_col, view, proj, 0.005);
        }

        // B. Render the 3D Leviathan Vertebrae Body Chain
        for i in (1..NUM_3D_SEGMENTS).rev() {
            let seg = &self.segments[i];
            let rot_mat = Mat4 {
                m: [
                    seg.right.x,   seg.right.y,   seg.right.z,   0.0,
                    seg.up.x,      seg.up.y,      seg.up.z,      0.0,
                    seg.forward.x, seg.forward.y, seg.forward.z, 0.0,
                    seg.pos.x,     seg.pos.y,     seg.pos.z,     1.0,
                ],
            };

            if i < self.vertebra_meshes.len() {
                r3d.render_mesh(gfx, &self.vertebra_meshes[i], &rot_mat, view, proj);
            }

            // Dorsal Fin flutter
            if i >= 4 && i <= 24 && i % 3 == 0 {
                let fin_ripple = (i as f32 * 0.3 - time_sec * 4.2).sin() * 0.30;
                let fin_rot = Mat4::rotation_z(fin_ripple).mul(&Mat4::rotation_x(-0.2));
                let fin_base = seg.pos + seg.up * (seg.radius_y * 1.05);
                let fin_model = Mat4::translation(fin_base.x, fin_base.y, fin_base.z)
                    .mul(&rot_mat)
                    .mul(&fin_rot);
                r3d.render_mesh(gfx, &self.fin_mesh, &fin_model, view, proj);
            }

            // Bioluminescent Runes pulsing along flanks
            if i % 2 == 0 {
                let pulse = (time_sec * 3.5 + i as f32 * 0.25).sin() * 0.5 + 0.5;
                let rune_col = Color::rgb(0, (190.0 + 65.0 * pulse) as u8, 240);

                let r_left = seg.pos + seg.right * (seg.radius_x * 1.05);
                let r_right = seg.pos - seg.right * (seg.radius_x * 1.05);

                r3d.draw_point_3d(gfx, r_left, 0.40, rune_col, view, proj);
                r3d.draw_point_3d(gfx, r_right, 0.40, rune_col, view, proj);

                if i + 2 < NUM_3D_SEGMENTS {
                    let prev_l = self.segments[i + 2].pos + self.segments[i + 2].right * (self.segments[i + 2].radius_x * 1.05);
                    let prev_r = self.segments[i + 2].pos - self.segments[i + 2].right * (self.segments[i + 2].radius_x * 1.05);
                    r3d.draw_line_3d(gfx, r_left, prev_l, Color::rgb(0, 160, 200), view, proj, 0.003);
                    r3d.draw_line_3d(gfx, r_right, prev_r, Color::rgb(0, 160, 200), view, proj, 0.003);
                }
            }
        }

        // C. Render the 3D Sea Dragon Head
        {
            let head = &self.segments[0];
            let head_mat = Mat4 {
                m: [
                    head.right.x,   head.right.y,   head.right.z,   0.0,
                    head.up.x,      head.up.y,      head.up.z,      0.0,
                    head.forward.x, head.forward.y, head.forward.z, 0.0,
                    head.pos.x,     head.pos.y,     head.pos.z,     1.0,
                ],
            };
            r3d.render_mesh(gfx, &self.head_mesh, &head_mat, view, proj);

            // Flowing 3D Bioluminescent Whiskers
            let snout_tip = head.pos + head.forward * 4.0;
            for w in 0..2 {
                let side = if w == 0 { 1.0f32 } else { -1.0f32 };
                let mut prev_pt = snout_tip + head.right * (side * 0.8) - head.up * 0.4;

                for s in 1..=10 {
                    let st = s as f32 / 10.0;
                    let drift = (time_sec * 5.2 + st * 3.8 + w as f32 * PI).sin() * 0.75;
                    let cur_pt = snout_tip
                        - head.forward * (st * 10.0)
                        + head.right * (side * (0.9 + st * 2.6) + drift)
                        - head.up * (st * 1.3);

                    let col = Color::rgb(0, (200.0 + 55.0 * st) as u8, (220.0 + 35.0 * st) as u8);
                    r3d.draw_line_3d(gfx, prev_pt, cur_pt, col, view, proj, 0.005);
                    if s == 10 {
                        r3d.draw_point_3d(gfx, cur_pt, 0.32, Color::rgb(160, 255, 245), view, proj);
                    }
                    prev_pt = cur_pt;
                }
            }
        }

        // D. Render 3D Tail Flukes
        {
            let tail = &self.segments[NUM_3D_SEGMENTS - 1];
            let spread_angles = [-0.55f32, -0.28, 0.0, 0.28, 0.55];
            for &sa in &spread_angles {
                let fin_rot = Mat4::rotation_axis(tail.right, sa);
                let tail_fwd = fin_rot.transform_dir(-tail.forward);
                let fluke_tip = tail.pos + tail_fwd * 6.5;
                r3d.draw_line_3d(gfx, tail.pos, fluke_tip, Color::rgb(0, 195, 235), view, proj, 0.005);
                r3d.draw_point_3d(gfx, fluke_tip, 0.35, Color::rgb(140, 255, 240), view, proj);
            }
        }

        // E. Render 3D Marine Snow & Bubbles
        for b in &self.bubbles {
            r3d.draw_point_3d(gfx, b.pos, b.radius, b.color, view, proj);
        }

        for m in &self.snow {
            r3d.draw_point_3d(gfx, m.pos, m.radius, m.color, view, proj);
        }
    }

    /// Mode 2: 3D Geometry & Lighting Showcase
    fn render_geometry_showcase(
        &self,
        gfx: &mut Graphics,
        r3d: &mut Renderer3D,
        view: &Mat4,
        proj: &Mat4,
        time_sec: f32,
    ) {
        let rot_torus = Mat4::rotation_x(time_sec * 0.7)
            .mul(&Mat4::rotation_y(time_sec * 1.1));
        let torus_model = Mat4::translation(0.0, 2.0, 0.0).mul(&rot_torus);
        r3d.render_mesh(gfx, &self.torus_mesh, &torus_model, view, proj);

        let scale_sphere = 1.0 + 0.12 * (time_sec * 3.0).sin();
        let sphere_model = Mat4::translation(0.0, 2.0, 0.0)
            .mul(&Mat4::scale(scale_sphere, scale_sphere, scale_sphere));
        r3d.render_mesh(gfx, &self.sphere_mesh, &sphere_model, view, proj);

        for i in 0..4 {
            let angle = time_sec * 0.9 + (i as f32 * PI * 0.5);
            let dist = 24.0;
            let ox = angle.cos() * dist;
            let oz = angle.sin() * dist;
            let oy = 2.0 + (angle * 2.0).sin() * 4.0;

            let sat_rot = Mat4::rotation_y(time_sec * 2.0 + i as f32)
                .mul(&Mat4::rotation_x(time_sec * 1.5));
            let sat_model = Mat4::translation(ox, oy, oz).mul(&sat_rot);

            if i % 2 == 0 {
                r3d.render_mesh(gfx, &self.crystal_mesh, &sat_model, view, proj);
            } else {
                r3d.render_mesh(gfx, &self.cube_mesh, &sat_model, view, proj);
            }

            r3d.draw_line_3d(gfx, Vec3::new(ox, oy, oz), Vec3::new(0.0, 2.0, 0.0), Color::rgb(0, 140, 180), view, proj, 0.003);
        }

        for b in &self.bubbles {
            r3d.draw_point_3d(gfx, b.pos, b.radius, b.color, view, proj);
        }
    }

    /// Mode 3: 3D Holographic Matrix Grid
    fn render_hologram_grid(
        &self,
        gfx: &mut Graphics,
        r3d: &mut Renderer3D,
        view: &Mat4,
        proj: &Mat4,
        time_sec: f32,
    ) {
        let old_shading = r3d.shading_mode;
        r3d.shading_mode = ShadingMode::Wireframe;

        self.render_abyss_scene(gfx, r3d, view, proj, time_sec);

        let grid_size = 50.0;
        let step = 10.0;
        let y_floor = -18.0;
        let grid_col = Color::rgb(0, 95, 125);

        let mut x = -grid_size;
        while x <= grid_size {
            r3d.draw_line_3d(
                gfx,
                Vec3::new(x, y_floor, -grid_size),
                Vec3::new(x, y_floor, grid_size),
                grid_col,
                view,
                proj,
                0.0,
            );
            x += step;
        }

        let mut z = -grid_size;
        while z <= grid_size {
            r3d.draw_line_3d(
                gfx,
                Vec3::new(-grid_size, y_floor, z),
                Vec3::new(grid_size, y_floor, z),
                grid_col,
                view,
                proj,
                0.0,
            );
            z += step;
        }

        r3d.shading_mode = old_shading;
    }
}
