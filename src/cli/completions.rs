use clap::CommandFactory;
use clap_complete::generate;
use std::io;

use super::args::Cli;

pub fn generate_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "pplx", &mut io::stdout());
}
