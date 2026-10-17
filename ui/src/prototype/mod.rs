pub mod abyss_scene;
pub mod boot_screen;
pub mod login_config;
pub mod login_screen;

use std::fs::File;
use std::path::Path;

use crate::graphics::Graphics;
use abyss_scene::AbyssScene;
use boot_screen::{BootScreen, BootStage};
use login_screen::{FocusedElement, LoginScreen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperienceMode {
    BootOnly,
    LoginOnly,
    CompleteFlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperienceState {
    BootSequence,
    Transitioning,
    LoginSession,
}

pub struct LeviathanExperience {
    pub scene: AbyssScene,
    pub boot: BootScreen,
    pub login: LoginScreen,
    pub mode: ExperienceMode,
    pub state: ExperienceState,
    pub transition_t: f32,
    pub reduced_motion: bool,
    pub should_exit: bool,
}

impl LeviathanExperience {
    pub fn new(width: u32, height: u32, mode: ExperienceMode) -> Self {
        let scene = AbyssScene::new(width, height);
        let mut boot = BootScreen::new();
        let login = LoginScreen::new();

        let state = match mode {
            ExperienceMode::BootOnly => ExperienceState::BootSequence,
            ExperienceMode::LoginOnly => {
                boot.skip();
                ExperienceState::LoginSession
            }
            ExperienceMode::CompleteFlow => ExperienceState::BootSequence,
        };

        Self {
            scene,
            boot,
            login,
            mode,
            state,
            transition_t: 0.0,
            reduced_motion: false,
            should_exit: false,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.scene.resize(width, height);
    }

    pub fn set_reduced_motion(&mut self, enabled: bool) {
        self.reduced_motion = enabled;
        self.scene.reduced_motion = enabled;
        self.boot.reduced_motion = enabled;
        self.login.reduced_motion = enabled;
    }

    pub fn replay_boot(&mut self) {
        self.boot.reset();
        self.state = ExperienceState::BootSequence;
        self.transition_t = 0.0;
        self.login.requested_replay = false;
        self.login.dropdown_open = false;
    }

    pub fn skip_to_login(&mut self) {
        self.boot.skip();
        self.state = ExperienceState::LoginSession;
        self.transition_t = 1.0;
    }

    pub fn update(&mut self, dt: f32) {
        self.scene.update(dt);

        if self.login.requested_replay {
            self.replay_boot();
            return;
        }

        if self.login.requested_exit {
            self.should_exit = true;
            return;
        }

        match self.state {
            ExperienceState::BootSequence => {
                self.boot.update(dt);
                if self.mode == ExperienceMode::BootOnly {
                    if self.boot.is_completed() {
                        self.boot.auto_loop = true;
                    }
                } else if self.boot.stage == BootStage::Transition {
                    self.state = ExperienceState::Transitioning;
                    self.transition_t = self.boot.login_blend;
                } else if self.boot.is_completed() {
                    self.state = ExperienceState::LoginSession;
                    self.transition_t = 1.0;
                }
            }
            ExperienceState::Transitioning => {
                self.boot.update(dt);
                self.login.update(dt);
                self.transition_t = self.boot.login_blend;
                if self.boot.is_completed() {
                    self.state = ExperienceState::LoginSession;
                    self.transition_t = 1.0;
                }
            }
            ExperienceState::LoginSession => {
                self.login.update(dt);
            }
        }
    }

    pub fn render(&self, gfx: &mut Graphics) {
        match self.state {
            ExperienceState::BootSequence => {
                self.boot.render(&self.scene, gfx);
            }
            ExperienceState::Transitioning => {
                // Smooth blended transition between Boot and Login
                let blend = self.transition_t.clamp(0.0, 1.0);
                self.boot.render(&self.scene, gfx);
                self.login.render(&self.scene, gfx, blend);
            }
            ExperienceState::LoginSession => {
                self.login.render(&self.scene, gfx, 1.0);
            }
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        match self.state {
            ExperienceState::BootSequence | ExperienceState::Transitioning => {
                if ch == ' ' || ch == '\n' || ch == '\r' {
                    self.skip_to_login();
                } else if ch == 'r' || ch == 'R' {
                    self.replay_boot();
                } else if ch == 'm' || ch == 'M' {
                    let next = !self.reduced_motion;
                    self.set_reduced_motion(next);
                } else if ch == 'q' || ch == 'Q' {
                    self.should_exit = true;
                }
            }
            ExperienceState::LoginSession => {
                if ch == 'r' || ch == 'R' {
                    // Only replay if password field is empty, otherwise type 'r'
                    if self.login.password_input.is_empty() && !self.login.dropdown_open {
                        self.replay_boot();
                        return;
                    }
                }
                self.login.handle_char(ch);
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_backspace();
        }
    }

    pub fn handle_enter(&mut self) {
        match self.state {
            ExperienceState::BootSequence | ExperienceState::Transitioning => {
                self.skip_to_login();
            }
            ExperienceState::LoginSession => {
                self.login.handle_enter();
            }
        }
    }

    pub fn handle_tab(&mut self) {
        match self.state {
            ExperienceState::BootSequence | ExperienceState::Transitioning => {
                self.skip_to_login();
            }
            ExperienceState::LoginSession => {
                self.login.handle_tab();
            }
        }
    }

    pub fn handle_up(&mut self) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_up();
        }
    }

    pub fn handle_down(&mut self) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_down();
        }
    }

    pub fn handle_left(&mut self) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_left();
        }
    }

    pub fn handle_right(&mut self) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_right();
        }
    }

    pub fn handle_mouse_move(&mut self, mouse_x: i32, mouse_y: i32, mouse_down: bool) {
        if self.state == ExperienceState::LoginSession {
            self.login.handle_mouse_move(mouse_x, mouse_y, mouse_down);
        }
    }

    pub fn handle_mouse_click(&mut self, mouse_x: i32, mouse_y: i32) {
        match self.state {
            ExperienceState::BootSequence | ExperienceState::Transitioning => {
                self.skip_to_login();
            }
            ExperienceState::LoginSession => {
                self.login.handle_click(mouse_x, mouse_y, self.scene.width, self.scene.height);
            }
        }
    }

    pub fn handle_escape(&mut self) {
        if self.state == ExperienceState::LoginSession {
            if self.login.dropdown_open {
                self.login.dropdown_open = false;
            } else if self.login.env_dropdown_open {
                self.login.env_dropdown_open = false;
            } else {
                self.login.password_input.clear();
                self.login.focus = FocusedElement::PasswordField;
            }
        } else {
            self.should_exit = true;
        }
    }
}

/// Helper function to save a Graphics backbuffer (BGRA) directly to PNG on disk
pub fn save_graphics_to_png<P: AsRef<Path>>(gfx: &Graphics, path: P) -> Result<(), String> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = File::create(p).map_err(|e| format!("Failed to create {}: {}", p.display(), e))?;
    let ref mut w = std::io::BufWriter::new(file);

    let width = gfx.width();
    let height = gfx.height();
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| format!("PNG header error: {}", e))?;

    let buffer = gfx.get_backbuffer();
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for (src_chunk, dst_chunk) in buffer.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        dst_chunk[0] = src_chunk[2]; // R
        dst_chunk[1] = src_chunk[1]; // G
        dst_chunk[2] = src_chunk[0]; // B
        dst_chunk[3] = 255;          // Framebuffer is fully opaque on screen
    }

    writer.write_image_data(&rgba).map_err(|e| format!("PNG write error: {}", e))?;
    Ok(())
}
