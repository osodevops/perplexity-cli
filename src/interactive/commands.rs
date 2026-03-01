use crate::api::types::Message;
use crate::cost::tracker::CostTracker;

use super::SessionConfig;

pub enum CommandResult {
    Continue,
    Quit,
    Error(String),
}

pub fn handle_command(
    input: &str,
    config: &mut SessionConfig,
    messages: &mut Vec<Message>,
    cost_tracker: &CostTracker,
) -> CommandResult {
    let input = input.trim();
    let (cmd, rest) = match input.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (input, ""),
    };

    match cmd {
        "/quit" | "/exit" => CommandResult::Quit,
        "/model" => {
            if rest.is_empty() {
                eprintln!("Current model: {}", config.model);
            } else {
                config.model = rest.to_string();
                eprintln!("Model set to: {}", config.model);
            }
            CommandResult::Continue
        }
        "/system" => {
            if rest.is_empty() {
                match &config.system_prompt {
                    Some(p) => eprintln!("System prompt: {p}"),
                    None => eprintln!("No system prompt set."),
                }
            } else {
                config.system_prompt = Some(rest.to_string());
                eprintln!("System prompt updated.");
            }
            CommandResult::Continue
        }
        "/clear" => {
            messages.clear();
            eprintln!("Conversation history cleared.");
            CommandResult::Continue
        }
        "/history" => {
            if messages.is_empty() {
                eprintln!("No messages in history.");
            } else {
                for msg in messages.iter() {
                    let prefix = match msg.role.as_str() {
                        "user" => "You",
                        "assistant" => "AI",
                        "system" => "System",
                        _ => &msg.role,
                    };
                    let content_preview = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..200])
                    } else {
                        msg.content.clone()
                    };
                    eprintln!("[{prefix}] {content_preview}");
                    eprintln!();
                }
            }
            CommandResult::Continue
        }
        "/cost" => {
            cost_tracker.render(!config.no_color);
            CommandResult::Continue
        }
        "/domain" => {
            let (sub, arg) = match rest.split_once(char::is_whitespace) {
                Some((s, a)) => (s, a.trim()),
                None => (rest, ""),
            };
            match sub {
                "add" => {
                    if arg.is_empty() {
                        return CommandResult::Error("Usage: /domain add <domain>".to_string());
                    }
                    config.search_domains.push(arg.to_string());
                    eprintln!("Added domain filter: {arg}");
                }
                "remove" => {
                    if arg.is_empty() {
                        return CommandResult::Error("Usage: /domain remove <domain>".to_string());
                    }
                    config.search_domains.retain(|d| d != arg);
                    config.search_exclude_domains.retain(|d| d != arg);
                    eprintln!("Removed domain filter: {arg}");
                }
                "clear" => {
                    config.search_domains.clear();
                    config.search_exclude_domains.clear();
                    eprintln!("All domain filters cleared.");
                }
                _ => {
                    eprintln!("Domain filters:");
                    if config.search_domains.is_empty() && config.search_exclude_domains.is_empty()
                    {
                        eprintln!("  (none)");
                    }
                    for d in &config.search_domains {
                        eprintln!("  include: {d}");
                    }
                    for d in &config.search_exclude_domains {
                        eprintln!("  exclude: {d}");
                    }
                }
            }
            CommandResult::Continue
        }
        "/recency" => {
            if rest.is_empty() {
                match &config.search_recency {
                    Some(r) => eprintln!("Recency filter: {r}"),
                    None => eprintln!("No recency filter set."),
                }
            } else {
                config.search_recency = Some(rest.to_string());
                eprintln!("Recency filter set to: {rest}");
            }
            CommandResult::Continue
        }
        "/mode" => {
            if rest.is_empty() {
                match &config.search_mode {
                    Some(m) => eprintln!("Search mode: {m}"),
                    None => eprintln!("No search mode set (default)."),
                }
            } else {
                config.search_mode = Some(rest.to_string());
                eprintln!("Search mode set to: {rest}");
            }
            CommandResult::Continue
        }
        "/context" => {
            if rest.is_empty() {
                match &config.search_context_size {
                    Some(c) => eprintln!("Context size: {c}"),
                    None => eprintln!("No context size set (default)."),
                }
            } else {
                config.search_context_size = Some(rest.to_string());
                eprintln!("Context size set to: {rest}");
            }
            CommandResult::Continue
        }
        "/export" => {
            let filename = if rest.is_empty() {
                "conversation.md"
            } else {
                rest
            };
            match export_conversation(messages, filename) {
                Ok(()) => eprintln!("Exported to: {filename}"),
                Err(e) => return CommandResult::Error(format!("Export failed: {e}")),
            }
            CommandResult::Continue
        }
        "/help" => {
            eprintln!("Available commands:");
            eprintln!("  /model [name]       Show or set the model");
            eprintln!("  /system [prompt]    Show or set the system prompt");
            eprintln!("  /clear              Clear conversation history");
            eprintln!("  /history            Show conversation history");
            eprintln!("  /cost               Show session cost summary");
            eprintln!("  /domain add|remove|clear [domain]");
            eprintln!("  /recency [val]      Set recency filter (hour/day/week/month/year)");
            eprintln!("  /mode [val]         Set search mode (web/academic/sec)");
            eprintln!("  /context [val]      Set search context size");
            eprintln!("  /export [file]      Export conversation to file");
            eprintln!("  /help               Show this help");
            eprintln!("  /quit /exit         Exit interactive mode");
            CommandResult::Continue
        }
        _ => CommandResult::Error(format!("Unknown command: {cmd}. Type /help for help.")),
    }
}

fn export_conversation(messages: &[Message], filename: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(filename)?;
    for msg in messages {
        let prefix = match msg.role.as_str() {
            "user" => "## User",
            "assistant" => "## Assistant",
            "system" => "## System",
            _ => "## Unknown",
        };
        writeln!(file, "{prefix}\n")?;
        writeln!(file, "{}\n", msg.content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Message;

    fn make_config() -> SessionConfig {
        SessionConfig {
            model: "sonar-pro".to_string(),
            system_prompt: None,
            output_format: "md".to_string(),
            show_reasoning: false,
            search_mode: None,
            search_recency: None,
            search_domains: vec![],
            search_exclude_domains: vec![],
            search_context_size: None,
            no_color: false,
        }
    }

    #[test]
    fn test_quit() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        assert!(matches!(
            handle_command("/quit", &mut config, &mut messages, &tracker),
            CommandResult::Quit
        ));
    }

    #[test]
    fn test_exit() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        assert!(matches!(
            handle_command("/exit", &mut config, &mut messages, &tracker),
            CommandResult::Quit
        ));
    }

    #[test]
    fn test_model_set() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        handle_command("/model sonar", &mut config, &mut messages, &tracker);
        assert_eq!(config.model, "sonar");
    }

    #[test]
    fn test_system_set() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        handle_command(
            "/system You are a helpful assistant",
            &mut config,
            &mut messages,
            &tracker,
        );
        assert_eq!(
            config.system_prompt,
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_clear() {
        let mut config = make_config();
        let mut messages = vec![Message {
            role: "user".to_string(),
            content: "hello".to_string(),
        }];
        let tracker = CostTracker::new();
        handle_command("/clear", &mut config, &mut messages, &tracker);
        assert!(messages.is_empty());
    }

    #[test]
    fn test_domain_add() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        handle_command(
            "/domain add example.com",
            &mut config,
            &mut messages,
            &tracker,
        );
        assert_eq!(config.search_domains, vec!["example.com".to_string()]);
    }

    #[test]
    fn test_domain_clear() {
        let mut config = make_config();
        config.search_domains = vec!["a.com".to_string()];
        let mut messages = vec![];
        let tracker = CostTracker::new();
        handle_command("/domain clear", &mut config, &mut messages, &tracker);
        assert!(config.search_domains.is_empty());
    }

    #[test]
    fn test_recency() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        handle_command("/recency week", &mut config, &mut messages, &tracker);
        assert_eq!(config.search_recency, Some("week".to_string()));
    }

    #[test]
    fn test_unknown_command() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        let result = handle_command("/foobar", &mut config, &mut messages, &tracker);
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn test_help() {
        let mut config = make_config();
        let mut messages = vec![];
        let tracker = CostTracker::new();
        assert!(matches!(
            handle_command("/help", &mut config, &mut messages, &tracker),
            CommandResult::Continue
        ));
    }
}
