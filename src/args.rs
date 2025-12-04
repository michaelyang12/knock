use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    pub(crate) input: String,

    #[arg(short, long)]
    pub(crate) verbose: bool,
}
