use crate::tui::Component;

/// Loader indicator options.
pub struct LoaderIndicatorOptions {
    pub frames: Vec<String>,
    pub interval_ms: u64,
}

impl Default for LoaderIndicatorOptions {
    fn default() -> Self {
        LoaderIndicatorOptions {
            frames: vec![
                "⠋".into(),
                "⠙".into(),
                "⠹".into(),
                "⠸".into(),
                "⠼".into(),
                "⠴".into(),
                "⠦".into(),
                "⠧".into(),
                "⠇".into(),
                "⠏".into(),
            ],
            interval_ms: 80,
        }
    }
}

/// Animated loading spinner.
pub struct Loader {
    message: String,
    spinner_color: Box<dyn Fn(&str) -> String>,
    msg_color: Box<dyn Fn(&str) -> String>,
    frame: usize,
    options: LoaderIndicatorOptions,
}

impl Loader {
    pub fn new(
        spinner_color: Box<dyn Fn(&str) -> String>,
        msg_color: Box<dyn Fn(&str) -> String>,
        message: &str,
    ) -> Self {
        Loader {
            message: message.to_string(),
            spinner_color,
            msg_color,
            frame: 0,
            options: LoaderIndicatorOptions::default(),
        }
    }

    pub fn set_message(&mut self, msg: &str) {
        self.message = msg.to_string();
    }

    pub fn set_indicator_options(&mut self, opts: LoaderIndicatorOptions) {
        self.options = opts;
    }

    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % self.options.frames.len();
    }
}

impl Component for Loader {
    fn render(&self, _width: u16) -> Vec<String> {
        let frame = &self.options.frames[self.frame];
        vec![format!(
            "{} {}",
            (self.spinner_color)(frame),
            (self.msg_color)(&self.message)
        )]
    }

    fn invalidate(&mut self) {}
}

/// Cancellable loader — adds Escape handling and abort signal.
pub struct CancellableLoader {
    loader: Loader,
    pub aborted: bool,
    #[allow(clippy::type_complexity)]
    pub on_abort: Option<Box<dyn FnOnce()>>,
}

impl CancellableLoader {
    pub fn new(
        spinner_color: Box<dyn Fn(&str) -> String>,
        msg_color: Box<dyn Fn(&str) -> String>,
        message: &str,
    ) -> Self {
        CancellableLoader {
            loader: Loader::new(spinner_color, msg_color, message),
            aborted: false,
            on_abort: None,
        }
    }

    pub fn set_message(&mut self, msg: &str) {
        self.loader.set_message(msg);
    }

    pub fn tick(&mut self) {
        self.loader.tick();
    }
}

impl Component for CancellableLoader {
    fn render(&self, width: u16) -> Vec<String> {
        self.loader.render(width)
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};
        if matches_key(data, Key::escape) {
            self.aborted = true;
            if let Some(cb) = self.on_abort.take() {
                cb();
            }
        }
    }

    fn invalidate(&mut self) {
        self.loader.invalidate();
    }
}
