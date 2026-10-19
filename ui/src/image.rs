use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA format
}

impl Image {
    pub fn load_png<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(&path).map_err(|e| format!("Failed to open {}: {}", path.as_ref().display(), e))?;
        let reader = BufReader::new(file);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().map_err(|e| format!("PNG decode error: {}", e))?;

        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG frame error: {}", e))?;

        let width = info.width;
        let height = info.height;

        // Convert to RGBA if needed
        let data = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in buf[..info.buffer_size()].chunks(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            }
            png::ColorType::Grayscale => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for &gray in &buf[..info.buffer_size()] {
                    rgba.push(gray);
                    rgba.push(gray);
                    rgba.push(gray);
                    rgba.push(255);
                }
                rgba
            }
            png::ColorType::GrayscaleAlpha => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in buf[..info.buffer_size()].chunks(2) {
                    let gray = chunk[0];
                    let alpha = chunk[1];
                    rgba.push(gray);
                    rgba.push(gray);
                    rgba.push(gray);
                    rgba.push(alpha);
                }
                rgba
            }
            _ => return Err("Unsupported PNG color type".to_string()),
        };

        Ok(Image { width, height, data })
    }

    /// Blend this image onto a target buffer at (x, y) with alpha blending
    pub fn draw_to_buffer(&self, target: &mut [u8], target_width: u32, target_height: u32,
                          target_pitch: u32, x: i32, y: i32, alpha: f32) {
        let bpp = 4; // Assuming 32-bit BGRA framebuffer

        for src_y in 0..self.height {
            let dst_y = y + src_y as i32;
            if dst_y < 0 || dst_y >= target_height as i32 {
                continue;
            }

            let src_row_offset = (src_y * self.width * 4) as usize;
            let dst_row_offset = (dst_y as usize) * (target_pitch as usize);

            for src_x in 0..self.width {
                let dst_x = x + src_x as i32;
                if dst_x < 0 || dst_x >= target_width as i32 {
                    continue;
                }

                let src_idx = src_row_offset + (src_x * 4) as usize;
                let dst_idx = dst_row_offset + (dst_x as usize) * bpp;

                if src_idx + 3 < self.data.len() && dst_idx + 3 < target.len() {
                    let src_r = self.data[src_idx];
                    let src_g = self.data[src_idx + 1];
                    let src_b = self.data[src_idx + 2];
                    let src_a = (self.data[src_idx + 3] as f32 / 255.0) * alpha;

                    if src_a > 0.0 {
                        let dst_b = target[dst_idx];
                        let dst_g = target[dst_idx + 1];
                        let dst_r = target[dst_idx + 2];

                        // Alpha blending
                        let inv_alpha = 1.0 - src_a;
                        target[dst_idx] = ((src_b as f32 * src_a) + (dst_b as f32 * inv_alpha)) as u8;
                        target[dst_idx + 1] = ((src_g as f32 * src_a) + (dst_g as f32 * inv_alpha)) as u8;
                        target[dst_idx + 2] = ((src_r as f32 * src_a) + (dst_r as f32 * inv_alpha)) as u8;
                    }
                }
            }
        }
    }

    /// Blend this image onto a target buffer at (dst_x, dst_y) with dimensions (dst_w, dst_h) and alpha
    pub fn draw_to_buffer_scaled(&self, target: &mut [u8], target_width: u32, target_height: u32,
                                target_pitch: u32, dst_x: i32, dst_y: i32,
                                dst_w: u32, dst_h: u32, alpha: f32) {
        if dst_w == 0 || dst_h == 0 || alpha <= 0.0 || self.width == 0 || self.height == 0 {
            return;
        }
        let bpp = 4; // Assuming 32-bit BGRA framebuffer

        for dy in 0..dst_h {
            let py = dst_y + dy as i32;
            if py < 0 || py >= target_height as i32 {
                continue;
            }

            let sy = ((dy as u64 * self.height as u64) / dst_h as u64) as u32;
            let src_row_offset = (sy * self.width * 4) as usize;
            let dst_row_offset = (py as usize) * (target_pitch as usize);

            for dx in 0..dst_w {
                let px = dst_x + dx as i32;
                if px < 0 || px >= target_width as i32 {
                    continue;
                }

                let sx = ((dx as u64 * self.width as u64) / dst_w as u64) as u32;
                let src_idx = src_row_offset + (sx * 4) as usize;
                let dst_idx = dst_row_offset + (px as usize) * bpp;

                if src_idx + 3 < self.data.len() && dst_idx + 3 < target.len() {
                    let src_r = self.data[src_idx];
                    let src_g = self.data[src_idx + 1];
                    let src_b = self.data[src_idx + 2];
                    let src_a = (self.data[src_idx + 3] as f32 / 255.0) * alpha;

                    if src_a > 0.0 {
                        let dst_b = target[dst_idx];
                        let dst_g = target[dst_idx + 1];
                        let dst_r = target[dst_idx + 2];

                        let inv_alpha = 1.0 - src_a;
                        target[dst_idx] = ((src_b as f32 * src_a) + (dst_b as f32 * inv_alpha)) as u8;
                        target[dst_idx + 1] = ((src_g as f32 * src_a) + (dst_g as f32 * inv_alpha)) as u8;
                        target[dst_idx + 2] = ((src_r as f32 * src_a) + (dst_r as f32 * inv_alpha)) as u8;
                    }
                }
            }
        }
    }

    /// Blend this image onto a target buffer rotated around center (center_x, center_y)
    pub fn draw_to_buffer_rotated_scaled(&self, target: &mut [u8], target_width: u32, target_height: u32,
                                         target_pitch: u32, center_x: i32, center_y: i32,
                                         scale_x: f32, scale_y: f32, angle_rad: f32, alpha: f32) {
        if alpha <= 0.0 || self.width == 0 || self.height == 0 || scale_x <= 0.0 || scale_y <= 0.0 {
            return;
        }
        let bpp = 4;
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let half_w = (self.width as f32 * scale_x * 0.5) as f32;
        let half_h = (self.height as f32 * scale_y * 0.5) as f32;
        let radius = (half_w * half_w + half_h * half_h).sqrt();

        let min_x = ((center_x as f32 - radius).max(0.0)) as i32;
        let max_x = ((center_x as f32 + radius).min(target_width as f32 - 1.0)) as i32;
        let min_y = ((center_y as f32 - radius).max(0.0)) as i32;
        let max_y = ((center_y as f32 + radius).min(target_height as f32 - 1.0)) as i32;

        let src_cx = self.width as f32 * 0.5;
        let src_cy = self.height as f32 * 0.5;

        for py in min_y..=max_y {
            let dy = py - center_y;
            let dst_row_offset = (py as usize) * (target_pitch as usize);

            for px in min_x..=max_x {
                let dx = px - center_x;

                // Inverse rotation & scale
                let rx = dx as f32 * cos_a + dy as f32 * sin_a;
                let ry = -dx as f32 * sin_a + dy as f32 * cos_a;

                let sx = rx / scale_x + src_cx;
                let sy = ry / scale_y + src_cy;

                if sx >= 0.0 && sx < (self.width as f32 - 1.0) && sy >= 0.0 && sy < (self.height as f32 - 1.0) {
                    let isx = sx as u32;
                    let isy = sy as u32;
                    let src_idx = ((isy * self.width + isx) * 4) as usize;
                    let dst_idx = dst_row_offset + (px as usize) * bpp;

                    if src_idx + 3 < self.data.len() && dst_idx + 3 < target.len() {
                        let src_a = (self.data[src_idx + 3] as f32 / 255.0) * alpha;
                        if src_a > 0.01 {
                            let src_r = self.data[src_idx];
                            let src_g = self.data[src_idx + 1];
                            let src_b = self.data[src_idx + 2];

                            let dst_b = target[dst_idx];
                            let dst_g = target[dst_idx + 1];
                            let dst_r = target[dst_idx + 2];

                            let inv_alpha = 1.0 - src_a;
                            target[dst_idx] = ((src_b as f32 * src_a) + (dst_b as f32 * inv_alpha)) as u8;
                            target[dst_idx + 1] = ((src_g as f32 * src_a) + (dst_g as f32 * inv_alpha)) as u8;
                            target[dst_idx + 2] = ((src_r as f32 * src_a) + (dst_r as f32 * inv_alpha)) as u8;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_logo_asset() {
        let candidate_paths = [
            Path::new("assets/logo.png"),
            Path::new("../assets/logo.png"),
            Path::new("/usr/share/leviathan/assets/logo.png"),
            Path::new("/etc/leviathan/assets/logo.png"),
        ];

        let found = candidate_paths.iter().find(|p| p.exists());
        if let Some(p) = found {
            let img = Image::load_png(p);
            assert!(img.is_ok(), "Failed to load logo: {:?}", img.err());
            let img = img.unwrap();
            assert!(img.width > 0 && img.height > 0);
        }
    }
}
