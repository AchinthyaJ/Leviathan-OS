use crate::color::Color;
use crate::graphics::Graphics;
use crate::image::Image;
use crate::mouse::MouseState;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct PrizeInfo {
    pub prize_num: u8,
    pub category: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub lines: &'static [&'static str],
    pub joke_box: Option<&'static [&'static str]>,
    pub accent_color: Color,
}

pub const PRIZES: [PrizeInfo; 5] = [
    // Prize 5 — Best Sister of the Year (Rightmost trophy, Scene 1)
    PrizeInfo {
        prize_num: 5,
        category: "PRIZE 5 OF 5  *  BEST SISTER OF THE YEAR",
        title: "BEST SISTER OF THE YEAR",
        subtitle: "Warm, sincere, but still celebratory.",
        lines: &[
            "For being an incredible sister, an amazing role model,",
            "and someone I can always look up to.",
        ],
        joke_box: None,
        accent_color: Color::new(255, 215, 0), // Pure Radiant Gold
    },
    // Prize 4 — The Developer Origin Story (Second from right trophy, Scene 2)
    PrizeInfo {
        prize_num: 4,
        category: "PRIZE 4 OF 5  *  THE DEVELOPER ORIGIN STORY",
        title: "THE DEVELOPER ORIGIN STORY",
        subtitle: "Where everything began.",
        lines: &[
            "For introducing a certain idiot to the world of programming.",
        ],
        joke_box: Some(&[
            "Side effects may include Linux, Git, Rust,",
            "broken builds, and questionable life decisions.",
        ]),
        accent_color: Color::new(0, 230, 255), // Cyan
    },
    // Prize 3 — Built Different Award (Middle trophy, Scene 3)
    PrizeInfo {
        prize_num: 3,
        category: "PRIZE 3 OF 5  *  BUILT DIFFERENT",
        title: "BUILT DIFFERENT",
        subtitle: "Actual accomplishments and relentless willpower.",
        lines: &[
            "For consistently doing things that make everyone else wonder:",
            "\"How the hell did she do that?\"",
        ],
        joke_box: None,
        accent_color: Color::new(255, 145, 50), // Electric Amber
    },
    // Prize 2 — World's Most Supportive Sister (Second from left trophy, Scene 4)
    PrizeInfo {
        prize_num: 2,
        category: "PRIZE 2 OF 5  *  WORLD'S MOST SUPPORTIVE SISTER",
        title: "WORLD'S MOST SUPPORTIVE SISTER",
        subtitle: "Quiet, sincere, and deeply meaningful.",
        lines: &[
            "For always being there, helping me, encouraging me,",
            "and making difficult things feel possible.",
        ],
        joke_box: None,
        accent_color: Color::new(255, 130, 185), // Warm Rose
    },
    // Prize 1 — Legendary Sister (Far left trophy, Scene 5)
    PrizeInfo {
        prize_num: 1,
        category: "PRIZE 1 OF 5  *  LEGENDARY SISTER",
        title: "LEGENDARY SISTER",
        subtitle: "The final award before the gift.",
        lines: &[
            "Some people are lucky enough to have a sister.",
            "",
            "I'm lucky enough to have you.",
        ],
        joke_box: None,
        accent_color: Color::new(255, 240, 110), // Starlight Gold
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CinematicState {
    Opening,
    MiloPeeking,
    MiloAtTable,
    Speech1,
    CakeIntro,
    CakeDancing,
    CakeBlown,
    CakeTiramisu,
    EatCakeAndTiramisu,
    SpeechPrizes,
    PrizeSelection(u8), // Current scene (1-5)
    PrizeReveal(u8),    // Which prize (1-5)
    SceneTransition(u8, u8),
    FinalDiscovery,
    FinalGift,
    HandwrittenNote,
    DesktopReveal,
    Finished,
}

#[derive(Clone, Copy)]
struct PrizeHitbox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    prize_num: u8,
}

#[derive(Clone, Copy)]
struct SmokeParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    alpha: f32,
}

#[derive(Clone, Copy)]
struct ConfettiParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    color: Color,
    rotation: f32,
    rot_speed: f32,
}

struct SimpleRng(u32);
impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x12345678 } else { seed })
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FFFFFF) as f32 / 16777216.0
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

pub struct BirthdayCinematic {
    state: CinematicState,
    state_start_time: Instant,

    // Assets
    scene_1: Image,
    scene_2: Image,
    scene_3: Image,
    scene_4: Image,
    scene_5: Image,
    scene_6: Image,
    milo: Image,
    speech_1: Image,
    speech_2: Image,
    speech_3: Image,
    speech_4: Image,
    handwritten_note: Image,
    sister_photo: Option<Image>,
    letter_amma: Option<Image>,
    letter_appa: Option<Image>,
    letter_friend: Option<Image>,
    friend_photo: Option<Image>,
    current_letter_idx: usize,

    // Birthday Cake & Celebration Assets
    cake_with_candles: Image,
    cake_blown: Image,
    tiramisu_asset: Option<Image>,
    cake_alpha: f32,
    smoke_particles: Vec<SmokeParticle>,
    confetti_particles: Vec<ConfettiParticle>,
    music_child: Option<std::process::Child>,
    assets_dir: PathBuf,
    rng: SimpleRng,

    // Animation state
    milo_x: f32,
    milo_y: f32,
    milo_rotation: f32,
    milo_scale: f32,
    milo_alpha: f32,

    speech_alpha: f32,
    speech_scale: f32,

    transition_progress: f32,

    // Prize state
    current_scene: u8, // 1-6
    active_prize_num: u8, // 1-5
    prizes_collected: [bool; 5],
    prize_gui_alpha: f32,
    showing_prize_index: Option<usize>,

    prize_hitboxes: Vec<PrizeHitbox>,
    hovered_prize: Option<u8>,

    // Final gift state
    gift_opened: bool,
    note_alpha: f32,

    // Desktop reveal
    reveal_progress: f32,
}

fn find_assets_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("CINEMATIC_ASSETS_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Ok(p);
        }
    }

    let candidates = [
        PathBuf::from("/cinematic-scene-assets"),
        PathBuf::from("/usr/share/cinematic-scene-assets"),
        PathBuf::from("/usr/share/leviathan/cinematic-scene-assets"),
        PathBuf::from("cinematic-scene-assets"),
        PathBuf::from("../cinematic-scene-assets"),
        PathBuf::from("../../cinematic-scene-assets"),
        PathBuf::from("/home/leviathan/cinematic-scene-assets"),
    ];

    for path in &candidates {
        if path.is_dir() && (path.join("Scene-1.png").exists() || path.join("scene-1.png").exists()) {
            return Ok(path.clone());
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_candidates = [
                exe_dir.join("cinematic-scene-assets"),
                exe_dir.join("../cinematic-scene-assets"),
                exe_dir.join("../../cinematic-scene-assets"),
                exe_dir.join("../../../cinematic-scene-assets"),
                exe_dir.join("../share/cinematic-scene-assets"),
                exe_dir.join("../usr/share/cinematic-scene-assets"),
            ];
            for path in &exe_candidates {
                if path.is_dir() && (path.join("Scene-1.png").exists() || path.join("scene-1.png").exists()) {
                    return Ok(path.clone());
                }
            }
        }
    }

    if Path::new("/cinematic-scene-assets").exists() {
        return Ok(PathBuf::from("/cinematic-scene-assets"));
    }

    Err("Could not find cinematic-scene-assets folder in any known location".to_string())
}

fn load_asset(dir: &Path, name: &str) -> Result<Image, String> {
    let p = dir.join(name);
    if p.exists() {
        if let Ok(img) = Image::load_png(&p) {
            return Ok(img);
        }
    }
    let lower = dir.join(name.to_lowercase());
    if lower.exists() {
        if let Ok(img) = Image::load_png(&lower) {
            return Ok(img);
        }
    }

    // Try png version if jpeg was passed
    if name.ends_with(".jpeg") || name.ends_with(".jpg") {
        let png_name = name.replace(".jpeg", ".png").replace(".jpg", ".png");
        let png_path = dir.join(&png_name);
        if png_path.exists() {
            if let Ok(img) = Image::load_png(&png_path) {
                return Ok(img);
            }
        }
    }

    // Handle sister photo aliases
    if name.contains("sister") {
        for alias in &["me_n_my_sister.png", "m_n_my_sister.png", "me_n_my_sister.jpeg", "m_n_my_sister.jpeg"] {
            let alias_path = dir.join(alias);
            if alias_path.exists() {
                if let Ok(img) = Image::load_png(&alias_path) {
                    return Ok(img);
                }
            }
        }
    }

    let cap = dir.join(match name {
        "scene-1.png" => "Scene-1.png",
        "scene-2.png" => "Scene-2.png",
        "scene-3.png" => "Scene-3.png",
        "scene-4.png" => "Scene-4.png",
        "scene-5.png" => "Scene-5.png",
        "scene-6.png" => "Scene-6.png",
        other => other,
    });
    if cap.exists() {
        if let Ok(img) = Image::load_png(&cap) {
            return Ok(img);
        }
    }
    Err(format!("Asset {} not found in {}", name, dir.display()))
}

impl BirthdayCinematic {
    pub fn new() -> Result<Self, String> {
        println!("Loading birthday cinematic assets...");

        let assets_dir = find_assets_dir()?;
        println!("leviathan-ui: Using cinematic assets from: {}", assets_dir.display());

        let scene_1 = load_asset(&assets_dir, "Scene-1.png")?;
        let scene_2 = load_asset(&assets_dir, "Scene-2.png")?;
        let scene_3 = load_asset(&assets_dir, "Scene-3.png")?;
        let scene_4 = load_asset(&assets_dir, "Scene-4.png")?;
        let scene_5 = load_asset(&assets_dir, "Scene-5.png")?;
        let scene_6 = load_asset(&assets_dir, "Scene-6.png")?;
        let milo = load_asset(&assets_dir, "milo.png")?;
        let speech_1 = load_asset(&assets_dir, "speech-1.png")?;
        let speech_2 = load_asset(&assets_dir, "speech-2.png")?;
        let speech_3 = load_asset(&assets_dir, "speech-3.png")?;
        let speech_4 = load_asset(&assets_dir, "speech-4.png")?;
        let handwritten_note = load_asset(&assets_dir, "handwritten-note.png")?;
        let cake_with_candles = load_asset(&assets_dir, "cake_with_candles.png")?;
        let cake_blown = load_asset(&assets_dir, "cake_blown.png")?;
        let tiramisu_asset = load_asset(&assets_dir, "tiramisu.png").ok();

        // Optional photo of developer and sister placed to the right of the letter
        let sister_photo = load_asset(&assets_dir, "m_n_my_sister.png")
            .or_else(|_| load_asset(&assets_dir, "me_n_my_sister.png"))
            .or_else(|_| load_asset(&assets_dir, "m_n_my_sister.jpeg"))
            .or_else(|_| Image::load_png("/home/inchara/Downloads/me_n_my_sister.png"))
            .ok();

        // Birthday letters from family and friends
        let letter_amma = load_asset(&assets_dir, "letter_from_amma.png").ok();
        let letter_appa = load_asset(&assets_dir, "letter_from_appa.png").ok();
        let letter_friend = load_asset(&assets_dir, "letter_from_friend.png").ok();
        let friend_photo = load_asset(&assets_dir, "friend.png").ok();

        if sister_photo.is_some() {
            println!("leviathan-ui: Sister photo asset loaded successfully!");
        }
        if letter_amma.is_some() {
            println!("leviathan-ui: Amma letter asset loaded successfully!");
        }
        if letter_appa.is_some() {
            println!("leviathan-ui: Appa letter asset loaded successfully!");
        }
        if letter_friend.is_some() {
            println!("leviathan-ui: Medha letter asset loaded successfully!");
        }
        if friend_photo.is_some() {
            println!("leviathan-ui: Medha friend photo asset loaded successfully!");
        }

        println!("leviathan-ui: Birthday cinematic assets loaded successfully!");

        let prize_hitboxes = Self::create_hitboxes_for_scene(1);

        Ok(Self {
            state: CinematicState::Opening,
            state_start_time: Instant::now(),

            scene_1,
            scene_2,
            scene_3,
            scene_4,
            scene_5,
            scene_6,
            milo,
            speech_1,
            speech_2,
            speech_3,
            speech_4,
            handwritten_note,
            sister_photo,
            letter_amma,
            letter_appa,
            letter_friend,
            friend_photo,
            current_letter_idx: 0,
            cake_with_candles,
            cake_blown,
            tiramisu_asset,
            cake_alpha: 0.0,
            smoke_particles: Vec::new(),
            confetti_particles: Vec::new(),
            music_child: None,
            assets_dir,
            rng: SimpleRng::new(0xCAFE_BABE),

            // Milo animation state - begins off-screen left, ready to peek head in
            milo_x: -360.0,
            milo_y: 520.0,
            milo_rotation: 0.0,
            milo_scale: 0.82,
            milo_alpha: 1.0,

            speech_alpha: 0.0,
            speech_scale: 0.9,

            transition_progress: 0.0,

            current_scene: 6,
            active_prize_num: 5,
            prizes_collected: [false; 5],
            prize_gui_alpha: 0.0,
            showing_prize_index: None,

            prize_hitboxes,
            hovered_prize: None,

            gift_opened: false,
            note_alpha: 0.0,

            reveal_progress: 0.0,
        })
    }

    /// Accurate trophy bounding boxes per scene (in 1920x1080 canvas coordinates)
    /// Order is RIGHT TO LEFT: Prize 5 (far right) down to Prize 1 (far left)
    fn create_hitboxes_for_scene(scene_num: u8) -> Vec<PrizeHitbox> {
        match scene_num {
            1 => vec![
                PrizeHitbox { x: 1160, y: 530, w: 180, h: 170, prize_num: 5 }, // Far right
            ],
            2 => vec![
                PrizeHitbox { x: 1110, y: 550, w: 185, h: 175, prize_num: 4 }, // 2nd from right
            ],
            3 => vec![
                PrizeHitbox { x: 980, y: 550, w: 180, h: 180, prize_num: 3 },  // Middle
            ],
            4 => vec![
                PrizeHitbox { x: 715, y: 545, w: 185, h: 170, prize_num: 2 },  // 2nd from left
            ],
            5 => vec![
                PrizeHitbox { x: 535, y: 540, w: 185, h: 175, prize_num: 1 },  // Far left
            ],
            6 => vec![
                PrizeHitbox { x: 760, y: 780, w: 160, h: 130, prize_num: 0 }, // Secret gift under table
            ],
            _ => vec![],
        }
    }

    pub fn update(&mut self, mouse: &MouseState, key_advance: bool, dt: f32) {
        let elapsed = self.state_start_time.elapsed().as_secs_f32();
        let trigger = mouse.left_clicked || key_advance;

        match self.state {
            CinematicState::Opening => {
                if elapsed > 0.5 || trigger {
                    self.change_state(CinematicState::MiloPeeking);
                }
            }

            CinematicState::MiloPeeking => {
                let progress = (elapsed * 0.9).min(1.0);
                let eased = Self::ease_out_cubic(progress);

                // Milo tilts in from the left edge so only the head and ears peek past the left bezel
                self.milo_x = -360.0 + eased * 85.0; // head peeks into frame at x >= 0
                self.milo_rotation = 14.0 * eased;

                if elapsed > 2.0 || (trigger && elapsed > 0.3) {
                    self.change_state(CinematicState::MiloAtTable);
                }
            }

            CinematicState::MiloAtTable => {
                let progress = (elapsed * 0.8).min(1.0);
                let eased = Self::ease_in_out_cubic(progress);

                // Milo returns upright and trots across the rug to the table
                self.milo_x = -275.0 + eased * 345.0; // arrives at x = 70
                self.milo_y = 520.0;
                self.milo_rotation = 14.0 * (1.0 - eased);

                if elapsed > 1.8 || (trigger && elapsed > 0.3) {
                    self.change_state(CinematicState::Speech1);
                }
            }

            CinematicState::Speech1 => {
                let progress = (elapsed * 2.5).min(1.0);
                self.speech_alpha = progress;
                self.speech_scale = 0.9 + progress * 0.1;

                if elapsed > 0.3 && (trigger || elapsed > 6.0) {
                    self.speech_alpha = 0.0;
                    self.play_birthday_music();
                    self.change_state(CinematicState::CakeIntro);
                }
            }

            CinematicState::CakeIntro => {
                let progress = (elapsed / 0.8).min(1.0);
                self.cake_alpha = Self::ease_out_cubic(progress);

                // Milo starts bopping gently
                let beat = elapsed * 2.1;
                let hop = (beat * std::f32::consts::PI).sin().abs() * 8.0;
                self.milo_y = 520.0 - hop;
                self.milo_rotation = (beat * std::f32::consts::PI * 0.5).sin() * 4.0;

                if elapsed > 0.8 || (trigger && elapsed > 0.2) {
                    self.cake_alpha = 1.0;
                    self.change_state(CinematicState::CakeDancing);
                }
            }

            CinematicState::CakeDancing => {
                self.cake_alpha = 1.0;

                // Milo cute synchronized hops and wobbles to the Happy Birthday music tempo (~126 BPM)
                let beat = elapsed * 2.1;
                let hop = (beat * std::f32::consts::PI).sin().abs() * 18.0;
                self.milo_y = 520.0 - hop;
                self.milo_rotation = (beat * std::f32::consts::PI * 0.5).sin() * 7.0;
                self.milo_scale = 0.82 + (hop / 18.0) * 0.04;

                // Cake click hitbox on table
                let cake_hovered = mouse.x >= 650 && mouse.x < 1150 && mouse.y >= 280 && mouse.y < 700;

                // Blow out candles when clicking cake, or clicking bottom pill / pressing space
                if (cake_hovered && mouse.left_clicked) || (trigger && elapsed > 0.3) {
                    self.spawn_smoke_particles();
                    self.spawn_confetti_particles();
                    self.change_state(CinematicState::CakeBlown);
                }
            }

            CinematicState::CakeBlown => {
                self.cake_alpha = 1.0;
                // Milo stops hopping and stands contentedly with joyful pose
                self.milo_y = 520.0;
                self.milo_rotation = 2.0;
                self.milo_scale = 0.82;

                self.update_particles(dt);

                // Celebratory pause (2.5s) or click after 0.5s to advance
                if elapsed > 2.5 || (trigger && elapsed > 0.5) {
                    self.change_state(CinematicState::CakeTiramisu);
                }
            }

            CinematicState::CakeTiramisu => {
                self.cake_alpha = 1.0;
                self.milo_y = 520.0;
                self.milo_rotation = 0.0;
                self.milo_scale = 0.82;
                self.update_particles(dt);

                let progress = (elapsed * 2.5).min(1.0);
                self.speech_alpha = progress;

                // Speech 2 RIGHT AFTER the cake: "ANDD OF COURSE, THE TIRAMISU!" (speech-4.png)
                if elapsed > 0.3 && (trigger || elapsed > 5.5) {
                    self.speech_alpha = 0.0;
                    self.change_state(CinematicState::EatCakeAndTiramisu);
                }
            }

            CinematicState::EatCakeAndTiramisu => {
                self.milo_y = 520.0;
                self.milo_rotation = 0.0;
                self.milo_scale = 0.82;
                self.update_particles(dt);

                let cake_or_tiramisu_hovered = (mouse.x >= 650 && mouse.x < 1150 && mouse.y >= 280 && mouse.y < 700)
                    || (mouse.x >= 1040 && mouse.x < 1340 && mouse.y >= 400 && mouse.y < 700);

                if self.cake_alpha >= 0.99 && ((cake_or_tiramisu_hovered && mouse.left_clicked) || (trigger && elapsed > 0.3) || elapsed > 6.0) {
                    self.spawn_confetti_particles();
                    self.cake_alpha = 0.98;
                }

                if self.cake_alpha < 0.99 {
                    self.cake_alpha = (self.cake_alpha - dt * 2.2).max(0.0);
                    if self.cake_alpha <= 0.01 {
                        self.cake_alpha = 0.0;
                        self.change_state(CinematicState::SpeechPrizes);
                    }
                }
            }

            CinematicState::SpeechPrizes => {
                self.cake_alpha = 0.0;
                self.milo_y = 520.0;
                self.milo_rotation = 0.0;
                self.milo_scale = 0.82;
                self.update_particles(dt);

                let progress = (elapsed * 2.5).min(1.0);
                self.speech_alpha = progress;

                // Speech 3 AFTER eating the cake and tiramisu: "OOO, THERE ARE SOME PRIZES FOR YOU" (speech-2.png)
                if elapsed > 0.3 && (trigger || elapsed > 5.5) {
                    self.speech_alpha = 0.0;
                    self.stop_birthday_music();
                    self.active_prize_num = 5;
                    self.prize_hitboxes = Self::create_hitboxes_for_scene(1);
                    self.change_state(CinematicState::SceneTransition(6, 1));
                }
            }

            CinematicState::PrizeSelection(_scene) => {
                self.hovered_prize = None;
                let mut clicked_prize = None;

                for hitbox in &self.prize_hitboxes {
                    if hitbox.prize_num == self.active_prize_num {
                        if mouse.x >= hitbox.x && mouse.x < hitbox.x + hitbox.w &&
                           mouse.y >= hitbox.y && mouse.y < hitbox.y + hitbox.h {
                            self.hovered_prize = Some(hitbox.prize_num);

                            if trigger {
                                clicked_prize = Some(hitbox.prize_num);
                                break;
                            }
                        }
                    }
                }

                // If user clicks on the prompt pill at the bottom OR clicks anywhere after brief delay
                if clicked_prize.is_none() && trigger {
                    if mouse.y >= 980 || elapsed > 0.3 || key_advance {
                        clicked_prize = Some(self.active_prize_num);
                    }
                }

                if let Some(prize_num) = clicked_prize {
                    // prize_num 5 -> idx 0, prize_num 4 -> idx 1, ..., prize_num 1 -> idx 4
                    let prize_idx = (5 - prize_num) as usize;
                    if prize_idx < 5 {
                        self.prizes_collected[prize_idx] = true;
                        self.showing_prize_index = Some(prize_idx);
                        self.play_sound_effect("prize.wav");
                        self.change_state(CinematicState::PrizeReveal(prize_num));
                    }
                }
            }

            CinematicState::PrizeReveal(prize_num) => {
                let progress = (elapsed * 2.5).min(1.0);
                self.prize_gui_alpha = progress;

                if elapsed > 0.3 && (trigger || elapsed > 8.0) {
                    if prize_num == 1 {
                        // All 5 trophies collected! Transition to scene 6
                        self.change_state(CinematicState::SceneTransition(5, 6));
                    } else {
                        let next_scene = self.current_scene + 1;
                        self.active_prize_num = prize_num - 1;
                        self.change_state(CinematicState::SceneTransition(self.current_scene, next_scene));
                    }
                }
            }

            CinematicState::SceneTransition(_from, to) => {
                let progress = (elapsed * 1.5).min(1.0);
                self.transition_progress = Self::ease_in_out_quad(progress);

                if progress >= 1.0 {
                    self.current_scene = to;
                    self.prize_gui_alpha = 0.0;
                    self.showing_prize_index = None;

                    if to == 6 {
                        self.change_state(CinematicState::FinalDiscovery);
                    } else {
                        self.prize_hitboxes = Self::create_hitboxes_for_scene(to);
                        self.change_state(CinematicState::PrizeSelection(to));
                    }
                }
            }

            CinematicState::FinalDiscovery => {
                let progress = (elapsed * 2.0).min(1.0);
                self.speech_alpha = progress;

                if elapsed > 0.3 && (trigger || elapsed > 6.0) {
                    self.speech_alpha = 0.0;
                    self.prize_hitboxes = Self::create_hitboxes_for_scene(6);
                    self.change_state(CinematicState::FinalGift);
                }
            }

            CinematicState::FinalGift => {
                self.hovered_prize = None;
                for hitbox in &self.prize_hitboxes {
                    if hitbox.prize_num == 0 {
                        if mouse.x >= hitbox.x && mouse.x < hitbox.x + hitbox.w &&
                           mouse.y >= hitbox.y && mouse.y < hitbox.y + hitbox.h {
                            self.hovered_prize = Some(0);
                        }
                    }
                }

                if !self.gift_opened {
                    if (trigger && elapsed > 0.3) || (self.hovered_prize.is_some() && mouse.left_clicked) {
                        self.gift_opened = true;
                        self.current_letter_idx = 0;
                        self.change_state(CinematicState::HandwrittenNote);
                    }
                }
            }

            CinematicState::HandwrittenNote => {
                let progress = (elapsed * 2.5).min(1.0);
                self.note_alpha = progress;

                if elapsed > 0.35 && trigger {
                    if self.current_letter_idx + 1 < 4 {
                        self.current_letter_idx += 1;
                        self.state_start_time = Instant::now();
                        self.note_alpha = 0.0;
                    } else {
                        self.change_state(CinematicState::DesktopReveal);
                    }
                }
            }

            CinematicState::DesktopReveal => {
                let duration = 2.5; // 2.5s cinematic transition to desktop wallpaper
                let progress = (elapsed / duration).min(1.0);
                let eased = Self::ease_in_out_cubic(progress);

                // 1. Note and photo fade out in the first 0.5s
                self.note_alpha = (1.0 - progress * 4.0).max(0.0);

                // 2. Milo moves from (70, 520) to bottom-right corner (1806, 922)
                let start_x = 70.0;
                let start_y = 520.0;
                let target_x = 1806.0;
                let target_y = 922.0;

                self.milo_x = start_x + (target_x - start_x) * eased;
                let base_y = start_y + (target_y - start_y) * eased;

                // Trotting bounce and gentle wobble while walking across the room to the corner
                if progress < 0.95 {
                    let walk_step = progress * 32.0;
                    let hop = walk_step.sin().abs() * 10.0 * (1.0 - progress);
                    self.milo_y = base_y - hop;
                    self.milo_rotation = walk_step.sin() * 3.5 * (1.0 - progress);
                } else {
                    self.milo_y = target_y;
                    self.milo_rotation = 0.0;
                }

                // 3. Milo shrinks down smoothly to companion size (90x90)
                let start_scale = 0.82;
                let target_scale = 90.0 / self.milo.height as f32; // 0.17964
                self.milo_scale = start_scale + (target_scale - start_scale) * eased;
                self.milo_alpha = 1.0;

                self.reveal_progress = self.ui_alpha();

                if progress >= 1.0 {
                    self.change_state(CinematicState::Finished);
                }
            }

            CinematicState::Finished => {}
        }
    }

    pub fn render(&self, gfx: &mut Graphics) {
        let width = gfx.width();
        let height = gfx.height();
        let pitch = gfx.pitch();

        // Canvas scaling factors for responsive layout
        let scale_x = width as f32 / 1920.0;
        let scale_y = height as f32 / 1080.0;

        // If in DesktopReveal: Desktop has ALREADY drawn the room wallpaper and fading-in top bar + dock!
        if let CinematicState::DesktopReveal = self.state {
            // 1. Draw note + sister photo fading out (first 0.5s of transition)
            if self.note_alpha > 0.001 {
                let overlay_alpha = (self.note_alpha * 205.0) as u8;
                gfx.fill_rect(0, 0, width as i32, height as i32, Color::new(0, 0, 0).with_alpha(overlay_alpha));
                self.draw_note_and_photo(gfx, width, height, pitch, self.note_alpha);
            }

            // 2. Draw Milo trotting across the room and shrinking into the corner
            let milo_w = (self.milo.width as f32 * self.milo_scale * scale_x) as u32;
            let milo_h = (self.milo.height as f32 * self.milo_scale * scale_y) as u32;
            let mx = (self.milo_x * scale_x) as i32;
            let my = (self.milo_y * scale_y) as i32;

            let buf = gfx.backbuffer_mut();
            if self.milo_rotation.abs() > 0.5 {
                let angle_rad = self.milo_rotation.to_radians();
                let cx = mx + (milo_w / 2) as i32;
                let cy = my + (milo_h / 2) as i32;
                self.milo.draw_to_buffer_rotated_scaled(
                    buf, width, height, pitch, cx, cy,
                    self.milo_scale * scale_x, self.milo_scale * scale_y, angle_rad, 1.0
                );
            } else {
                self.milo.draw_to_buffer_scaled(buf, width, height, pitch, mx, my, milo_w, milo_h, 1.0);
            }

            gfx.mark_all_dirty();
            return;
        }

        // 1. Render background image - ALWAYS SCALED TO FILL 100% OF THE SCREEN
        {
            let buf = gfx.backbuffer_mut();

            let current_bg = match self.current_scene {
                1 => &self.scene_1,
                2 => &self.scene_2,
                3 => &self.scene_3,
                4 => &self.scene_4,
                5 => &self.scene_5,
                6 => &self.scene_6,
                _ => &self.scene_1,
            };

            if let CinematicState::SceneTransition(from, to) = self.state {
                let from_bg = match from {
                    1 => &self.scene_1,
                    2 => &self.scene_2,
                    3 => &self.scene_3,
                    4 => &self.scene_4,
                    5 => &self.scene_5,
                    6 => &self.scene_6,
                    _ => &self.scene_6,
                };
                let to_bg = match to {
                    1 => &self.scene_1,
                    2 => &self.scene_2,
                    3 => &self.scene_3,
                    4 => &self.scene_4,
                    5 => &self.scene_5,
                    6 => &self.scene_6,
                    _ => &self.scene_1,
                };

                from_bg.draw_to_buffer_scaled(buf, width, height, pitch, 0, 0, width, height, 1.0 - self.transition_progress);
                to_bg.draw_to_buffer_scaled(buf, width, height, pitch, 0, 0, width, height, self.transition_progress);
            } else {
                current_bg.draw_to_buffer_scaled(buf, width, height, pitch, 0, 0, width, height, 1.0);
            }

            // Draw Milo (scaled to sit naturally on the floor rug, or rotated when peeking from edge)
            if !matches!(self.state, CinematicState::Finished) {
                let milo_w = (self.milo.width as f32 * self.milo_scale * scale_x) as u32;
                let milo_h = (self.milo.height as f32 * self.milo_scale * scale_y) as u32;
                let mx = (self.milo_x * scale_x) as i32;
                let my = (self.milo_y * scale_y) as i32;

                if self.milo_rotation.abs() > 0.5 {
                    let angle_rad = self.milo_rotation.to_radians();
                    let cx = mx + (milo_w / 2) as i32;
                    let cy = my + (milo_h / 2) as i32;
                    self.milo.draw_to_buffer_rotated_scaled(
                        buf, width, height, pitch, cx, cy,
                        self.milo_scale * scale_x, self.milo_scale * scale_y, angle_rad, self.milo_alpha
                    );
                } else {
                    self.milo.draw_to_buffer_scaled(buf, width, height, pitch, mx, my, milo_w, milo_h, self.milo_alpha);
                }
            }

            // Draw speech bubbles pointing directly at Milo
            match self.state {
                CinematicState::Speech1 => {
                    let sw = (self.speech_1.width as f32 * scale_x) as u32;
                    let sh = (self.speech_1.height as f32 * scale_y) as u32;
                    let sx = (40.0 * scale_x) as i32;
                    let sy = (290.0 * scale_y) as i32;
                    self.speech_1.draw_to_buffer_scaled(buf, width, height, pitch, sx, sy, sw, sh, self.speech_alpha);
                }
                CinematicState::CakeTiramisu => {
                    let sw = (self.speech_4.width as f32 * scale_x) as u32;
                    let sh = (self.speech_4.height as f32 * scale_y) as u32;
                    let sx = (40.0 * scale_x) as i32;
                    let sy = (290.0 * scale_y) as i32;
                    self.speech_4.draw_to_buffer_scaled(buf, width, height, pitch, sx, sy, sw, sh, self.speech_alpha);
                }
                CinematicState::SpeechPrizes => {
                    let sw = (self.speech_2.width as f32 * scale_x) as u32;
                    let sh = (self.speech_2.height as f32 * scale_y) as u32;
                    let sx = (40.0 * scale_x) as i32;
                    let sy = (290.0 * scale_y) as i32;
                    self.speech_2.draw_to_buffer_scaled(buf, width, height, pitch, sx, sy, sw, sh, self.speech_alpha);
                }
                CinematicState::FinalDiscovery => {
                    let sw = (self.speech_3.width as f32 * scale_x) as u32;
                    let sh = (self.speech_3.height as f32 * scale_y) as u32;
                    let sx = (40.0 * scale_x) as i32;
                    let sy = (260.0 * scale_y) as i32;
                    self.speech_3.draw_to_buffer_scaled(buf, width, height, pitch, sx, sy, sw, sh, self.speech_alpha);
                }
                _ => {}
            }

            // Draw big birthday cake above table during cake celebration
            let is_cake_state = matches!(
                self.state,
                CinematicState::CakeIntro
                    | CinematicState::CakeDancing
                    | CinematicState::CakeBlown
                    | CinematicState::CakeTiramisu
                    | CinematicState::EatCakeAndTiramisu
            );

            if is_cake_state && self.cake_alpha > 0.01 {
                let cw = (440.0 * scale_x) as u32;
                let ch = (440.0 * scale_y) as u32;
                let cx = (680.0 * scale_x) as i32;
                let cy = (295.0 * scale_y) as i32;

                let cake_img = if matches!(self.state, CinematicState::CakeIntro | CinematicState::CakeDancing) {
                    &self.cake_with_candles
                } else {
                    &self.cake_blown
                };

                cake_img.draw_to_buffer_scaled(buf, width, height, pitch, cx, cy, cw, ch, self.cake_alpha);
            }

            // Draw tiramisu on table next to cake
            if matches!(self.state, CinematicState::CakeTiramisu | CinematicState::EatCakeAndTiramisu) {
                if let Some(ref tiramisu) = self.tiramisu_asset {
                    let tw = (260.0 * scale_x) as u32;
                    let th = (260.0 * scale_y) as u32;
                    let tx = (1060.0 * scale_x) as i32;
                    let ty = (420.0 * scale_y) as i32;
                    tiramisu.draw_to_buffer_scaled(buf, width, height, pitch, tx, ty, tw, th, self.cake_alpha);
                }
            }
        } // Buffer borrow released

        // 2. Interactive Highlights & Visual Prompts
        let elapsed = self.state_start_time.elapsed().as_secs_f32();
        let pulse = ((elapsed * 4.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);

        // Candlelight animated warm glow around candles
        if matches!(self.state, CinematicState::CakeDancing | CinematicState::CakeIntro) && self.cake_alpha > 0.1 {
            let flicker = ((elapsed * 13.0).sin() * 0.15 + (elapsed * 21.0).cos() * 0.1) * self.cake_alpha;
            let candle_flames = [
                ((680.0 + 440.0 * 0.342) * scale_x, (295.0 + 440.0 * 0.300) * scale_y),
                ((680.0 + 440.0 * 0.426) * scale_x, (295.0 + 440.0 * 0.186) * scale_y),
                ((680.0 + 440.0 * 0.596) * scale_x, (295.0 + 440.0 * 0.194) * scale_y),
                ((680.0 + 440.0 * 0.596) * scale_x, (295.0 + 440.0 * 0.406) * scale_y),
            ];

            for (fx, fy) in candle_flames {
                let fx_i = fx as i32;
                let fy_i = fy as i32;
                let rad1 = ((16.0 + 3.0 * flicker) * scale_y) as i32;
                let rad2 = ((8.0 + 2.0 * flicker) * scale_y) as i32;
                let alpha1 = ((50.0 + 30.0 * flicker) * self.cake_alpha) as u8;
                let alpha2 = ((120.0 + 40.0 * flicker) * self.cake_alpha) as u8;
                gfx.fill_circle(fx_i, fy_i, rad1, Color::new(255, 190, 40).with_alpha(alpha1));
                gfx.fill_circle(fx_i, fy_i, rad2, Color::new(255, 255, 190).with_alpha(alpha2));
            }
        }

        // Floating musical notes from Milo while dancing
        if let CinematicState::CakeDancing = self.state {
            let notes = ["*", "+", "*"];
            for (i, symbol) in notes.iter().enumerate() {
                let offset = (elapsed * 1.5 + i as f32 * 0.7) % 2.0;
                let ny = ((440.0 - offset * 90.0) * scale_y) as i32;
                let nx = ((210.0 + (offset * 5.0).sin() * 20.0 + i as f32 * 35.0) * scale_x) as i32;
                let n_alpha = ((1.0 - (offset / 2.0)).max(0.0) * 220.0) as u8;
                gfx.draw_text_scaled(nx, ny, symbol, 2, Color::new(255, 235, 120).with_alpha(n_alpha));
            }
        }

        // Draw smoke particles rising from extinguished candle wicks
        for p in &self.smoke_particles {
            let sx = (p.x * scale_x) as i32;
            let sy = (p.y * scale_y) as i32;
            let s_rad = ((p.size / 2.0) * scale_y).max(2.0) as i32;
            let s_alpha = (p.alpha * 120.0) as u8;
            gfx.fill_circle(sx, sy, s_rad, Color::new(220, 225, 235).with_alpha(s_alpha));
        }

        // Draw confetti particles
        for p in &self.confetti_particles {
            let sx = (p.x * scale_x) as i32;
            let sy = (p.y * scale_y) as i32;
            let sw = (p.size * scale_x).max(3.0) as i32;
            let sh = ((p.size * 0.6) * scale_y).max(2.0) as i32;
            gfx.fill_rect(sx, sy, sw, sh, p.color);
        }

        // Speech continuation pill
        if matches!(
            self.state,
            CinematicState::Speech1
                | CinematicState::CakeTiramisu
                | CinematicState::SpeechPrizes
                | CinematicState::FinalDiscovery
        ) {
            if self.speech_alpha > 0.85 {
                let prompt = "[ ✦ Click anywhere or press Space to continue ✦ ]";
                self.draw_bottom_pill(gfx, prompt, Color::new(245, 245, 250), Color::new(140, 170, 230), pulse);
            }
        } else if let CinematicState::CakeDancing = self.state {
            let prompt = "[ ✦ CLICK THE CAKE OR PRESS SPACE TO BLOW OUT CANDLES! ✦ ]";
            self.draw_bottom_pill(gfx, prompt, Color::new(255, 235, 110), Color::new(255, 180, 40), pulse);
        } else if let CinematicState::CakeBlown = self.state {
            let prompt = "[ ✦ HAPPY BIRTHDAY! ✦ ]";
            self.draw_bottom_pill(gfx, prompt, Color::new(255, 245, 255), Color::new(255, 100, 180), pulse);
        } else if let CinematicState::EatCakeAndTiramisu = self.state {
            let prompt = "[ ✦ CLICK THE CAKE OR TIRAMISU TO ENJOY! 🍰✨ ✦ ]";
            self.draw_bottom_pill(gfx, prompt, Color::new(255, 240, 180), Color::new(255, 170, 60), pulse);
        }

        // Active trophy delicate glint and interactive highlight in PrizeSelection
        if let CinematicState::PrizeSelection(_) = self.state {
            for hitbox in &self.prize_hitboxes {
                if hitbox.prize_num == self.active_prize_num {
                    let is_hovered = self.hovered_prize == Some(hitbox.prize_num);

                    // Subtle, elegant hover outline only when hovered (or very faint whisper when idle)
                    if is_hovered {
                        gfx.draw_rounded_rect(hitbox.x, hitbox.y, hitbox.w, hitbox.h, 12, Color::new(255, 225, 120).with_alpha(140));
                    } else {
                        let faint_alpha = (30.0 + 25.0 * pulse) as u8;
                        gfx.draw_rounded_rect(hitbox.x, hitbox.y, hitbox.w, hitbox.h, 12, Color::new(255, 215, 80).with_alpha(faint_alpha));
                    }

                    // A delicate, gentle glint sparkle at the top shine point of the trophy
                    let glint_x = hitbox.x + hitbox.w - 24;
                    let glint_y = hitbox.y + 16;
                    let glint_alpha = ((pulse * 170.0) as u8).max(25);

                    let glint_core = Color::new(255, 255, 245).with_alpha(glint_alpha);
                    let glint_ray = Color::new(255, 230, 130).with_alpha((glint_alpha as f32 * 0.6) as u8);

                    // Tiny 3x3 diamond glint (clean, small, subtle sparkle)
                    gfx.put_pixel(glint_x, glint_y, glint_core);
                    gfx.put_pixel(glint_x - 1, glint_y, glint_ray);
                    gfx.put_pixel(glint_x + 1, glint_y, glint_ray);
                    gfx.put_pixel(glint_x, glint_y - 1, glint_ray);
                    gfx.put_pixel(glint_x, glint_y + 1, glint_ray);
                }
            }

            let prompt = format!("[ ✦ CLICK ON THE TROPHY TO REVEAL AWARD #{} ✦ ]", self.active_prize_num);
            self.draw_bottom_pill(gfx, &prompt, Color::new(255, 235, 110), Color::new(255, 195, 45), pulse);
        }

        // Final secret gift under table highlight
        if let CinematicState::FinalGift = self.state {
            let hx = 760;
            let hy = 780;
            let hw = 160;
            let hh = 130;
            let is_hovered = self.hovered_prize == Some(0);

            if is_hovered {
                gfx.draw_rounded_rect(hx, hy, hw, hh, 12, Color::new(120, 240, 255).with_alpha(150));
            } else {
                let faint_alpha = (30.0 + 25.0 * pulse) as u8;
                gfx.draw_rounded_rect(hx, hy, hw, hh, 12, Color::new(100, 220, 255).with_alpha(faint_alpha));
            }

            let gx = hx + hw / 2;
            let gy = hy + 14;
            let glint_alpha = ((pulse * 170.0) as u8).max(25);
            let g_core = Color::new(240, 255, 255).with_alpha(glint_alpha);
            let g_ray = Color::new(100, 230, 255).with_alpha((glint_alpha as f32 * 0.6) as u8);
            gfx.put_pixel(gx, gy, g_core);
            gfx.put_pixel(gx - 1, gy, g_ray);
            gfx.put_pixel(gx + 1, gy, g_ray);
            gfx.put_pixel(gx, gy - 1, g_ray);
            gfx.put_pixel(gx, gy + 1, g_ray);

            let prompt = "[ ✦ CLICK ON THE GIFT UNDER THE TABLE TO OPEN IT! ✦ ]";
            self.draw_bottom_pill(gfx, prompt, Color::new(100, 240, 255), Color::new(0, 205, 250), pulse);
        }

        // 3. Draw luxury prize GUI modal card
        if let CinematicState::PrizeReveal(_) = self.state {
            if let Some(prize_idx) = self.showing_prize_index {
                self.draw_prize_modal(gfx, &PRIZES[prize_idx]);
            }
        }

        // 4. Draw handwritten note + sister / Medha photo
        if let CinematicState::HandwrittenNote = self.state {
            // Dark vignette room curtain (softly alpha blended so background room remains visible)
            gfx.fill_rect(0, 0, width as i32, height as i32, Color::new(0, 0, 0).with_alpha(120));
            self.draw_note_and_photo(gfx, width, height, pitch, self.note_alpha);

            if self.note_alpha > 0.85 {
                let prompt = match self.current_letter_idx {
                    0 => "[ ✦ Letter 1 of 4: Sister · Click or Space for Next Letter ✦ ]",
                    1 => "[ ✦ Letter 2 of 4: Amma · Click or Space for Next Letter ✦ ]",
                    2 => "[ ✦ Letter 3 of 4: Appa · Click or Space for Next Letter ✦ ]",
                    _ => "[ ✦ Letter 4 of 4: Medha · Click or Space to Enter Incy OS ✦ ]",
                };
                self.draw_bottom_pill(gfx, prompt, Color::new(255, 255, 255), Color::new(255, 215, 0), pulse);
            }
        }

        gfx.mark_all_dirty();
    }

    /// Renders a luxury floating glassmorphism prompt pill at the bottom of the screen with radiant pulsing glow
    fn draw_bottom_pill(&self, gfx: &mut Graphics, text: &str, fg: Color, border: Color, pulse: f32) {
        let width = gfx.width() as i32;
        let height = gfx.height() as i32;

        let pill_w = (text.len() as i32 * 8) + 56;
        let pill_h = 42;
        let px = (width - pill_w) / 2;
        let py = height - 76; // Floating gracefully above the bottom edge

        // 1. Soft radiant outer glow shadow layers that pulse with breathing rhythm
        let glow_alpha1 = (35.0 + 40.0 * pulse) as u8;
        let glow_alpha2 = (70.0 + 55.0 * pulse) as u8;
        gfx.fill_rounded_rect(px - 8, py - 5, pill_w + 16, pill_h + 10, 18, border.with_alpha(glow_alpha1));
        gfx.fill_rounded_rect(px - 4, py - 2, pill_w + 8, pill_h + 4, 15, border.with_alpha(glow_alpha2));

        // 2. Glassmorphism dark acrylic container body
        gfx.fill_rounded_rect(px, py, pill_w, pill_h, 12, Color::new(14, 18, 26).with_alpha(245));

        // 3. Multi-layer glowing border
        let border_pulse = Color::new(
            (border.r as f32 * (0.8 + 0.2 * pulse)).min(255.0) as u8,
            (border.g as f32 * (0.8 + 0.2 * pulse)).min(255.0) as u8,
            (border.b as f32 * (0.8 + 0.2 * pulse)).min(255.0) as u8,
        );
        gfx.draw_rounded_rect(px, py, pill_w, pill_h, 12, border_pulse);
        gfx.draw_rounded_rect(px + 1, py + 1, pill_w - 2, pill_h - 2, 11, Color::new(255, 255, 255).with_alpha((60.0 + 40.0 * pulse) as u8));

        // 4. Centered crisp glowing text with drop shadow
        let text_x = px + (pill_w - text.len() as i32 * 8) / 2;
        let text_y = py + 16;
        gfx.draw_text_shadow(text_x, text_y, text, 1, fg, Color::new(0, 0, 0));
    }

    /// Renders the luxury trophy reveal modal card with rich typography and glowing borders
    fn draw_prize_modal(&self, gfx: &mut Graphics, prize: &PrizeInfo) {
        let width = gfx.width() as i32;
        let height = gfx.height() as i32;

        let has_joke = prize.joke_box.is_some();
        let gui_w = 780;
        let gui_h = if has_joke { 460 } else { 400 };
        let gui_x = (width - gui_w) / 2;
        let gui_y = (height - gui_h) / 2;

        let alpha = (self.prize_gui_alpha * 255.0) as u8;
        let card_alpha = (self.prize_gui_alpha * 230.0) as u8;

        // Dim background backdrop softly so room scene is clearly visible behind modal
        gfx.fill_rect(0, 0, width, height, Color::new(0, 0, 0).with_alpha((self.prize_gui_alpha * 90.0) as u8));

        // Soft outer shadow & subtle warm accent glow
        gfx.fill_rounded_rect(gui_x + 6, gui_y + 8, gui_w, gui_h, 18, Color::new(0, 0, 0).with_alpha((self.prize_gui_alpha * 70.0) as u8));
        gfx.fill_rounded_rect(gui_x - 2, gui_y - 2, gui_w + 4, gui_h + 4, 18, prize.accent_color.with_alpha((self.prize_gui_alpha * 25.0) as u8));

        // Card glassmorphism acrylic body
        gfx.fill_gradient_v(gui_x, gui_y, gui_w, gui_h,
                            Color::new(20, 22, 32).with_alpha(card_alpha),
                            Color::new(12, 14, 22).with_alpha(card_alpha));

        // Crisp elegant glowing borders
        gfx.draw_rounded_rect(gui_x, gui_y, gui_w, gui_h, 16, prize.accent_color.with_alpha((self.prize_gui_alpha * 220.0) as u8));
        gfx.draw_rounded_rect(gui_x + 1, gui_y + 1, gui_w - 2, gui_h - 2, 15, Color::new(255, 255, 255).with_alpha((self.prize_gui_alpha * 40.0) as u8));

        if self.prize_gui_alpha > 0.4 {
            let mut cur_y = gui_y + 26;

            // 1. Header category badge
            let badge_w = (prize.category.len() as i32 * 8) + 32;
            let badge_x = gui_x + (gui_w - badge_w) / 2;
            gfx.fill_rounded_rect(badge_x, cur_y, badge_w, 24, 8, Color::new(32, 36, 48).with_alpha(alpha));
            gfx.draw_rounded_rect(badge_x, cur_y, badge_w, 24, 8, prize.accent_color.with_alpha(180));
            gfx.draw_text_shadow(badge_x + 16, cur_y + 8, prize.category, 1, Color::new(240, 245, 255), Color::new(0, 0, 0));

            cur_y += 42;

            // 2. Large celebratory title (Scale 2 with drop shadow)
            let title_w = prize.title.len() as i32 * 16;
            let title_x = gui_x + (gui_w - title_w) / 2;
            gfx.draw_text_shadow(title_x, cur_y, prize.title, 2, prize.accent_color, Color::new(0, 0, 0));

            cur_y += 36;

            // 3. Subtitle
            let sub_w = prize.subtitle.len() as i32 * 8;
            let sub_x = gui_x + (gui_w - sub_w) / 2;
            gfx.draw_text_shadow(sub_x, cur_y, prize.subtitle, 1, Color::new(180, 190, 205), Color::new(0, 0, 0));

            cur_y += 24;

            // 4. Decorative divider line with center diamond
            let line_margin = 60;
            let sep_x1 = gui_x + line_margin;
            let sep_x2 = gui_x + gui_w - line_margin;
            gfx.draw_line(sep_x1, cur_y, sep_x2, cur_y, Color::new(70, 78, 96).with_alpha(150));
            gfx.draw_text_shadow(gui_x + gui_w / 2 - 4, cur_y - 4, "*", 1, prize.accent_color, Color::new(0, 0, 0));

            cur_y += 24;

            // 5. Core message body lines
            for line in prize.lines {
                if line.is_empty() {
                    cur_y += 10;
                    continue;
                }
                let line_w = line.len() as i32 * 8;
                let line_x = gui_x + (gui_w - line_w) / 2;
                gfx.draw_text_shadow(line_x, cur_y, line, 1, Color::new(255, 255, 255), Color::new(0, 0, 0));
                cur_y += 22;
            }

            // 6. Special retro terminal box for Developer Origin Story
            if let Some(joke_lines) = prize.joke_box {
                cur_y += 12;
                let box_w = gui_w - 100;
                let box_h = 64;
                let box_x = gui_x + 50;

                gfx.fill_rounded_rect(box_x, cur_y, box_w, box_h, 8, Color::new(10, 14, 20).with_alpha(alpha));
                gfx.draw_rounded_rect(box_x, cur_y, box_w, box_h, 8, Color::new(0, 200, 230).with_alpha(160));

                let mut jy = cur_y + 14;
                for j_line in joke_lines {
                    let j_x = box_x + 20;
                    gfx.draw_text_shadow(j_x, jy, j_line, 1, Color::new(0, 230, 255), Color::new(0, 0, 0));
                    jy += 18;
                }
            }

            // 7. Pulsating Continue prompt pill
            if self.prize_gui_alpha > 0.85 {
                let cont_text = "[ ✦ Click anywhere or press Space to Continue ✦ ]";
                let btn_w = (cont_text.len() as i32 * 8) + 40;
                let btn_h = 34;
                let btn_x = gui_x + (gui_w - btn_w) / 2;
                let btn_y = gui_y + gui_h - 48;

                let pulse = ((self.state_start_time.elapsed().as_secs_f32() * 5.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let btn_border = Color::new((160.0 + 80.0 * pulse) as u8, (170.0 + 85.0 * pulse) as u8, 255);

                gfx.fill_rounded_rect(btn_x, btn_y, btn_w, btn_h, 10, Color::new(26, 30, 42).with_alpha(alpha));
                gfx.draw_rounded_rect(btn_x, btn_y, btn_w, btn_h, 10, btn_border);
                gfx.draw_text_shadow(btn_x + 20, btn_y + 12, cont_text, 1, Color::new(255, 255, 255), Color::new(0, 0, 0));
            }
        }
    }

    fn change_state(&mut self, new_state: CinematicState) {
        self.state = new_state;
        self.state_start_time = Instant::now();
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.state, CinematicState::Finished)
    }

    pub fn is_revealing_desktop(&self) -> bool {
        matches!(self.state, CinematicState::DesktopReveal)
    }

    pub fn ui_alpha(&self) -> f32 {
        if let CinematicState::DesktopReveal = self.state {
            let elapsed = self.state_start_time.elapsed().as_secs_f32();
            let progress = (elapsed / 2.5).min(1.0);
            if progress < 0.25 {
                0.0
            } else {
                ((progress - 0.25) / 0.75).min(1.0)
            }
        } else if matches!(self.state, CinematicState::Finished) {
            1.0
        } else {
            0.0
        }
    }

    fn draw_note_and_photo(&self, gfx: &mut Graphics, width: u32, height: u32, pitch: u32, alpha: f32) {
        let (current_note, current_photo, author_title) = match self.current_letter_idx {
            0 => (&self.handwritten_note, self.sister_photo.as_ref(), "✦ Letter from Your Sister ✦"),
            1 => (self.letter_amma.as_ref().unwrap_or(&self.handwritten_note), None, "✦ Letter from Amma ✦"),
            2 => (self.letter_appa.as_ref().unwrap_or(&self.handwritten_note), None, "✦ Letter from Appa ✦"),
            _ => (self.letter_friend.as_ref().unwrap_or(&self.handwritten_note), self.friend_photo.as_ref(), "✦ Letter from Medha ✦"),
        };

        // Draw top author badge pill with squircle geometry
        let badge_w = (author_title.len() as i32 * 8) + 40;
        let badge_h = 30;
        let badge_x = (width as i32 - badge_w) / 2;
        let badge_y = 12;
        gfx.draw_squircle_card(
            badge_x, badge_y, badge_w, badge_h, 6,
            Color::new(20, 18, 28).with_alpha((alpha * 230.0) as u8),
            Color::INCY_GOLD.with_alpha((alpha * 210.0) as u8),
        );
        gfx.draw_text_shadow(
            badge_x + 20, badge_y + 8, author_title, 1,
            Color::new(255, 240, 210).with_alpha((alpha * 255.0) as u8),
            Color::new(0, 0, 0),
        );

        if let Some(photo) = current_photo {
            // Side-by-side layout: Letter on left, Photo on right
            let max_h: u32 = 880.min(height.saturating_sub(130));
            let max_photo_w: u32 = 840.min((width as f32 * 0.45) as u32);

            let note_h: u32 = max_h;
            let note_w: u32 = ((current_note.width as f32) * (note_h as f32 / current_note.height as f32)) as u32;

            // Fit photo proportionally inside (max_photo_w, max_h) - works for both landscape & portrait
            let photo_scale = (max_photo_w as f32 / photo.width as f32).min(max_h as f32 / photo.height as f32);
            let photo_w = ((photo.width as f32) * photo_scale) as u32;
            let photo_h = ((photo.height as f32) * photo_scale) as u32;

            let gap: i32 = 44;
            let total_w = note_w as i32 + gap + photo_w as i32;
            let start_x = ((width as i32 - total_w) / 2).max(16);

            let note_x = start_x;
            let note_y = ((height as i32 - note_h as i32) / 2 + 6).max(badge_y + badge_h + 8);

            let photo_x = note_x + note_w as i32 + gap;
            let photo_y = ((height as i32 - photo_h as i32) / 2 + 6).max(badge_y + badge_h + 8);

            // Soft shadows behind both items
            let shadow_alpha = (alpha * 170.0) as u8;
            gfx.fill_rounded_rect(note_x + 8, note_y + 8, note_w as i32, note_h as i32, 10, Color::new(0, 0, 0).with_alpha(shadow_alpha));
            gfx.fill_rounded_rect(photo_x + 8, photo_y + 8, photo_w as i32, photo_h as i32, 12, Color::new(0, 0, 0).with_alpha(shadow_alpha));

            // Memory photo frame backing & subtle gold border
            gfx.fill_rounded_rect(photo_x - 4, photo_y - 4, photo_w as i32 + 8, photo_h as i32 + 8, 12, Color::new(245, 245, 250).with_alpha((alpha * 240.0) as u8));
            gfx.draw_rounded_rect(photo_x - 5, photo_y - 5, photo_w as i32 + 10, photo_h as i32 + 10, 13, Color::new(255, 215, 0).with_alpha((alpha * 210.0) as u8));

            {
                let buf = gfx.backbuffer_mut();
                current_note.draw_to_buffer_scaled(buf, width, height, pitch,
                                                   note_x, note_y, note_w, note_h, alpha);
                photo.draw_to_buffer_scaled(buf, width, height, pitch,
                                            photo_x, photo_y, photo_w, photo_h, alpha);
            }
        } else {
            // Clean centered layout for Mom's and Dad's letters (NO photo)
            let note_h: u32 = 910.min(height.saturating_sub(130));
            let note_w: u32 = ((current_note.width as f32) * (note_h as f32 / current_note.height as f32)) as u32;
            let note_x = (width as i32 - note_w as i32) / 2;
            let note_y = ((height as i32 - note_h as i32) / 2 + 6).max(badge_y + badge_h + 8);

            let shadow_alpha = (alpha * 160.0) as u8;
            gfx.fill_rounded_rect(note_x + 10, note_y + 10, note_w as i32, note_h as i32, 10, Color::new(0, 0, 0).with_alpha(shadow_alpha));

            {
                let buf = gfx.backbuffer_mut();
                current_note.draw_to_buffer_scaled(buf, width, height, pitch,
                                                   note_x, note_y, note_w, note_h, alpha);
            }
        }
    }

    fn ease_out_cubic(t: f32) -> f32 {
        let t = t - 1.0;
        t * t * t + 1.0
    }

    #[allow(dead_code)]
    fn ease_in_cubic(t: f32) -> f32 {
        t * t * t
    }

    fn ease_in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = t - 1.0;
            1.0 + 4.0 * t * t * t
        }
    }

    fn ease_in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            let t = t - 1.0;
            1.0 - 2.0 * t * t
        }
    }

    fn draw_milo_speech_bubble(&self, gfx: &mut Graphics, text: &str, scale_x: f32, scale_y: f32, alpha: f32) {
        let (clean_text, is_tiramisu, is_presents) = if text.contains("tiramisu") {
            ("And obviously... tiramisu.", true, false)
        } else {
            ("Okay... now for the presents!", false, true)
        };

        let bx = (50.0 * scale_x) as i32;
        let by = (290.0 * scale_y) as i32;
        let bw = (clean_text.len() as i32 * 8 + 66).max(340);
        let bh = 56;

        let a_u8 = (alpha * 255.0) as u8;
        // Drop shadow
        gfx.fill_rounded_rect(bx + 4, by + 4, bw, bh, 14, Color::new(0, 0, 0).with_alpha((alpha * 100.0) as u8));
        // Glass/white bubble
        gfx.fill_rounded_rect(bx, by, bw, bh, 14, Color::new(255, 255, 255).with_alpha(a_u8));
        gfx.draw_rounded_rect(bx, by, bw, bh, 14, Color::new(20, 24, 32).with_alpha(a_u8));
        gfx.draw_rounded_rect(bx + 1, by + 1, bw - 2, bh - 2, 13, Color::new(20, 24, 32).with_alpha(a_u8));

        // Speech bubble pointer tail pointing down towards Milo
        let tail_x = bx + 50;
        let tail_y = by + bh;
        for dy in 0..14 {
            let w = 16 - dy;
            let x = tail_x - (dy / 2);
            gfx.fill_rect(x, tail_y + dy - 1, w, 1, Color::new(255, 255, 255).with_alpha(a_u8));
        }
        gfx.draw_line(tail_x, tail_y, tail_x - 7, tail_y + 13, Color::new(20, 24, 32).with_alpha(a_u8));
        gfx.draw_line(tail_x + 16, tail_y, tail_x - 7, tail_y + 13, Color::new(20, 24, 32).with_alpha(a_u8));

        // Text
        let tx = bx + 22;
        let ty = by + 24;
        gfx.draw_text_scaled(tx, ty, clean_text, 1, Color::new(20, 24, 32).with_alpha(a_u8));

        let icon_x = tx + (clean_text.len() as i32 * 8) + 8;
        let icon_y = ty - 4;

        if is_tiramisu {
            // Cute cake slice icon
            gfx.fill_rect(icon_x, icon_y + 6, 16, 8, Color::new(210, 160, 110).with_alpha(a_u8)); // cake sponge
            gfx.fill_rect(icon_x, icon_y + 4, 16, 3, Color::new(255, 240, 210).with_alpha(a_u8)); // mascarpone cream
            gfx.fill_rect(icon_x, icon_y + 2, 16, 2, Color::new(130, 80, 50).with_alpha(a_u8));   // cocoa powder
            gfx.fill_circle(icon_x + 8, icon_y, 3, Color::new(230, 40, 60).with_alpha(a_u8));     // cherry
        } else if is_presents {
            // Cute gift box icon
            gfx.fill_rounded_rect(icon_x, icon_y + 3, 16, 14, 2, Color::new(230, 50, 60).with_alpha(a_u8)); // red box
            gfx.fill_rect(icon_x + 7, icon_y + 3, 2, 14, Color::new(255, 215, 0).with_alpha(a_u8));        // yellow ribbon v
            gfx.fill_rect(icon_x, icon_y + 9, 16, 2, Color::new(255, 215, 0).with_alpha(a_u8));            // yellow ribbon h
            gfx.fill_circle(icon_x + 8, icon_y + 2, 3, Color::new(255, 215, 0).with_alpha(a_u8));          // bow
        }
    }

    fn spawn_smoke_particles(&mut self) {
        self.smoke_particles.clear();
        let wicks = [
            (964.0, 594.0),
            (988.0, 576.0),
            (1012.0, 579.0),
            (1036.0, 600.0),
        ];

        for &(wx, wy) in &wicks {
            for _ in 0..7 {
                let vx = self.rng.range_f32(-12.0, 12.0);
                let vy = self.rng.range_f32(30.0, 75.0);
                let max_life = self.rng.range_f32(2.0, 3.5);
                let size = self.rng.range_f32(4.0, 7.0);
                self.smoke_particles.push(SmokeParticle {
                    x: wx + self.rng.range_f32(-4.0, 4.0),
                    y: wy + self.rng.range_f32(-2.0, 2.0),
                    vx,
                    vy,
                    life: self.rng.range_f32(0.0, 0.4),
                    max_life,
                    size,
                    alpha: self.rng.range_f32(0.6, 0.85),
                });
            }
        }
    }

    fn spawn_confetti_particles(&mut self) {
        self.confetti_particles.clear();
        let palette = [
            Color::new(255, 215, 0),   // Gold
            Color::new(255, 90, 150),  // Bright Pink
            Color::new(0, 220, 255),   // Cyan
            Color::new(170, 100, 255), // Violet
            Color::new(80, 235, 120),  // Emerald
            Color::new(255, 140, 40),  // Orange
            Color::new(255, 255, 255), // Pure White
        ];

        let cx = 1000.0;
        let cy = 580.0;

        for _ in 0..90 {
            let vx = self.rng.range_f32(-380.0, 380.0);
            let vy = self.rng.range_f32(-550.0, -180.0);
            let size = self.rng.range_f32(6.0, 12.0);
            let color_idx = (self.rng.next_u32() as usize) % palette.len();
            let rot_speed = self.rng.range_f32(-180.0, 180.0);
            self.confetti_particles.push(ConfettiParticle {
                x: cx + self.rng.range_f32(-40.0, 40.0),
                y: cy + self.rng.range_f32(-30.0, 10.0),
                vx,
                vy,
                size,
                color: palette[color_idx],
                rotation: self.rng.range_f32(0.0, 360.0),
                rot_speed,
            });
        }
    }

    fn update_particles(&mut self, dt: f32) {
        // Smoke
        for p in &mut self.smoke_particles {
            p.life += dt;
            p.y -= p.vy * dt;
            p.x += (p.vx + (p.life * 3.5).sin() * 12.0) * dt;
            p.size += 5.0 * dt;
            let rem = (1.0 - p.life / p.max_life).max(0.0);
            p.alpha = rem * 0.8;
        }
        self.smoke_particles.retain(|p| p.life < p.max_life);

        // Confetti
        for p in &mut self.confetti_particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 460.0 * dt;
            p.vx *= (1.0 - 0.4 * dt).max(0.0);
            p.rotation += p.rot_speed * dt;
        }
        self.confetti_particles.retain(|p| p.y < 1120.0);
    }

    fn play_birthday_music(&mut self) {
        self.stop_birthday_music();

        let wav_candidates = [
            self.assets_dir.join("happy_birthday.wav"),
            PathBuf::from("/usr/share/cinematic-scene-assets/happy_birthday.wav"),
            PathBuf::from("/usr/share/audio/happy_birthday.wav"),
            PathBuf::from("cinematic-scene-assets/happy_birthday.wav"),
            PathBuf::from("../cinematic-scene-assets/happy_birthday.wav"),
        ];

        let mut wav_path = None;
        for path in &wav_candidates {
            if path.exists() {
                wav_path = Some(path.clone());
                break;
            }
        }

        if let Some(path) = wav_path {
            let players = ["/usr/bin/aplay", "/bin/aplay", "aplay", "paplay"];
            for player in &players {
                if let Ok(child) = std::process::Command::new(player)
                    .arg("-q")
                    .arg(&path)
                    .spawn()
                {
                    println!("leviathan-ui: Birthday music playback started via {}", player);
                    self.music_child = Some(child);
                    return;
                }
            }
        }
    }

    fn stop_birthday_music(&mut self) {
        if let Some(mut child) = self.music_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn play_sound_effect(&self, filename: &str) {
        let wav_candidates = [
            self.assets_dir.join(filename),
            PathBuf::from("/usr/share/cinematic-scene-assets").join(filename),
            PathBuf::from("/usr/share/audio").join(filename),
            PathBuf::from("cinematic-scene-assets").join(filename),
            PathBuf::from("../cinematic-scene-assets").join(filename),
        ];

        let mut wav_path = None;
        for path in &wav_candidates {
            if path.exists() {
                wav_path = Some(path.clone());
                break;
            }
        }

        if let Some(path) = wav_path {
            let players = ["/usr/bin/aplay", "/bin/aplay", "aplay", "paplay"];
            for player in &players {
                if let Ok(_) = std::process::Command::new(player)
                    .arg("-q")
                    .arg(&path)
                    .spawn()
                {
                    return;
                }
            }
        }
    }
}

impl Drop for BirthdayCinematic {
    fn drop(&mut self) {
        self.stop_birthday_music();
    }
}
