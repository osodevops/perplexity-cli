/// State machine for parsing `<think>...</think>` blocks from streaming content.
///
/// Reasoning models emit inline `<think>` blocks in the content stream.
/// Tags may be split across SSE chunks, so we buffer partial matches.

#[derive(Debug, Clone, PartialEq)]
pub enum ThinkEvent {
    /// Normal content (outside think blocks).
    Normal(String),
    /// Thinking content (inside think blocks).
    Think(String),
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Normal,
    /// Buffering characters that might be the start of `<think>` or `</think>`.
    MaybeTag,
    InsideThink,
    /// Buffering characters that might be `</think>`.
    MaybeCloseTag,
}

const OPEN_TAG: &str = "<think>";
const CLOSE_TAG: &str = "</think>";

#[derive(Debug)]
pub struct ThinkParser {
    state: State,
    buffer: String,
}

impl ThinkParser {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            buffer: String::new(),
        }
    }

    /// Feed a chunk of text through the parser. Returns events for any
    /// complete normal or thinking content found.
    pub fn feed(&mut self, text: &str) -> Vec<ThinkEvent> {
        let mut events = Vec::new();
        let mut normal_buf = String::new();
        let mut think_buf = String::new();

        for ch in text.chars() {
            match self.state {
                State::Normal => {
                    if ch == '<' {
                        self.buffer.clear();
                        self.buffer.push(ch);
                        self.state = State::MaybeTag;
                    } else {
                        normal_buf.push(ch);
                    }
                }
                State::MaybeTag => {
                    self.buffer.push(ch);
                    if OPEN_TAG.starts_with(&self.buffer) {
                        if self.buffer == OPEN_TAG {
                            // Matched full <think>
                            if !normal_buf.is_empty() {
                                events.push(ThinkEvent::Normal(std::mem::take(&mut normal_buf)));
                            }
                            self.buffer.clear();
                            self.state = State::InsideThink;
                        }
                        // else still partial match, keep buffering
                    } else {
                        // Not a match — flush buffer as normal content
                        normal_buf.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = State::Normal;
                    }
                }
                State::InsideThink => {
                    if ch == '<' {
                        self.buffer.clear();
                        self.buffer.push(ch);
                        self.state = State::MaybeCloseTag;
                    } else {
                        think_buf.push(ch);
                    }
                }
                State::MaybeCloseTag => {
                    self.buffer.push(ch);
                    if CLOSE_TAG.starts_with(&self.buffer) {
                        if self.buffer == CLOSE_TAG {
                            // Matched full </think>
                            if !think_buf.is_empty() {
                                events.push(ThinkEvent::Think(std::mem::take(&mut think_buf)));
                            }
                            self.buffer.clear();
                            self.state = State::Normal;
                        }
                        // else still partial match, keep buffering
                    } else {
                        // Not a close tag — flush buffer as think content
                        think_buf.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = State::InsideThink;
                    }
                }
            }
        }

        // Flush any accumulated content
        if !normal_buf.is_empty() {
            events.push(ThinkEvent::Normal(normal_buf));
        }
        if !think_buf.is_empty() {
            events.push(ThinkEvent::Think(think_buf));
        }

        events
    }

    /// Flush any remaining buffered content. Call at end of stream.
    pub fn flush(&mut self) -> Vec<ThinkEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let buf = std::mem::take(&mut self.buffer);
            match self.state {
                State::MaybeTag | State::Normal => {
                    events.push(ThinkEvent::Normal(buf));
                }
                State::MaybeCloseTag | State::InsideThink => {
                    events.push(ThinkEvent::Think(buf));
                }
            }
        }
        self.state = State::Normal;
        events
    }
}

impl Default for ThinkParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_passthrough() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("Hello, world!");
        assert_eq!(events, vec![ThinkEvent::Normal("Hello, world!".into())]);
    }

    #[test]
    fn test_think_extraction() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("<think>reasoning here</think>the answer");
        assert_eq!(
            events,
            vec![
                ThinkEvent::Think("reasoning here".into()),
                ThinkEvent::Normal("the answer".into()),
            ]
        );
    }

    #[test]
    fn test_think_block_in_middle() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("before<think>thinking</think>after");
        assert_eq!(
            events,
            vec![
                ThinkEvent::Normal("before".into()),
                ThinkEvent::Think("thinking".into()),
                ThinkEvent::Normal("after".into()),
            ]
        );
    }

    #[test]
    fn test_partial_tag_across_chunks() {
        let mut parser = ThinkParser::new();

        let e1 = parser.feed("before<thi");
        assert_eq!(e1, vec![ThinkEvent::Normal("before".into())]);

        let e2 = parser.feed("nk>reason");
        assert_eq!(e2, vec![ThinkEvent::Think("reason".into())]);

        let e3 = parser.feed("ing</thin");
        assert_eq!(e3, vec![ThinkEvent::Think("ing".into())]);

        let e4 = parser.feed("k>after");
        assert_eq!(e4, vec![ThinkEvent::Normal("after".into())]);
    }

    #[test]
    fn test_non_matching_tag_not_eaten() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("<thinking>not a think tag</thinking>");
        // <thinking> doesn't match <think> (extra chars after <think)
        // Actually <think starts matching, then 'i' after <think makes it <thinki which doesn't match
        // Let's trace: '<' -> MaybeTag, 't' -> <t matches <t, 'h' -> <th, 'i' -> <thi, 'n' -> <thin, 'k' -> <think
        // Next 'i' -> <thinki — does NOT start_with OPEN_TAG (<think>), so flush "<thinki" as normal
        // Then 'n' normal, 'g' normal, '>' normal...
        assert_eq!(
            events,
            vec![ThinkEvent::Normal(
                "<thinking>not a think tag</thinking>".into()
            )]
        );
    }

    #[test]
    fn test_lone_angle_bracket() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("a < b and c > d");
        // '<' triggers MaybeTag, ' ' after '<' doesn't match '<t', so flush '< ' as normal
        assert_eq!(events, vec![ThinkEvent::Normal("a < b and c > d".into())]);
    }

    #[test]
    fn test_multiple_think_blocks() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("<think>first</think>middle<think>second</think>end");
        assert_eq!(
            events,
            vec![
                ThinkEvent::Think("first".into()),
                ThinkEvent::Normal("middle".into()),
                ThinkEvent::Think("second".into()),
                ThinkEvent::Normal("end".into()),
            ]
        );
    }

    #[test]
    fn test_empty_think_block() {
        let mut parser = ThinkParser::new();
        let events = parser.feed("<think></think>after");
        assert_eq!(events, vec![ThinkEvent::Normal("after".into())]);
    }

    #[test]
    fn test_flush_partial_open_tag() {
        let mut parser = ThinkParser::new();
        let e1 = parser.feed("hello<thi");
        assert_eq!(e1, vec![ThinkEvent::Normal("hello".into())]);
        let e2 = parser.flush();
        assert_eq!(e2, vec![ThinkEvent::Normal("<thi".into())]);
    }

    #[test]
    fn test_flush_inside_think() {
        let mut parser = ThinkParser::new();
        let e1 = parser.feed("<think>unclosed");
        assert_eq!(e1, vec![ThinkEvent::Think("unclosed".into())]);
        let e2 = parser.flush();
        assert_eq!(e2, vec![]);
    }

    #[test]
    fn test_streaming_realistic() {
        // Simulate realistic SSE chunk boundaries
        let mut parser = ThinkParser::new();
        let chunks = vec![
            "<think>Let me ",
            "think about this.",
            "\n\nThe answer involves ",
            "multiple steps.</think>",
            "\n\nHere is the answer.",
        ];

        let mut think_parts = Vec::new();
        let mut normal_parts = Vec::new();

        for chunk in chunks {
            for event in parser.feed(chunk) {
                match event {
                    ThinkEvent::Think(s) => think_parts.push(s),
                    ThinkEvent::Normal(s) => normal_parts.push(s),
                }
            }
        }

        let think_content: String = think_parts.concat();
        let normal_content: String = normal_parts.concat();

        assert_eq!(
            think_content,
            "Let me think about this.\n\nThe answer involves multiple steps."
        );
        assert_eq!(normal_content, "\n\nHere is the answer.");
    }
}
