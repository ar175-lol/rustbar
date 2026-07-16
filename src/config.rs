pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

pub const BAR_WIDTH: u32 = 1920;
pub const BAR_HEIGHT: u32 = 30;

pub const FONT_NAME: &str = "JetBrainsMono Nerd Font";
pub const FONT_SIZE: f64 = 14.0;
pub const FONT_WEIGHT: cairo::FontWeight = cairo::FontWeight::Bold;

pub const ITEM_PADDING: f64 = 15.0;
pub const LINE_WIDTH: f64 = 2.0;

pub const ICON_MUTED: &str = "󰖁";
pub const ICONS: [&str; 3] = ["󰕿", "󰖀", "󰕾"];

pub const SCROLL_STEP: u32 = 3;

pub const SINK: &str = "@DEFAULT_AUDIO_SINK@";

pub const BACKGROUND_COLOR: Color = Color {
    r: 0.14,
    g: 0.15,
    b: 0.23,
};
pub const TEXT_COLOR: Color = Color {
    r: 0.79,
    g: 0.83,
    b: 0.96,
};
pub const ACTIVE_WORKSPACE_COLOR: Color = Color {
    r: 0.72,
    g: 0.74,
    b: 0.99,
};
pub const INACTIVE_WORKSPACE_COLOR: Color = Color {
    r: 0.36,
    g: 0.39,
    b: 0.48,
};

pub const MODULE_BG_DARK: Color = Color {
    r: 0.18,
    g: 0.19,
    b: 0.28,
};
pub const BATTERY_BG_GREEN: Color = Color {
    r: 0.65,
    g: 0.87,
    b: 0.63,
};
pub const BATTERY_TEXT_DARK: Color = Color {
    r: 0.11,
    g: 0.12,
    b: 0.18,
};

pub const BLUETOOTH_BG_SAPPHIRE: Color = Color {
    r: 0.49,
    g: 0.69,
    b: 0.93,
};

pub const AUDIO_BLUE: Color = Color {
    r: 0.537,
    g: 0.706,
    b: 0.980,
};

pub const CONTAINER_RADIUS: f64 = 6.0;

pub fn validate_config() -> Result<(), String> {
    if BAR_WIDTH == 0 {
        return Err("BAR_WIDTH cannot be zero!".into());
    }
    if BAR_HEIGHT == 0 {
        return Err("BAR_HEIGHT cannot be zero!".into());
    }
    if FONT_SIZE <= 0.0 {
        return Err("FONT_SIZE must be > 0".into());
    }
    if CONTAINER_RADIUS < 0.0 {
        return Err("CONTAINER_RADIUS cannot be negative".into());
    }
    if LINE_WIDTH < 0.0 {
        return Err("LINE_WIDTH cannot be negative".into());
    }
    Ok(())
}
