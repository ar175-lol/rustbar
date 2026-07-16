use crate::config::{BATTERY_BG_GREEN, BATTERY_TEXT_DARK, CONTAINER_RADIUS, ITEM_PADDING};

pub struct BatteryModule {
    pub text: String,
}

impl BatteryModule {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn draw(&self, cr: &cairo::Context, bar_height: f64, end_x: f64) -> f64 {
        if self.text.is_empty() {
            return end_x;
        }

        let extents = cr.text_extents(&self.text).unwrap();
        let pad_x = 12.0;
        let bg_w = extents.x_advance() + (pad_x * 2.0);
        let bg_h = 22.0;
        let bg_y = (bar_height - bg_h) / 2.0;
        let bg_x = end_x - bg_w;

        cr.new_sub_path();
        cr.arc(
            bg_x + CONTAINER_RADIUS,
            bg_y + CONTAINER_RADIUS,
            CONTAINER_RADIUS,
            180.0f64.to_radians(),
            270.0f64.to_radians(),
        );
        cr.arc(
            bg_x + bg_w - CONTAINER_RADIUS,
            bg_y + CONTAINER_RADIUS,
            CONTAINER_RADIUS,
            270.0f64.to_radians(),
            360.0f64.to_radians(),
        );
        cr.arc(
            bg_x + bg_w - CONTAINER_RADIUS,
            bg_y + bg_h - CONTAINER_RADIUS,
            CONTAINER_RADIUS,
            0.0f64.to_radians(),
            90.0f64.to_radians(),
        );
        cr.arc(
            bg_x + CONTAINER_RADIUS,
            bg_y + bg_h - CONTAINER_RADIUS,
            CONTAINER_RADIUS,
            90.0f64.to_radians(),
            180.0f64.to_radians(),
        );
        cr.close_path();

        cr.set_source_rgb(BATTERY_BG_GREEN.r, BATTERY_BG_GREEN.g, BATTERY_BG_GREEN.b);
        cr.fill().unwrap();

        let text_y = bg_y + (bg_h - extents.height()) / 2.0 - extents.y_bearing();

        cr.set_source_rgb(
            BATTERY_TEXT_DARK.r,
            BATTERY_TEXT_DARK.g,
            BATTERY_TEXT_DARK.b,
        );
        cr.move_to(bg_x + pad_x, text_y);
        cr.show_text(&self.text).unwrap();

        bg_x - ITEM_PADDING
    }
}
