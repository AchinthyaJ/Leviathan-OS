use crate::color::Color;
use crate::math3d::{Mat4, Vec3};
use std::f32::consts::PI;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub normal: Vec3,
    pub color: Color,
}

impl Vertex {
    pub fn new(pos: Vec3, normal: Vec3, color: Color) -> Self {
        Self { pos, normal, color }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
    pub normal: Vec3,
    pub color: Color,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
    pub edges: Vec<[usize; 2]>,
}

#[allow(dead_code)]
impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            triangles: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_vertex(&mut self, pos: Vec3, normal: Vec3, color: Color) -> usize {
        let idx = self.vertices.len();
        self.vertices.push(Vertex::new(pos, normal, color));
        idx
    }

    pub fn add_triangle(&mut self, v0: usize, v1: usize, v2: usize, color: Color) {
        let p0 = self.vertices[v0].pos;
        let p1 = self.vertices[v1].pos;
        let p2 = self.vertices[v2].pos;
        let normal = (p1 - p0).cross(p2 - p0).normalize();
        self.triangles.push(Triangle {
            v0,
            v1,
            v2,
            normal,
            color,
        });

        // Add edges
        self.add_edge(v0, v1);
        self.add_edge(v1, v2);
        self.add_edge(v2, v0);
    }

    fn add_edge(&mut self, a: usize, b: usize) {
        let (min, max) = if a < b { (a, b) } else { (b, a) };
        if !self.edges.iter().any(|e| e[0] == min && e[1] == max) {
            self.edges.push([min, max]);
        }
    }

    pub fn recalculate_normals(&mut self) {
        for tri in &mut self.triangles {
            let p0 = self.vertices[tri.v0].pos;
            let p1 = self.vertices[tri.v1].pos;
            let p2 = self.vertices[tri.v2].pos;
            tri.normal = (p1 - p0).cross(p2 - p0).normalize();
        }

        let mut vert_normals = vec![Vec3::ZERO; self.vertices.len()];
        for tri in &self.triangles {
            vert_normals[tri.v0] += tri.normal;
            vert_normals[tri.v1] += tri.normal;
            vert_normals[tri.v2] += tri.normal;
        }

        for (i, v) in self.vertices.iter_mut().enumerate() {
            v.normal = vert_normals[i].normalize();
        }
    }

    pub fn transform(&mut self, mat: &Mat4) {
        for v in &mut self.vertices {
            v.pos = mat.transform_point(v.pos);
            v.normal = mat.transform_dir(v.normal);
        }
        for tri in &mut self.triangles {
            tri.normal = mat.transform_dir(tri.normal);
        }
    }

    pub fn merge(&mut self, other: &Mesh) {
        let offset = self.vertices.len();
        self.vertices.extend_from_slice(&other.vertices);
        for tri in &other.triangles {
            self.triangles.push(Triangle {
                v0: tri.v0 + offset,
                v1: tri.v1 + offset,
                v2: tri.v2 + offset,
                normal: tri.normal,
                color: tri.color,
            });
        }
        for e in &other.edges {
            self.edges.push([e[0] + offset, e[1] + offset]);
        }
    }

    // --- Procedural 3D Mesh Generators ---

    pub fn create_cube(size: f32, color: Color) -> Self {
        let mut mesh = Self::new();
        let h = size * 0.5;

        let corners = [
            Vec3::new(-h, -h, -h), // 0
            Vec3::new( h, -h, -h), // 1
            Vec3::new( h,  h, -h), // 2
            Vec3::new(-h,  h, -h), // 3
            Vec3::new(-h, -h,  h), // 4
            Vec3::new( h, -h,  h), // 5
            Vec3::new( h,  h,  h), // 6
            Vec3::new(-h,  h,  h), // 7
        ];

        for &c in &corners {
            mesh.add_vertex(c, c.normalize(), color);
        }

        // Front (Z+)
        mesh.add_triangle(4, 5, 6, color);
        mesh.add_triangle(4, 6, 7, color);
        // Back (Z-)
        mesh.add_triangle(1, 0, 3, color);
        mesh.add_triangle(1, 3, 2, color);
        // Left (X-)
        mesh.add_triangle(0, 4, 7, color);
        mesh.add_triangle(0, 7, 3, color);
        // Right (X+)
        mesh.add_triangle(5, 1, 2, color);
        mesh.add_triangle(5, 2, 6, color);
        // Top (Y+)
        mesh.add_triangle(7, 6, 2, color);
        mesh.add_triangle(7, 2, 3, color);
        // Bottom (Y-)
        mesh.add_triangle(0, 1, 5, color);
        mesh.add_triangle(0, 5, 4, color);

        mesh.recalculate_normals();
        mesh
    }

    pub fn create_cylinder(
        r_bottom: f32,
        r_top: f32,
        height: f32,
        segments: usize,
        color: Color,
    ) -> Self {
        let mut mesh = Self::new();
        let half_h = height * 0.5;

        // Bottom center & Top center
        let bot_center = mesh.add_vertex(Vec3::new(0.0, -half_h, 0.0), -Vec3::Y, color);
        let top_center = mesh.add_vertex(Vec3::new(0.0, half_h, 0.0), Vec3::Y, color);

        let mut bot_ring = Vec::new();
        let mut top_ring = Vec::new();

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * 2.0 * PI;
            let (s, c) = angle.sin_cos();

            let p_bot = Vec3::new(c * r_bottom, -half_h, s * r_bottom);
            let n_bot = Vec3::new(c, 0.0, s).normalize();
            bot_ring.push(mesh.add_vertex(p_bot, n_bot, color));

            let p_top = Vec3::new(c * r_top, half_h, s * r_top);
            let n_top = Vec3::new(c, 0.0, s).normalize();
            top_ring.push(mesh.add_vertex(p_top, n_top, color));
        }

        for i in 0..segments {
            let next = (i + 1) % segments;

            // Side quads (2 triangles)
            mesh.add_triangle(bot_ring[i], top_ring[i], top_ring[next], color);
            mesh.add_triangle(bot_ring[i], top_ring[next], bot_ring[next], color);

            // Bottom cap
            mesh.add_triangle(bot_center, bot_ring[next], bot_ring[i], color);
            // Top cap
            mesh.add_triangle(top_center, top_ring[i], top_ring[next], color);
        }

        mesh.recalculate_normals();
        mesh
    }

    pub fn create_sphere(radius: f32, stacks: usize, slices: usize, color: Color) -> Self {
        let mut mesh = Self::new();

        for i in 0..=stacks {
            let lat = (i as f32 / stacks as f32) * PI - (PI * 0.5);
            let (sin_lat, cos_lat) = lat.sin_cos();

            for j in 0..=slices {
                let lon = (j as f32 / slices as f32) * 2.0 * PI;
                let (sin_lon, cos_lon) = lon.sin_cos();

                let normal = Vec3::new(cos_lat * cos_lon, sin_lat, cos_lat * sin_lon);
                let pos = normal * radius;
                mesh.add_vertex(pos, normal, color);
            }
        }

        for i in 0..stacks {
            for j in 0..slices {
                let first = i * (slices + 1) + j;
                let second = first + slices + 1;

                mesh.add_triangle(first, second, first + 1, color);
                mesh.add_triangle(second, second + 1, first + 1, color);
            }
        }

        mesh.recalculate_normals();
        mesh
    }

    pub fn create_torus(
        r_major: f32,
        r_minor: f32,
        seg_major: usize,
        seg_minor: usize,
        color: Color,
    ) -> Self {
        let mut mesh = Self::new();

        for i in 0..seg_major {
            let u = (i as f32 / seg_major as f32) * 2.0 * PI;
            let (sin_u, cos_u) = u.sin_cos();
            let center = Vec3::new(cos_u * r_major, 0.0, sin_u * r_major);

            for j in 0..seg_minor {
                let v = (j as f32 / seg_minor as f32) * 2.0 * PI;
                let (sin_v, cos_v) = v.sin_cos();

                let normal = Vec3::new(cos_u * cos_v, sin_v, sin_u * cos_v);
                let pos = center + normal * r_minor;
                mesh.add_vertex(pos, normal, color);
            }
        }

        for i in 0..seg_major {
            let next_i = (i + 1) % seg_major;
            for j in 0..seg_minor {
                let next_j = (j + 1) % seg_minor;

                let v0 = i * seg_minor + j;
                let v1 = next_i * seg_minor + j;
                let v2 = next_i * seg_minor + next_j;
                let v3 = i * seg_minor + next_j;

                mesh.add_triangle(v0, v1, v2, color);
                mesh.add_triangle(v0, v2, v3, color);
            }
        }

        mesh.recalculate_normals();
        mesh
    }

    pub fn create_crystal(height: f32, radius: f32, color: Color) -> Self {
        let mut mesh = Self::new();
        let half_h = height * 0.5;
        let segs = 6;

        let top_tip = mesh.add_vertex(Vec3::new(0.0, half_h, 0.0), Vec3::Y, color);
        let bot_tip = mesh.add_vertex(Vec3::new(0.0, -half_h, 0.0), -Vec3::Y, color);

        let mut mid_upper = Vec::new();
        let mut mid_lower = Vec::new();

        for i in 0..segs {
            let a = (i as f32 / segs as f32) * 2.0 * PI;
            let (s, c) = a.sin_cos();

            let p_u = Vec3::new(c * radius, half_h * 0.3, s * radius);
            mid_upper.push(mesh.add_vertex(p_u, Vec3::new(c, 0.0, s).normalize(), color));

            let p_l = Vec3::new(c * radius, -half_h * 0.3, s * radius);
            mid_lower.push(mesh.add_vertex(p_l, Vec3::new(c, 0.0, s).normalize(), color));
        }

        for i in 0..segs {
            let next = (i + 1) % segs;

            // Top pyramid
            mesh.add_triangle(top_tip, mid_upper[i], mid_upper[next], color);

            // Middle prism
            mesh.add_triangle(mid_upper[i], mid_lower[i], mid_upper[next], color);
            mesh.add_triangle(mid_lower[i], mid_lower[next], mid_upper[next], color);

            // Bottom pyramid
            mesh.add_triangle(bot_tip, mid_lower[next], mid_lower[i], color);
        }

        mesh.recalculate_normals();
        mesh
    }

    /// Creates an armored, faceted 3D vertebra segment for the Leviathan
    pub fn create_vertebra(
        rx: f32,
        ry: f32,
        length: f32,
        dorsal_ridge: f32,
        color_dorsal: Color,
        color_ventral: Color,
    ) -> Self {
        let mut mesh = Self::new();
        let segments = 8;
        let half_len = length * 0.5;

        let mut front_ring = Vec::new();
        let mut back_ring = Vec::new();

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * 2.0 * PI;
            let (sin_a, cos_a) = angle.sin_cos();

            // Dorsal side is +Y
            let is_dorsal = sin_a > 0.0;
            let col = if is_dorsal { color_dorsal } else { color_ventral };

            let extra_y = if sin_a > 0.7 { dorsal_ridge * (sin_a - 0.7) / 0.3 } else { 0.0 };
            let vx = cos_a * rx;
            let vy = sin_a * ry + extra_y;

            let p_front = Vec3::new(vx, vy, half_len);
            let p_back = Vec3::new(vx * 0.94, vy * 0.94, -half_len);

            let norm = Vec3::new(cos_a, sin_a, 0.0).normalize();

            front_ring.push(mesh.add_vertex(p_front, norm, col));
            back_ring.push(mesh.add_vertex(p_back, norm, col));
        }

        for i in 0..segments {
            let next = (i + 1) % segments;
            let col = mesh.vertices[front_ring[i]].color;

            mesh.add_triangle(front_ring[i], back_ring[i], back_ring[next], col);
            mesh.add_triangle(front_ring[i], back_ring[next], front_ring[next], col);
        }

        mesh.recalculate_normals();
        mesh
    }

    /// Creates a 3D dragon skull with horned snout and articulated lower jaw
    pub fn create_dragon_head(scale: f32, primary: Color, rune_color: Color) -> Self {
        let mut mesh = Self::new();
        let s = scale;

        // Cranium / Brow
        let v_snout_tip = mesh.add_vertex(Vec3::new(0.0, -0.1 * s, 2.2 * s), Vec3::new(0.0, 0.0, 1.0), primary);
        let v_snout_top = mesh.add_vertex(Vec3::new(0.0, 0.4 * s, 1.4 * s), Vec3::new(0.0, 1.0, 0.3), primary);
        let v_snout_l   = mesh.add_vertex(Vec3::new(-0.45 * s, 0.0 * s, 1.2 * s), Vec3::new(-1.0, 0.0, 0.5), primary);
        let v_snout_r   = mesh.add_vertex(Vec3::new(0.45 * s, 0.0 * s, 1.2 * s), Vec3::new(1.0, 0.0, 0.5), primary);

        let v_brow_c    = mesh.add_vertex(Vec3::new(0.0, 0.8 * s, 0.3 * s), Vec3::new(0.0, 1.0, 0.0), rune_color);
        let v_brow_l    = mesh.add_vertex(Vec3::new(-0.85 * s, 0.6 * s, 0.4 * s), Vec3::new(-0.8, 0.6, 0.0), primary);
        let v_brow_r    = mesh.add_vertex(Vec3::new(0.85 * s, 0.6 * s, 0.4 * s), Vec3::new(0.8, 0.6, 0.0), primary);

        let v_eye_l     = mesh.add_vertex(Vec3::new(-0.75 * s, 0.4 * s, 0.6 * s), Vec3::new(-1.0, 0.2, 0.2), Color::rgb(255, 200, 0));
        let v_eye_r     = mesh.add_vertex(Vec3::new(0.75 * s, 0.4 * s, 0.6 * s), Vec3::new(1.0, 0.2, 0.2), Color::rgb(255, 200, 0));

        let v_skull_top = mesh.add_vertex(Vec3::new(0.0, 1.1 * s, -0.6 * s), Vec3::new(0.0, 1.0, -0.2), primary);
        let v_skull_l   = mesh.add_vertex(Vec3::new(-1.0 * s, 0.3 * s, -0.8 * s), Vec3::new(-1.0, 0.0, -0.3), primary);
        let v_skull_r   = mesh.add_vertex(Vec3::new(1.0 * s, 0.3 * s, -0.8 * s), Vec3::new(1.0, 0.0, -0.3), primary);
        let v_skull_back= mesh.add_vertex(Vec3::new(0.0, 0.2 * s, -1.2 * s), Vec3::new(0.0, 0.0, -1.0), primary);

        // Lower Jaw / Chin
        let v_chin_tip  = mesh.add_vertex(Vec3::new(0.0, -0.65 * s, 1.8 * s), Vec3::new(0.0, -1.0, 0.8), primary);
        let v_jaw_l     = mesh.add_vertex(Vec3::new(-0.55 * s, -0.4 * s, 0.5 * s), Vec3::new(-0.8, -0.6, 0.0), primary);
        let v_jaw_r     = mesh.add_vertex(Vec3::new(0.55 * s, -0.4 * s, 0.5 * s), Vec3::new(0.8, -0.6, 0.0), primary);
        let v_throat    = mesh.add_vertex(Vec3::new(0.0, -0.7 * s, -0.6 * s), Vec3::new(0.0, -1.0, 0.0), primary);

        // Horn Tips (sweeping backwards and outwards)
        let v_horn_l    = mesh.add_vertex(Vec3::new(-1.6 * s, 1.6 * s, -2.4 * s), Vec3::new(-0.5, 0.7, -0.5), rune_color);
        let v_horn_r    = mesh.add_vertex(Vec3::new(1.6 * s, 1.6 * s, -2.4 * s), Vec3::new(0.5, 0.7, -0.5), rune_color);

        // Snout facets
        mesh.add_triangle(v_snout_tip, v_snout_l, v_snout_top, primary);
        mesh.add_triangle(v_snout_tip, v_snout_top, v_snout_r, primary);
        mesh.add_triangle(v_snout_top, v_snout_l, v_brow_l, primary);
        mesh.add_triangle(v_snout_top, v_brow_r, v_snout_r, primary);
        mesh.add_triangle(v_snout_top, v_brow_l, v_brow_c, primary);
        mesh.add_triangle(v_snout_top, v_brow_c, v_brow_r, primary);

        // Brow and Skull top
        mesh.add_triangle(v_brow_c, v_brow_l, v_skull_top, primary);
        mesh.add_triangle(v_brow_c, v_skull_top, v_brow_r, primary);
        mesh.add_triangle(v_skull_top, v_brow_l, v_skull_l, primary);
        mesh.add_triangle(v_skull_top, v_skull_r, v_brow_r, primary);
        mesh.add_triangle(v_skull_top, v_skull_l, v_skull_back, primary);
        mesh.add_triangle(v_skull_top, v_skull_back, v_skull_r, primary);

        // Eyes
        mesh.add_triangle(v_eye_l, v_brow_l, v_snout_l, Color::rgb(255, 210, 30));
        mesh.add_triangle(v_eye_r, v_snout_r, v_brow_r, Color::rgb(255, 210, 30));

        // Horns
        mesh.add_triangle(v_skull_top, v_horn_l, v_skull_l, rune_color);
        mesh.add_triangle(v_skull_top, v_skull_r, v_horn_r, rune_color);

        // Lower Jaw
        mesh.add_triangle(v_chin_tip, v_jaw_l, v_throat, primary);
        mesh.add_triangle(v_chin_tip, v_throat, v_jaw_r, primary);
        mesh.add_triangle(v_chin_tip, v_snout_l, v_jaw_l, primary);
        mesh.add_triangle(v_chin_tip, v_jaw_r, v_snout_r, primary);

        mesh.recalculate_normals();
        mesh
    }

    /// Creates a 3D fin mesh that flutters with hydrodynamic wave motion
    pub fn create_fin(length: f32, width: f32, color: Color) -> Self {
        let mut mesh = Self::new();

        let v0 = mesh.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::Y, color);
        let v1 = mesh.add_vertex(Vec3::new(0.0, 0.0, length * 0.4), Vec3::Y, color);
        let v2 = mesh.add_vertex(Vec3::new(width * 0.7, 0.0, length * 0.8), Vec3::Y, color);
        let v3 = mesh.add_vertex(Vec3::new(width, 0.0, length), Vec3::Y, color);
        let v4 = mesh.add_vertex(Vec3::new(width * 0.4, 0.0, length * 0.5), Vec3::Y, color);

        mesh.add_triangle(v0, v1, v4, color);
        mesh.add_triangle(v1, v2, v4, color);
        mesh.add_triangle(v2, v3, v4, color);

        // Double-sided fin
        mesh.add_triangle(v0, v4, v1, color);
        mesh.add_triangle(v1, v4, v2, color);
        mesh.add_triangle(v2, v4, v3, color);

        mesh.recalculate_normals();
        mesh
    }
}
