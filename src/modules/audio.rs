use crate::config::{self, ICON_MUTED, ICONS, SCROLL_STEP, SINK};

pub struct AudioState {
    pub volume: u32,
    pub is_muted: bool,
}

pub fn query_volume() -> Option<AudioState> {
    let output = std::process::Command::new("wpctl")
        .args(["get-volume", SINK])
        .output()
        .ok()?;
    parse_wpctl_output(&String::from_utf8_lossy(&output.stdout))
}

fn parse_wpctl_output(stdout: &str) -> Option<AudioState> {
    let rest = stdout.trim().strip_prefix("Volume:")?.trim();
    let is_muted = rest.contains("[MUTED]");
    let fraction: f64 = rest.split_whitespace().next()?.parse().ok()?;
    Some(AudioState {
        volume: (fraction * 100.0).round() as u32,
        is_muted,
    })
}

pub fn toggle_mute(tx: calloop::channel::Sender<crate::AppEvent>) {
    std::thread::spawn(move || {
        let _ = std::process::Command::new("wpctl")
            .args(["set-mute", SINK, "toggle"])
            .status();
        if let Some(state) = query_volume() {
            let _ = tx.send(crate::AppEvent::Audio(state));
        }
    });
}

pub fn step_volume(tx: calloop::channel::Sender<crate::AppEvent>, increase: bool) {
    std::thread::spawn(move || {
        let step_arg = format!("{}%{}", SCROLL_STEP, if increase { "+" } else { "-" });
        let _ = std::process::Command::new("wpctl")
            .args(["set-volume", SINK, &step_arg])
            .status();
        if let Some(state) = query_volume() {
            let _ = tx.send(crate::AppEvent::Audio(state));
        }
    });
}

pub struct AudioModule {
    pub volume: u32,
    pub is_muted: bool,
    pub is_hovered: bool,
    bbox: Option<(f64, f64, f64, f64)>,
}

impl AudioModule {
    pub fn new() -> Self {
        Self {
            volume: 0,
            is_muted: false,
            is_hovered: false,
            bbox: None,
        }
    }

    pub fn apply(&mut self, state: AudioState) {
        self.volume = state.volume;
        self.is_muted = state.is_muted;
    }

    fn icon(&self) -> &'static str {
        if self.is_muted {
            return ICON_MUTED;
        }
        let idx = ((self.volume as f64 / 100.0) * ICONS.len() as f64)
            .floor()
            .clamp(0.0, (ICONS.len() - 1) as f64) as usize;
        ICONS[idx]
    }

    fn label(&self) -> String {
        format!("{} {}%", self.icon(), self.volume)
    }

    pub fn hit_test(&self, x: f64, y: f64) -> bool {
        match self.bbox {
            Some((bx, by, bw, bh)) => x >= bx && x <= bx + bw && y >= by && y <= by + bh,
            None => false,
        }
    }

    pub fn draw(&mut self, cr: &cairo::Context, bar_height: f64, end_x: f64) -> f64 {
        let text = self.label();
        let extents = cr.text_extents(&text).unwrap();
        let pad_x = 12.0;
        let bg_w = extents.x_advance() + (pad_x * 2.0);
        let bg_h = 22.0;
        let bg_y = (bar_height - bg_h) / 2.0;
        let bg_x = end_x - bg_w;

        self.bbox = Some((bg_x, bg_y, bg_w, bg_h));

        if self.is_hovered {
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
                config::AUDIO_BLUE.r,
                config::AUDIO_BLUE.g,
                config::AUDIO_BLUE.b,
            );
            cr.fill_preserve().unwrap();

            cr.set_source_rgb(
                config::AUDIO_BLUE.r,
                config::AUDIO_BLUE.g,
                config::AUDIO_BLUE.b,
            );
            cr.set_line_width(1.0);
            cr.stroke().unwrap();

            // color: @base; (reusing BACKGROUND_COLOR -- see config.rs)
            cr.set_source_rgb(
                config::BACKGROUND_COLOR.r,
                config::BACKGROUND_COLOR.g,
                config::BACKGROUND_COLOR.b,
            );
        } else {
            // color: @blue;
            cr.set_source_rgb(
                config::AUDIO_BLUE.r,
                config::AUDIO_BLUE.g,
                config::AUDIO_BLUE.b,
            );
        }

        let text_y = bg_y + (bg_h - extents.height()) / 2.0 - extents.y_bearing();
        cr.move_to(bg_x + pad_x, text_y);
        cr.show_text(&text).unwrap();

        bg_x - config::ITEM_PADDING
    }
}
