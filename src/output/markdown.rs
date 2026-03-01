use std::io::{self, Write};

use termimad::MadSkin;

/// Create a token handler that prints raw tokens as they arrive (streaming mode).
pub fn streaming_token_handler() -> Box<dyn FnMut(&str)> {
    Box::new(|token: &str| {
        print!("{token}");
        let _ = io::stdout().flush();
    })
}

/// Create a token handler for thinking content in markdown mode (dimmed italic).
pub fn streaming_think_token_handler() -> Box<dyn FnMut(&str)> {
    use owo_colors::OwoColorize;
    Box::new(|token: &str| {
        print!("{}", token.dimmed().italic());
        let _ = io::stdout().flush();
    })
}

/// Render the full content as formatted markdown (--no-stream mode).
pub fn render_full(content: &str, use_color: bool) {
    if use_color {
        let skin = MadSkin::default_dark();
        skin.print_text(content);
    } else {
        println!("{content}");
    }
}
