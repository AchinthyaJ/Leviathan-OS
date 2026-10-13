use crate::math3d::{Mat4, Vec3};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    CinematicOrbit,
    ChaseFollow,
    HeadOn,
    TrenchOverview,
}

impl CameraMode {
    pub fn name(&self) -> &'static str {
        match self {
            CameraMode::CinematicOrbit => "Cinematic Orbit (360 Pan)",
            CameraMode::ChaseFollow => "Chase Cam (Skeletal Lock)",
            CameraMode::HeadOn => "Head-On Intercept Cam",
            CameraMode::TrenchOverview => "Trench Overview (Wide Angle)",
        }
    }
}

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub mode: CameraMode,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            eye: Vec3::new(0.0, 10.0, -42.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::UP,
            fov_y: 58.0 * (PI / 180.0),
            near: 1.0,
            far: 300.0,
            mode: CameraMode::CinematicOrbit,
        }
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            CameraMode::CinematicOrbit => CameraMode::ChaseFollow,
            CameraMode::ChaseFollow => CameraMode::HeadOn,
            CameraMode::HeadOn => CameraMode::TrenchOverview,
            CameraMode::TrenchOverview => CameraMode::CinematicOrbit,
        };
    }

    pub fn update(&mut self, time_sec: f32, target_pos: Vec3, target_forward: Vec3) {
        match self.mode {
            CameraMode::CinematicOrbit => {
                let orbit_speed = 0.32;
                let angle = time_sec * orbit_speed;
                let radius = 42.0;
                let height = 8.0 + 4.0 * (time_sec * 0.25).sin();

                self.eye = Vec3::new(
                    angle.sin() * radius,
                    height,
                    angle.cos() * radius,
                );
                self.target = Vec3::lerp(self.target, target_pos * 0.6, 0.10);
                self.up = Vec3::UP;
            }
            CameraMode::ChaseFollow => {
                let chase_dist = 26.0;
                let chase_height = 6.5;
                let desired_eye = target_pos - target_forward * chase_dist + Vec3::new(0.0, chase_height, 0.0);
                let desired_target = target_pos + target_forward * 16.0;

                self.eye = Vec3::lerp(self.eye, desired_eye, 0.14);
                self.target = Vec3::lerp(self.target, desired_target, 0.18);
                self.up = Vec3::UP;
            }
            CameraMode::HeadOn => {
                let front_dist = 24.0;
                let desired_eye = target_pos + target_forward * front_dist + Vec3::new(0.0, 2.5, 0.0);
                let desired_target = target_pos;

                self.eye = Vec3::lerp(self.eye, desired_eye, 0.12);
                self.target = Vec3::lerp(self.target, desired_target, 0.18);
                self.up = Vec3::UP;
            }
            CameraMode::TrenchOverview => {
                self.eye = Vec3::new(0.0, 36.0, -65.0);
                self.target = Vec3::new(0.0, -3.0, 5.0);
                self.up = Vec3::UP;
            }
        }
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye, self.target, self.up)
    }

    pub fn get_projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(self.fov_y, aspect, self.near, self.far)
    }
}
