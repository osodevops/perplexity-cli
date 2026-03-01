use owo_colors::OwoColorize;

pub fn render_citations(citations: &[String], use_color: bool) {
    if citations.is_empty() {
        return;
    }
    println!();
    if use_color {
        println!("{}", "Citations:".bold());
    } else {
        println!("Citations:");
    }
    for (i, url) in citations.iter().enumerate() {
        if use_color {
            println!("  [{}] {}", (i + 1).dimmed(), url.cyan());
        } else {
            println!("  [{}] {}", i + 1, url);
        }
    }
}
