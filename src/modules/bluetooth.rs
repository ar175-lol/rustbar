use crate::config;

pub struct BluetoothModule {
    pub text: String,
    pub is_on: bool,
}

impl BluetoothModule {
    pub fn new() -> Self {
        Self {
            text: "󰂲".to_string(),
            is_on: false,
        }
    }

    pub fn draw(&self, cr: &cairo::Context, bar_height: f64, end_x: f64) -> f64 {
        let extents = cr.text_extents(&self.text).unwrap();
        let pad_x = 12.0;
        let bg_w = extents.x_advance() + (pad_x * 2.0);
        let bg_h = 22.0;
        let bg_y = (bar_height - bg_h) / 2.0;
        let bg_x = end_x - bg_w;

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

        if self.is_on {
            cr.set_source_rgb(
                config::BLUETOOTH_BG_SAPPHIRE.r,
                config::BLUETOOTH_BG_SAPPHIRE.g,
                config::BLUETOOTH_BG_SAPPHIRE.b,
            );
            cr.fill().unwrap();
            cr.set_source_rgb(
                config::BATTERY_TEXT_DARK.r,
                config::BATTERY_TEXT_DARK.g,
                config::BATTERY_TEXT_DARK.b,
            );
        } else {
            cr.set_source_rgb(
                config::MODULE_BG_DARK.r,
                config::MODULE_BG_DARK.g,
                config::MODULE_BG_DARK.b,
            );
            cr.fill().unwrap();
            cr.set_source_rgb(
                config::INACTIVE_WORKSPACE_COLOR.r,
                config::INACTIVE_WORKSPACE_COLOR.g,
                config::INACTIVE_WORKSPACE_COLOR.b,
            );
        }

        let text_y = bg_y + (bg_h - extents.height()) / 2.0 - extents.y_bearing();
        cr.move_to(bg_x + pad_x, text_y);
        cr.show_text(&self.text).unwrap();

        bg_x - config::ITEM_PADDING
    }
}
