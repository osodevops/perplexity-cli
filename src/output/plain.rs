use std::io::{self, Write};

/// Create a token handler for plain text streaming.
pub fn streaming_token_handler() -> Box<dyn FnMut(&str)> {
    Box::new(|token: &str| {
        print!("{token}");
        let _ = io::stdout().flush();
    })
}

/// Create a token handler for thinking content in plain text mode.
pub fn streaming_think_token_handler() -> Box<dyn FnMut(&str)> {
    let mut at_line_start = true;
    Box::new(move |token: &str| {
        for ch in token.chars() {
            if at_line_start {
                print!("[thinking] ");
                at_line_start = false;
            }
            print!("{ch}");
            if ch == '\n' {
                at_line_start = true;
            }
        }
        let _ = io::stdout().flush();
    })
}

/// Render full content as plain text.
pub fn render_full(content: &str) {
    println!("{content}");
}
