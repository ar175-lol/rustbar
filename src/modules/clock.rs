use crate::config;
use chrono::Timelike;

pub struct ClockModule {
    pub last_minute: u32,
}

impl ClockModule {
    pub fn new() -> Self {
        Self {
            last_minute: chrono::Local::now().minute(),
        }
    }

    pub fn update(&mut self) -> bool {
        let current_minute = chrono::Local::now().minute();
        if current_minute != self.last_minute {
            self.last_minute = current_minute;
            true
        } else {
            false
        }
    }

    pub fn draw(&self, cr: &cairo::Context, bar_height: f64) {
        let time_str = chrono::Local::now().format("%H:%M").to_string();
        let extents = cr.text_extents(&time_str).unwrap();

        let pad_x = 12.0;
        let bg_w = extents.x_advance() + (pad_x * 2.0);
        let bg_h = 22.0;
        let bg_y = (bar_height - bg_h) / 2.0;
        let bg_x = config::ITEM_PADDING;

        cr.new_sub_path();
        cr.arc(
            bg_x + config::CONTAINER_RADIUS,
            bg_y + config::CONTAINER_RADIUS,
            config::CONTAINER_RADIUS,
            180.0f64.to_radians(),
            270.0f64.to_radians(),
        );
        cr.arc(
            bg_x + bg_w - config::CONTAINER_RADIUS,
            bg_y + config::CONTAINER_RADIUS,
            config::CONTAINER_RADIUS,
            270.0f64.to_radians(),
            360.0f64.to_radians(),
        );
        cr.arc(
            bg_x + bg_w - config::CONTAINER_RADIUS,
            bg_y + bg_h - config::CONTAINER_RADIUS,
            config::CONTAINER_RADIUS,
            0.0f64.to_radians(),
            90.0f64.to_radians(),
        );
        cr.arc(
            bg_x + config::CONTAINER_RADIUS,
            bg_y + bg_h - config::CONTAINER_RADIUS,
            config::CONTAINER_RADIUS,
            90.0f64.to_radians(),
            180.0f64.to_radians(),
        );
        cr.close_path();

        cr.set_source_rgb(
            config::MODULE_BG_DARK.r,
            config::MODULE_BG_DARK.g,
            config::MODULE_BG_DARK.b,
        );
        cr.fill().unwrap();

        let text_y = bg_y + (bg_h - extents.height()) / 2.0 - extents.y_bearing();

        cr.set_source_rgb(
            config::TEXT_COLOR.r,
            config::TEXT_COLOR.g,
            config::TEXT_COLOR.b,
        );
        cr.move_to(bg_x + pad_x, text_y);
        cr.show_text(&time_str).unwrap();
    }
}
