use crate::api::types::Usage;
use owo_colors::OwoColorize;

pub fn render(usage: &Usage, use_color: bool) {
    let Some(ref cost) = usage.cost else {
        return;
    };

    println!();

    let total = cost.total_cost.unwrap_or(0.0);
    if use_color {
        println!("{} ${:.6}", "Cost:".bold(), total);
    } else {
        println!("Cost: ${:.6}", total);
    }

    if let Some(v) = cost.input_tokens_cost {
        println!("  Input tokens:     {:>6} (${:.6})", usage.prompt_tokens, v);
    }
    if let Some(v) = cost.output_tokens_cost {
        println!(
            "  Output tokens:    {:>6} (${:.6})",
            usage.completion_tokens, v
        );
    }
    if let Some(v) = cost.request_cost {
        println!("  Request fee:             (${:.6})", v);
    }
    if let Some(v) = cost.citation_tokens_cost {
        let ct = usage.citation_tokens.unwrap_or(0);
        println!("  Citation tokens:  {:>6} (${:.6})", ct, v);
    }
    if let Some(v) = cost.reasoning_tokens_cost {
        let rt = usage.reasoning_tokens.unwrap_or(0);
        println!("  Reasoning tokens: {:>6} (${:.6})", rt, v);
    }
    if let Some(v) = cost.search_queries_cost {
        let sq = usage.num_search_queries.unwrap_or(0);
        println!("  Search queries:   {:>6} (${:.6})", sq, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Cost;

    #[test]
    fn test_render_no_cost() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            cost: None,
            search_context_size: None,
            citation_tokens: None,
            num_search_queries: None,
            reasoning_tokens: None,
        };
        // Should not panic
        render(&usage, false);
    }

    #[test]
    fn test_render_with_cost() {
        let usage = Usage {
            prompt_tokens: 500,
            completion_tokens: 200,
            total_tokens: 700,
            cost: Some(Cost {
                input_tokens_cost: Some(0.0005),
                output_tokens_cost: Some(0.0002),
                total_cost: Some(0.0057),
                request_cost: Some(0.005),
                reasoning_tokens_cost: None,
                citation_tokens_cost: None,
                search_queries_cost: None,
            }),
            search_context_size: None,
            citation_tokens: None,
            num_search_queries: None,
            reasoning_tokens: None,
        };
        // Should not panic
        render(&usage, false);
    }
}
