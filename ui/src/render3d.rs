use crate::color::Color;
use crate::graphics::Graphics;
use crate::math3d::{Mat4, Vec3, Vec4};
use crate::mesh::Mesh;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingMode {
    Solid,
    Wireframe,
    Hybrid,
}

impl ShadingMode {
    pub fn name(&self) -> &'static str {
        match self {
            ShadingMode::Solid => "Solid Shaded (Lambertian Diffuse + Deep Fog)",
            ShadingMode::Wireframe => "Bioluminescent Holographic Wireframe",
            ShadingMode::Hybrid => "Hybrid (Armored Scutes + Glowing Lattice)",
        }
    }
}

pub struct Renderer3D {
    pub z_buffer: Vec<f32>, // Stores 1.0 / depth (0.0 = infinitely far, larger = closer)
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bytes_per_pixel: u32,
    pub light_dir: Vec3,
    pub ambient_color: Color,
    pub fog_color: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub shading_mode: ShadingMode,
}

#[derive(Clone, Copy)]
struct ClippedVertex {
    view_pos: Vec3,
    world_normal: Vec3,
    color: Color,
}

impl Renderer3D {
    pub fn new(gfx: &Graphics) -> Self {
        let width = gfx.width();
        let height = gfx.height();
        let z_buffer = vec![0.0f32; (width * height) as usize];

        Self {
            z_buffer,
            width,
            height,
            pitch: gfx.pitch(),
            bytes_per_pixel: gfx.bytes_per_pixel(),
            light_dir: Vec3::new(0.35, 0.85, -0.40).normalize(),
            ambient_color: Color::rgb(6, 22, 34),
            fog_color: Color::rgb(2, 10, 18),
            fog_start: 25.0,
            fog_end: 120.0,
            shading_mode: ShadingMode::Solid,
        }
    }

    #[inline(always)]
    pub fn clear_depth(&mut self) {
        self.z_buffer.fill(0.0f32);
    }

    pub fn cycle_shading_mode(&mut self) {
        self.shading_mode = match self.shading_mode {
            ShadingMode::Solid => ShadingMode::Wireframe,
            ShadingMode::Wireframe => ShadingMode::Hybrid,
            ShadingMode::Hybrid => ShadingMode::Solid,
        };
    }

    /// Renders a complete 3D Mesh with lighting, depth testing, and distance fog
    pub fn render_mesh(
        &mut self,
        gfx: &mut Graphics,
        mesh: &Mesh,
        model: &Mat4,
        view: &Mat4,
        proj: &Mat4,
    ) {
        let model_view = view.mul(model);

        // Pre-transform all vertices to view space & world normals
        let mut view_verts = Vec::with_capacity(mesh.vertices.len());
        let mut world_norms = Vec::with_capacity(mesh.vertices.len());

        for v in &mesh.vertices {
            view_verts.push(model_view.transform_point(v.pos));
            world_norms.push(model.transform_dir(v.normal));
        }

        let near = 0.6f32;

        if self.shading_mode == ShadingMode::Solid || self.shading_mode == ShadingMode::Hybrid {
            for tri in &mesh.triangles {
                let v0 = ClippedVertex {
                    view_pos: view_verts[tri.v0],
                    world_normal: world_norms[tri.v0],
                    color: tri.color,
                };
                let v1 = ClippedVertex {
                    view_pos: view_verts[tri.v1],
                    world_normal: world_norms[tri.v1],
                    color: tri.color,
                };
                let v2 = ClippedVertex {
                    view_pos: view_verts[tri.v2],
                    world_normal: world_norms[tri.v2],
                    color: tri.color,
                };

                self.clip_and_rasterize_triangle(gfx, v0, v1, v2, proj, near);
            }
        }

        if self.shading_mode == ShadingMode::Wireframe || self.shading_mode == ShadingMode::Hybrid {
            let edge_col = if self.shading_mode == ShadingMode::Wireframe {
                Color::rgb(0, 240, 230)
            } else {
                Color::rgb(0, 200, 190)
            };

            for edge in &mesh.edges {
                let p0 = mesh.vertices[edge[0]].pos;
                let p1 = mesh.vertices[edge[1]].pos;
                let world_p0 = model.transform_point(p0);
                let world_p1 = model.transform_point(p1);
                self.draw_line_3d(gfx, world_p0, world_p1, edge_col, view, proj, 0.003);
            }
        }
    }

    /// Clips triangle against view-space near plane and rasterizes resulting polygon
    fn clip_and_rasterize_triangle(
        &mut self,
        gfx: &mut Graphics,
        v0: ClippedVertex,
        v1: ClippedVertex,
        v2: ClippedVertex,
        proj: &Mat4,
        near: f32,
    ) {
        let mut inside = [v0; 3];
        let mut outside = [v0; 3];
        let mut n_in = 0;
        let mut n_out = 0;

        let verts = [v0, v1, v2];
        for &v in &verts {
            if v.view_pos.z >= near {
                inside[n_in] = v;
                n_in += 1;
            } else {
                outside[n_out] = v;
                n_out += 1;
            }
        }

        match n_in {
            0 => {}
            3 => {
                self.rasterize_projected_triangle(gfx, v0, v1, v2, proj);
            }
            1 => {
                let in_v = inside[0];
                let out0 = outside[0];
                let out1 = outside[1];

                let t0 = (near - in_v.view_pos.z) / (out0.view_pos.z - in_v.view_pos.z);
                let t1 = (near - in_v.view_pos.z) / (out1.view_pos.z - in_v.view_pos.z);

                let p0 = ClippedVertex {
                    view_pos: Vec3::lerp(in_v.view_pos, out0.view_pos, t0),
                    world_normal: Vec3::lerp(in_v.world_normal, out0.world_normal, t0).normalize(),
                    color: in_v.color,
                };
                let p1 = ClippedVertex {
                    view_pos: Vec3::lerp(in_v.view_pos, out1.view_pos, t1),
                    world_normal: Vec3::lerp(in_v.world_normal, out1.world_normal, t1).normalize(),
                    color: in_v.color,
                };

                self.rasterize_projected_triangle(gfx, in_v, p0, p1, proj);
            }
            2 => {
                let in0 = inside[0];
                let in1 = inside[1];
                let out_v = outside[0];

                let t0 = (near - in0.view_pos.z) / (out_v.view_pos.z - in0.view_pos.z);
                let t1 = (near - in1.view_pos.z) / (out_v.view_pos.z - in1.view_pos.z);

                let p0 = ClippedVertex {
                    view_pos: Vec3::lerp(in0.view_pos, out_v.view_pos, t0),
                    world_normal: Vec3::lerp(in0.world_normal, out_v.world_normal, t0).normalize(),
                    color: in0.color,
                };
                let p1 = ClippedVertex {
                    view_pos: Vec3::lerp(in1.view_pos, out_v.view_pos, t1),
                    world_normal: Vec3::lerp(in1.world_normal, out_v.world_normal, t1).normalize(),
                    color: in1.color,
                };

                self.rasterize_projected_triangle(gfx, in0, in1, p0, proj);
                self.rasterize_projected_triangle(gfx, in1, p1, p0, proj);
            }
            _ => {}
        }
    }

    /// Blazing-fast 1/z incremental rasterizer with zero per-pixel divisions
    fn rasterize_projected_triangle(
        &mut self,
        gfx: &mut Graphics,
        v0: ClippedVertex,
        v1: ClippedVertex,
        v2: ClippedVertex,
        proj: &Mat4,
    ) {
        let p0_clip = proj.transform_vec4(Vec4::from_vec3(v0.view_pos, 1.0));
        let p1_clip = proj.transform_vec4(Vec4::from_vec3(v1.view_pos, 1.0));
        let p2_clip = proj.transform_vec4(Vec4::from_vec3(v2.view_pos, 1.0));

        let inv_w0 = 1.0 / p0_clip.w;
        let inv_w1 = 1.0 / p1_clip.w;
        let inv_w2 = 1.0 / p2_clip.w;

        let width_f = self.width as f32;
        let height_f = self.height as f32;

        let s0_x = (p0_clip.x * inv_w0 * 0.5 + 0.5) * width_f;
        let s0_y = (1.0 - (p0_clip.y * inv_w0 * 0.5 + 0.5)) * height_f;

        let s1_x = (p1_clip.x * inv_w1 * 0.5 + 0.5) * width_f;
        let s1_y = (1.0 - (p1_clip.y * inv_w1 * 0.5 + 0.5)) * height_f;

        let s2_x = (p2_clip.x * inv_w2 * 0.5 + 0.5) * width_f;
        let s2_y = (1.0 - (p2_clip.y * inv_w2 * 0.5 + 0.5)) * height_f;

        // 2D Cross product for screen orientation (Backface Culling)
        let cross_2d = (s1_x - s0_x) * (s2_y - s0_y) - (s1_y - s0_y) * (s2_x - s0_x);
        if cross_2d <= 0.001 {
            return;
        }

        // Bounding Box
        let min_x = s0_x.min(s1_x).min(s2_x).floor() as i32;
        let max_x = s0_x.max(s1_x).max(s2_x).ceil() as i32;
        let min_y = s0_y.min(s1_y).min(s2_y).floor() as i32;
        let max_y = s0_y.max(s1_y).max(s2_y).ceil() as i32;

        let x0 = min_x.clamp(0, self.width as i32 - 1);
        let x1 = max_x.clamp(0, self.width as i32 - 1);
        let y0 = min_y.clamp(0, self.height as i32 - 1);
        let y1 = max_y.clamp(0, self.height as i32 - 1);

        if x0 > x1 || y0 > y1 {
            return;
        }

        // Lighting & Shading
        let tri_norm = (v0.world_normal + v1.world_normal + v2.world_normal).normalize();
        let diffuse = self.light_dir.dot(tri_norm).max(0.0);
        let lit_color = v0.color.scale(diffuse * 0.72 + 0.28).add_clamped(self.ambient_color);

        let z0 = v0.view_pos.z;
        let z1 = v1.view_pos.z;
        let z2 = v2.view_pos.z;
        let avg_z = (z0 + z1 + z2) / 3.0;

        let fog_factor = ((avg_z - self.fog_start) / (self.fog_end - self.fog_start)).clamp(0.0, 1.0);
        let shaded_color = Color::blend(lit_color, self.fog_color, fog_factor * 0.88);
        let packed_color = gfx.pack_color(shaded_color);

        let inv_cross = 1.0 / cross_2d;
        let inv_z0 = 1.0 / z0;
        let inv_z1 = 1.0 / z1;
        let inv_z2 = 1.0 / z2;

        // Linear edge function constants: E(x, y) = A*x + B*y + C
        let a0 = s1_y - s2_y;
        let b0 = s2_x - s1_x;
        let c0 = s1_x * s2_y - s1_y * s2_x;

        let a1 = s2_y - s0_y;
        let b1 = s0_x - s2_x;
        let c1 = s2_x * s0_y - s2_y * s0_x;

        let dw0_dx = a0 * inv_cross;
        let dw0_dy = b0 * inv_cross;
        let dw1_dx = a1 * inv_cross;
        let dw1_dy = b1 * inv_cross;

        let dz0 = inv_z0 - inv_z2;
        let dz1 = inv_z1 - inv_z2;
        let d_inv_z_dx = (dw0_dx * dz0) + (dw1_dx * dz1);
        let d_inv_z_dy = (dw0_dy * dz0) + (dw1_dy * dz1);

        let pitch = self.pitch as usize;
        let bpp = self.bytes_per_pixel as usize;
        let backbuf = gfx.backbuffer_mut();
        let backbuf_ptr = backbuf.as_mut_ptr();
        let z_ptr = self.z_buffer.as_mut_ptr();
        let width_usize = self.width as usize;

        let fx0 = x0 as f32 + 0.5;
        let fy0 = y0 as f32 + 0.5;
        let mut w0_row = (a0 * fx0 + b0 * fy0 + c0) * inv_cross;
        let mut w1_row = (a1 * fx0 + b1 * fy0 + c1) * inv_cross;
        let mut inv_z_row = inv_z2 + w0_row * dz0 + w1_row * dz1;

        for py in y0..=y1 {
            let row_offset = (py as usize) * pitch;
            let z_row_offset = (py as usize) * width_usize;

            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut inv_z = inv_z_row;

            for px in x0..=x1 {
                if w0 >= 0.0 && w1 >= 0.0 && (w0 + w1) <= 1.0 {
                    let z_idx = z_row_offset + (px as usize);
                    unsafe {
                        let cur_z = *z_ptr.add(z_idx);
                        if inv_z > cur_z {
                            *z_ptr.add(z_idx) = inv_z;

                            let pixel_offset = row_offset + (px as usize) * bpp;
                            *(backbuf_ptr.add(pixel_offset) as *mut u32) = packed_color;
                        }
                    }
                }

                w0 += dw0_dx;
                w1 += dw1_dx;
                inv_z += d_inv_z_dx;
            }

            w0_row += dw0_dy;
            w1_row += dw1_dy;
            inv_z_row += d_inv_z_dy;
        }
    }

    /// Draws a 3D line in world coordinates with depth testing
    pub fn draw_line_3d(
        &mut self,
        gfx: &mut Graphics,
        p0: Vec3,
        p1: Vec3,
        color: Color,
        view: &Mat4,
        proj: &Mat4,
        inv_z_bias: f32,
    ) {
        let v0_view = view.transform_point(p0);
        let v1_view = view.transform_point(p1);

        let near = 0.5f32;
        if v0_view.z < near && v1_view.z < near {
            return;
        }

        let (v0_c, v1_c) = if v0_view.z < near {
            let t = (near - v0_view.z) / (v1_view.z - v0_view.z);
            (Vec3::lerp(v0_view, v1_view, t), v1_view)
        } else if v1_view.z < near {
            let t = (near - v1_view.z) / (v0_view.z - v1_view.z);
            (v0_view, Vec3::lerp(v1_view, v0_view, t))
        } else {
            (v0_view, v1_view)
        };

        let clip0 = proj.transform_vec4(Vec4::from_vec3(v0_c, 1.0));
        let clip1 = proj.transform_vec4(Vec4::from_vec3(v1_c, 1.0));

        let width_f = self.width as f32;
        let height_f = self.height as f32;

        let s0_x = ((clip0.x / clip0.w) * 0.5 + 0.5) * width_f;
        let s0_y = (1.0 - ((clip0.y / clip0.w) * 0.5 + 0.5)) * height_f;

        let s1_x = ((clip1.x / clip1.w) * 0.5 + 0.5) * width_f;
        let s1_y = (1.0 - ((clip1.y / clip1.w) * 0.5 + 0.5)) * height_f;

        let x0 = s0_x as i32;
        let y0 = s0_y as i32;
        let x1 = s1_x as i32;
        let y1 = s1_y as i32;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let dist_screen = ((x1 - x0) as f32).hypot((y1 - y0) as f32).max(1.0);
        let mut cur_x = x0;
        let mut cur_y = y0;

        let inv_z0 = 1.0 / v0_c.z;
        let inv_z1 = 1.0 / v1_c.z;

        let packed_color = gfx.pack_color(color);
        let pitch = self.pitch as usize;
        let bpp = self.bytes_per_pixel as usize;
        let backbuf = gfx.backbuffer_mut();

        loop {
            if cur_x >= 0 && (cur_x as u32) < self.width && cur_y >= 0 && (cur_y as u32) < self.height {
                let traveled = ((cur_x - x0) as f32).hypot((cur_y - y0) as f32) / dist_screen;
                let inv_z = (inv_z0 + (inv_z1 - inv_z0) * traveled) + inv_z_bias;

                let z_idx = (cur_y as usize) * (self.width as usize) + (cur_x as usize);
                if inv_z > self.z_buffer[z_idx] {
                    self.z_buffer[z_idx] = inv_z;
                    let offset = (cur_y as usize) * pitch + (cur_x as usize) * bpp;
                    if offset + 4 <= backbuf.len() {
                        backbuf[offset..offset + 4].copy_from_slice(&packed_color.to_ne_bytes());
                    }
                }
            }

            if cur_x == x1 && cur_y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cur_x += sx;
            }
            if e2 <= dx {
                err += dx;
                cur_y += sy;
            }
        }
    }

    /// Draws a 3D billboard point / bubble with depth testing and perspective scaling
    pub fn draw_point_3d(
        &mut self,
        gfx: &mut Graphics,
        p: Vec3,
        world_radius: f32,
        color: Color,
        view: &Mat4,
        proj: &Mat4,
    ) {
        let p_view = view.transform_point(p);
        if p_view.z < 0.8 {
            return;
        }

        let clip = proj.transform_vec4(Vec4::from_vec3(p_view, 1.0));
        let width_f = self.width as f32;
        let height_f = self.height as f32;

        let sx = ((clip.x / clip.w) * 0.5 + 0.5) * width_f;
        let sy = (1.0 - ((clip.y / clip.w) * 0.5 + 0.5)) * height_f;

        let screen_r = ((world_radius / p_view.z) * height_f * 0.8).clamp(1.0, 16.0) as i32;
        let cx = sx as i32;
        let cy = sy as i32;
        let inv_z = 1.0 / p_view.z;

        let fog_factor = ((p_view.z - self.fog_start) / (self.fog_end - self.fog_start)).clamp(0.0, 1.0);
        let shaded_color = Color::blend(color, self.fog_color, fog_factor * 0.9);
        let packed_color = gfx.pack_color(shaded_color);

        let pitch = self.pitch as usize;
        let bpp = self.bytes_per_pixel as usize;
        let backbuf = gfx.backbuffer_mut();

        let r2 = screen_r * screen_r;
        for dy in -screen_r..=screen_r {
            let py = cy + dy;
            if py < 0 || (py as u32) >= self.height {
                continue;
            }
            let row_offset = (py as usize) * pitch;
            let z_row_offset = (py as usize) * (self.width as usize);

            for dx in -screen_r..=screen_r {
                if dx * dx + dy * dy <= r2 {
                    let px = cx + dx;
                    if px < 0 || (px as u32) >= self.width {
                        continue;
                    }

                    let z_idx = z_row_offset + (px as usize);
                    if inv_z > self.z_buffer[z_idx] {
                        self.z_buffer[z_idx] = inv_z;
                        let offset = row_offset + (px as usize) * bpp;
                        if offset + 4 <= backbuf.len() {
                            backbuf[offset..offset + 4].copy_from_slice(&packed_color.to_ne_bytes());
                        }
                    }
                }
            }
        }
    }
}
