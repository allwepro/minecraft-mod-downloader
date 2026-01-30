use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Context, IconData, Id, Ui, vec2};
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

pub fn get_app_icon() -> IconData {
    let icon_path = "assets/icon.png";
    let image = image::open(icon_path).unwrap_or_else(|_| generate_fallback_icon(128));
    let image_rgba = image.to_rgba8();
    let (width, height) = image_rgba.dimensions();

    IconData {
        rgba: image_rgba.into_raw(),
        width,
        height,
    }
}

pub fn show_app_icon(ui: &mut Ui, size: f32) {
    let handle = get_texture_icon(ui.ctx());
    let size_vec = vec2(size, size);
    ui.image((handle.id(), size_vec));
}

pub fn get_texture_icon(ctx: &Context) -> TextureHandle {
    let texture_name = "app_icon";

    if let Some(handle) = ctx.data(|d| d.get_temp::<TextureHandle>(Id::from(texture_name))) {
        return handle;
    }
    let icon_data = get_app_icon();
    let size = [icon_data.width as usize, icon_data.height as usize];
    let color_image = ColorImage::from_rgba_unmultiplied(size, &icon_data.rgba);

    let handle = ctx.load_texture(texture_name, color_image, TextureOptions::LINEAR);

    ctx.data_mut(|d| d.insert_temp(Id::from(texture_name), handle.clone()));

    handle
}

fn generate_fallback_icon(size: u32) -> DynamicImage {
    let mut img: RgbaImage = ImageBuffer::new(size, size);

    let bg = Rgba([40, 40, 45, 255]);
    let fg = Rgba([252, 203, 143, 255]);
    let shadow = Rgba([120, 70, 40, 255]);

    for pixel in img.pixels_mut() {
        *pixel = bg;
    }

    let size_f = size as f32;
    let center = size_f / 2.0;

    let top_padding = size_f * 0.2;
    let bottom_padding = size_f * 0.15;

    let head_tip_y = size_f - bottom_padding;
    let head_height = size_f * 0.35;
    let neck_y = head_tip_y - head_height;

    let shaft_top_y = top_padding;
    let shaft_bottom_y = neck_y + 5.0;

    let shaft_half_width = size_f * 0.08;
    let head_half_width = size_f * 0.3;

    let shadow_offset = (size_f / 40.0).max(1.0);

    let mut draw_smooth_shape = |offset_x: f32, offset_y: f32, color: Rgba<u8>| {
        for y in 0..size {
            for x in 0..size {
                let mut hits = 0;

                for sy in 0..2 {
                    for sx in 0..2 {
                        let px = (x * 2 + sx) as f32 + 0.5;
                        let py = (y * 2 + sy) as f32 + 0.5;

                        let dx = px - center + offset_x;
                        let dy = py + offset_y;

                        let is_inside = if dy >= shaft_top_y && dy < shaft_bottom_y {
                            dx.abs() < shaft_half_width
                        } else if dy >= shaft_bottom_y && dy < head_tip_y {
                            let progress = (dy - shaft_bottom_y) / head_height;
                            let current_half_width = head_half_width * (1.0 - progress);
                            dx.abs() < current_half_width
                        } else {
                            false
                        };

                        if is_inside {
                            hits += 1;
                        }
                    }
                }

                if hits > 0 {
                    let blend = hits as f32 / 4.0;
                    let current = img.get_pixel(x, y);
                    let r = (color[0] as f32 * blend + current[0] as f32 * (1.0 - blend)) as u8;
                    let g = (color[1] as f32 * blend + current[1] as f32 * (1.0 - blend)) as u8;
                    let b = (color[2] as f32 * blend + current[2] as f32 * (1.0 - blend)) as u8;

                    if color[3] == 255 {
                        img.put_pixel(x, y, Rgba([r, g, b, 255]));
                    } else {
                        img.put_pixel(x, y, Rgba([r, g, b, color[3]]));
                    }
                }
            }
        }
    };

    draw_smooth_shape(shadow_offset, shadow_offset, shadow);
    draw_smooth_shape(0.0, 0.0, fg);

    let radius = size_f * 0.1875;

    for y in 0..size {
        for x in 0..size {
            let is_corner = !(x >= radius as u32 && x < size - radius as u32
                || y >= radius as u32 && y < size - radius as u32);

            if is_corner {
                let dx = if x < radius as u32 {
                    radius - x as f32 - 0.5
                } else {
                    x as f32 + 0.5 - (size_f - radius)
                };

                let dy = if y < radius as u32 {
                    radius - y as f32 - 0.5
                } else {
                    y as f32 + 0.5 - (size_f - radius)
                };

                if (dx * dx + dy * dy) > radius * radius {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
    }

    DynamicImage::ImageRgba8(img)
}
