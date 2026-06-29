use crate::tui::Component;

pub struct ImageTheme {
    pub fallback_color: Box<dyn Fn(&str) -> String>,
}

pub struct ImageOptions {
    pub max_width_cells: Option<u32>,
    pub max_height_cells: Option<u32>,
    pub filename: Option<String>,
}

/// Image widget for Kitty/iTerm2 inline images.
pub struct Image {
    base64_data: String,
    mime_type: String,
    opts: ImageOptions,
}

impl Image {
    pub fn new(base64_data: &str, mime_type: &str, _theme: ImageTheme, opts: ImageOptions) -> Self {
        Image {
            base64_data: base64_data.to_string(),
            mime_type: mime_type.to_string(),
            opts,
        }
    }
}

impl Component for Image {
    fn render(&self, _width: u16) -> Vec<String> {
        let render_opts = crate::terminal_image::ImageRenderOptions {
            max_width_cells: self.opts.max_width_cells,
            max_height_cells: self.opts.max_height_cells,
            filename: self.opts.filename.clone(),
        };
        let output =
            crate::terminal_image::render_image(&self.base64_data, &self.mime_type, &render_opts);
        vec![output]
    }

    fn invalidate(&mut self) {}
}
