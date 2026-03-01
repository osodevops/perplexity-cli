use crate::api::types::Usage;
use owo_colors::OwoColorize;

/// Accumulates usage/cost across multiple API calls within a session.
#[derive(Debug, Default)]
pub struct CostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub request_count: u32,
    // Per-category cost accumulators
    pub input_cost: f64,
    pub output_cost: f64,
    pub request_cost: f64,
    pub citation_cost: f64,
    pub reasoning_cost: f64,
    pub search_cost: f64,
    pub total_citation_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub total_search_queries: u64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulate usage from a single API call.
    pub fn add(&mut self, usage: &Usage) {
        self.request_count += 1;
        self.total_input_tokens += usage.prompt_tokens as u64;
        self.total_output_tokens += usage.completion_tokens as u64;
        self.total_tokens += usage.total_tokens as u64;

        if let Some(ct) = usage.citation_tokens {
            self.total_citation_tokens += ct as u64;
        }
        if let Some(rt) = usage.reasoning_tokens {
            self.total_reasoning_tokens += rt as u64;
        }
        if let Some(sq) = usage.num_search_queries {
            self.total_search_queries += sq as u64;
        }

        if let Some(ref cost) = usage.cost {
            self.total_cost += cost.total_cost.unwrap_or(0.0);
            self.input_cost += cost.input_tokens_cost.unwrap_or(0.0);
            self.output_cost += cost.output_tokens_cost.unwrap_or(0.0);
            self.request_cost += cost.request_cost.unwrap_or(0.0);
            self.citation_cost += cost.citation_tokens_cost.unwrap_or(0.0);
            self.reasoning_cost += cost.reasoning_tokens_cost.unwrap_or(0.0);
            self.search_cost += cost.search_queries_cost.unwrap_or(0.0);
        }
    }

    /// Render a full cumulative cost summary.
    pub fn render(&self, use_color: bool) {
        if self.request_count == 0 {
            println!("No API calls made yet.");
            return;
        }

        println!();
        if use_color {
            println!(
                "{} ({} requests)",
                "Session Cost Summary:".bold(),
                self.request_count
            );
            println!("  {} ${:.6}", "Total cost:".bold(), self.total_cost);
        } else {
            println!("Session Cost Summary: ({} requests)", self.request_count);
            println!("  Total cost: ${:.6}", self.total_cost);
        }

        println!(
            "  Input tokens:     {:>6} (${:.6})",
            self.total_input_tokens, self.input_cost
        );
        println!(
            "  Output tokens:    {:>6} (${:.6})",
            self.total_output_tokens, self.output_cost
        );
        if self.request_cost > 0.0 {
            println!("  Request fees:            (${:.6})", self.request_cost);
        }
        if self.total_citation_tokens > 0 {
            println!(
                "  Citation tokens:  {:>6} (${:.6})",
                self.total_citation_tokens, self.citation_cost
            );
        }
        if self.total_reasoning_tokens > 0 {
            println!(
                "  Reasoning tokens: {:>6} (${:.6})",
                self.total_reasoning_tokens, self.reasoning_cost
            );
        }
        if self.total_search_queries > 0 {
            println!(
                "  Search queries:   {:>6} (${:.6})",
                self.total_search_queries, self.search_cost
            );
        }
        println!("  Total tokens:     {:>6}", self.total_tokens);
    }

    /// One-line summary for session exit.
    pub fn summary_line(&self) -> String {
        if self.request_count == 0 {
            return "No API calls made.".to_string();
        }
        format!(
            "Session: {} requests, {} tokens, ${:.6}",
            self.request_count, self.total_tokens, self.total_cost
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Cost;

    fn sample_usage(prompt: u32, completion: u32, total_cost_val: f64) -> Usage {
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            cost: Some(Cost {
                input_tokens_cost: Some(total_cost_val * 0.3),
                output_tokens_cost: Some(total_cost_val * 0.2),
                total_cost: Some(total_cost_val),
                request_cost: Some(0.005),
                reasoning_tokens_cost: None,
                citation_tokens_cost: None,
                search_queries_cost: None,
            }),
            search_context_size: None,
            citation_tokens: None,
            num_search_queries: None,
            reasoning_tokens: None,
        }
    }

    #[test]
    fn test_accumulate_multiple() {
        let mut tracker = CostTracker::new();
        tracker.add(&sample_usage(100, 50, 0.01));
        tracker.add(&sample_usage(200, 100, 0.02));

        assert_eq!(tracker.request_count, 2);
        assert_eq!(tracker.total_input_tokens, 300);
        assert_eq!(tracker.total_output_tokens, 150);
        assert_eq!(tracker.total_tokens, 450);
        assert!((tracker.total_cost - 0.03).abs() < 1e-9);
    }

    #[test]
    fn test_missing_cost_data() {
        let mut tracker = CostTracker::new();
        let usage = Usage {
            prompt_tokens: 50,
            completion_tokens: 25,
            total_tokens: 75,
            cost: None,
            search_context_size: None,
            citation_tokens: None,
            num_search_queries: None,
            reasoning_tokens: None,
        };
        tracker.add(&usage);

        assert_eq!(tracker.request_count, 1);
        assert_eq!(tracker.total_tokens, 75);
        assert_eq!(tracker.total_cost, 0.0);
    }

    #[test]
    fn test_zero_state_render() {
        let tracker = CostTracker::new();
        // Should not panic
        tracker.render(false);
        assert_eq!(tracker.summary_line(), "No API calls made.");
    }

    #[test]
    fn test_summary_line() {
        let mut tracker = CostTracker::new();
        tracker.add(&sample_usage(100, 50, 0.01));
        let line = tracker.summary_line();
        assert!(line.contains("1 requests"));
        assert!(line.contains("150 tokens"));
        assert!(line.contains("$0.010000"));
    }
}
