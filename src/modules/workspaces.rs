use crate::config;

pub struct WorkspacesModule {
    pub workspaces: Vec<niri_ipc::Workspace>,
}

impl WorkspacesModule {
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
        }
    }

    pub fn draw(&self, cr: &cairo::Context, bar_height: f64, bar_width: u32) {
        let mut sorted_workspaces = self.workspaces.clone();
        sorted_workspaces.sort_by_key(|w| w.idx);

        if sorted_workspaces.is_empty() {
            return;
        }

        let item_padding = 15.0;
        let mut total_workspaces_width = 0.0;
        let mut extents_list = Vec::new();

        for ws in &sorted_workspaces {
            let ws_str = ws.idx.to_string();
            let extents = cr.text_extents(&ws_str).unwrap();
            total_workspaces_width += extents.x_advance();
            extents_list.push(extents);
        }

        total_workspaces_width += item_padding * (sorted_workspaces.len() as f64 - 1.0);

        let mut center_x_offset = (bar_width as f64 / 2.0) - (total_workspaces_width / 2.0);

        for (i, ws) in sorted_workspaces.into_iter().enumerate() {
            let ws_str = ws.idx.to_string();
            let text_w = extents_list[i].x_advance();
            let text_h = extents_list[i].height();

            let text_y = ((bar_height - text_h) / 2.0) - extents_list[i].y_bearing();

            if ws.is_active {
                cr.set_source_rgb(
                    config::ACTIVE_WORKSPACE_COLOR.r,
                    config::ACTIVE_WORKSPACE_COLOR.g,
                    config::ACTIVE_WORKSPACE_COLOR.b,
                );
                cr.set_line_width(config::LINE_WIDTH);
                let line_y = (bar_height / 2.0) + (text_h / 2.0) + 4.0;
                cr.move_to(center_x_offset, line_y);
                cr.line_to(center_x_offset + text_w, line_y);
                cr.stroke().unwrap();
            } else {
                cr.set_source_rgb(
                    config::INACTIVE_WORKSPACE_COLOR.r,
                    config::INACTIVE_WORKSPACE_COLOR.g,
                    config::INACTIVE_WORKSPACE_COLOR.b,
                );
            }

            cr.move_to(center_x_offset, text_y);
            cr.show_text(&ws_str).unwrap();

            center_x_offset += text_w + item_padding;
        }
    }
}
