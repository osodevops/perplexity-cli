use std::io::{self, Write};

/// Create a token handler for plain text streaming.
pub fn streaming_token_handler() -> Box<dyn FnMut(&str)> {
    Box::new(|token: &str| {
        print!("{token}");
        let _ = io::stdout().flush();
    })
}

/// Render full content as plain text.
pub fn render_full(content: &str) {
    println!("{content}");
}
