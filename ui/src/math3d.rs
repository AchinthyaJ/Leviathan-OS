use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const X: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
    pub const UP: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const FORWARD: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn dot(self, rhs: Vec3) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline(always)]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline(always)]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline(always)]
    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len > 1e-6 {
            self * (1.0 / len)
        } else {
            Vec3::ZERO
        }
    }

    #[inline(always)]
    pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
        a + (b - a) * t
    }

    #[inline(always)]
    pub fn distance(self, other: Vec3) -> f32 {
        (self - other).length()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Vec3) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Vec3) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline(always)]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self * rhs.x, self * rhs.y, self * rhs.z)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn div(self, rhs: f32) -> Vec3 {
        let inv = 1.0 / rhs;
        Vec3::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[allow(dead_code)]
impl Vec4 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline(always)]
    pub fn from_vec3(v: Vec3, w: f32) -> Self {
        Self { x: v.x, y: v.y, z: v.z, w }
    }

    #[inline(always)]
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

/// 4x4 Matrix in Column-Major Order (index = col * 4 + row)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [f32; 16],
}

#[allow(dead_code)]
impl Mat4 {
    #[inline(always)]
    pub const fn identity() -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, // col 0
                0.0, 1.0, 0.0, 0.0, // col 1
                0.0, 0.0, 1.0, 0.0, // col 2
                0.0, 0.0, 0.0, 1.0, // col 3
            ],
        }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { m: [0.0; 16] }
    }

    #[inline(always)]
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                x,   y,   z,   1.0,
            ],
        }
    }

    #[inline(always)]
    pub fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            m: [
                sx,  0.0, 0.0, 0.0,
                0.0, sy,  0.0, 0.0,
                0.0, 0.0, sz,  0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    #[inline(always)]
    pub fn rotation_x(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0,
                0.0, c,   s,   0.0,
                0.0, -s,  c,   0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    #[inline(always)]
    pub fn rotation_y(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                c,   0.0, -s,  0.0,
                0.0, 1.0, 0.0, 0.0,
                s,   0.0, c,   0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    #[inline(always)]
    pub fn rotation_z(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                c,   s,   0.0, 0.0,
                -s,  c,   0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let rz = Self::rotation_z(roll);
        let rx = Self::rotation_x(pitch);
        let ry = Self::rotation_y(yaw);
        ry.mul(&rx).mul(&rz)
    }

    pub fn rotation_axis(axis: Vec3, rad: f32) -> Self {
        let a = axis.normalize();
        let (s, c) = rad.sin_cos();
        let oc = 1.0 - c;

        Self {
            m: [
                oc * a.x * a.x + c,       oc * a.x * a.y + a.z * s, oc * a.z * a.x - a.y * s, 0.0,
                oc * a.x * a.y - a.z * s, oc * a.y * a.y + c,       oc * a.y * a.z + a.x * s, 0.0,
                oc * a.z * a.x + a.y * s, oc * a.y * a.z - a.x * s, oc * a.z * a.z + c,       0.0,
                0.0,                      0.0,                      0.0,                      1.0,
            ],
        }
    }

    #[inline(always)]
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut out = [0.0f32; 16];
        for c in 0..4 {
            for r in 0..4 {
                out[c * 4 + r] = self.m[0 * 4 + r] * rhs.m[c * 4 + 0]
                               + self.m[1 * 4 + r] * rhs.m[c * 4 + 1]
                               + self.m[2 * 4 + r] * rhs.m[c * 4 + 2]
                               + self.m[3 * 4 + r] * rhs.m[c * 4 + 3];
            }
        }
        Mat4 { m: out }
    }

    #[inline(always)]
    pub fn transform_vec4(&self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.m[0] * v.x + self.m[4] * v.y + self.m[8]  * v.z + self.m[12] * v.w,
            self.m[1] * v.x + self.m[5] * v.y + self.m[9]  * v.z + self.m[13] * v.w,
            self.m[2] * v.x + self.m[6] * v.y + self.m[10] * v.z + self.m[14] * v.w,
            self.m[3] * v.x + self.m[7] * v.y + self.m[11] * v.z + self.m[15] * v.w,
        )
    }

    #[inline(always)]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let v = self.transform_vec4(Vec4::new(p.x, p.y, p.z, 1.0));
        if v.w.abs() > 1e-6 {
            let inv_w = 1.0 / v.w;
            Vec3::new(v.x * inv_w, v.y * inv_w, v.z * inv_w)
        } else {
            Vec3::new(v.x, v.y, v.z)
        }
    }

    #[inline(always)]
    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0] * d.x + self.m[4] * d.y + self.m[8]  * d.z,
            self.m[1] * d.x + self.m[5] * d.y + self.m[9]  * d.z,
            self.m[2] * d.x + self.m[6] * d.y + self.m[10] * d.z,
        ).normalize()
    }

    /// LookAt matrix with camera pointing down +Z in view space
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalize();
        let right = up.cross(forward).normalize();
        let true_up = forward.cross(right);

        Self {
            m: [
                right.x,         true_up.x,       forward.x,       0.0,
                right.y,         true_up.y,       forward.y,       0.0,
                right.z,         true_up.z,       forward.z,       0.0,
                -right.dot(eye), -true_up.dot(eye), -forward.dot(eye), 1.0,
            ],
        }
    }

    /// Perspective projection matrix for +Z view direction
    pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_half = (fov_y_rad * 0.5).tan();
        let f_y = 1.0 / tan_half;
        let f_x = f_y / aspect;
        let d = far - near;

        Self {
            m: [
                f_x, 0.0, 0.0, 0.0,
                0.0, f_y, 0.0, 0.0,
                0.0, 0.0, far / d, 1.0,
                0.0, 0.0, -(far * near) / d, 0.0,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_ops() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(a.dot(b), 4.0 + 10.0 + 18.0);

        let cross = Vec3::X.cross(Vec3::Y);
        assert!((cross - Vec3::Z).length() < 1e-6);

        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mat4_transform() {
        let t = Mat4::translation(10.0, -5.0, 20.0);
        let p = Vec3::new(1.0, 2.0, 3.0);
        let transformed = t.transform_point(p);
        assert_eq!(transformed, Vec3::new(11.0, -3.0, 23.0));

        let s = Mat4::scale(2.0, 3.0, 4.0);
        assert_eq!(s.transform_point(p), Vec3::new(2.0, 6.0, 12.0));
    }

    #[test]
    fn test_perspective_bounds() {
        let proj = Mat4::perspective(std::f32::consts::PI / 3.0, 16.0 / 9.0, 1.0, 100.0);
        let p_near = Vec4::new(0.0, 0.0, 1.0, 1.0);
        let clip_near = proj.transform_vec4(p_near);
        assert!((clip_near.z / clip_near.w).abs() < 1e-4);

        let p_far = Vec4::new(0.0, 0.0, 100.0, 1.0);
        let clip_far = proj.transform_vec4(p_far);
        assert!(((clip_far.z / clip_far.w) - 1.0).abs() < 1e-4);
    }
}
