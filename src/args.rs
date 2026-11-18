use clap::{Parser, Subcommand, ValueEnum};

/// Describe a shell command in plain language, get the command back.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// What you want to do; quotes are optional (with --history: a search term)
    pub query: Vec<String>,

    /// Explain the generated command
    #[arg(short, long, conflicts_with = "alt")]
    pub verbose: bool,

    /// Show alternative commands
    #[arg(short, long)]
    pub alt: bool,

    /// Run the command after confirmation
    #[arg(short = 'x', long)]
    pub execute: bool,

    /// Ignore the cache for this query
    #[arg(short, long)]
    pub fresh: bool,

    /// Use this provider for one run (e.g. ollama, anthropic, claude-code)
    #[arg(long, value_name = "NAME")]
    pub provider: Option<String>,

    /// Use this model for one run
    #[arg(long, value_name = "NAME")]
    pub model: Option<String>,

    /// Don't copy the command to the clipboard
    #[arg(long)]
    pub no_copy: bool,

    /// Print only the command, for scripts and the shell integration
    #[arg(long, conflicts_with_all = ["verbose", "alt", "execute"])]
    pub raw: bool,

    /// Show recent commands, or search them with a query
    #[arg(long)]
    pub history: bool,

    /// Choose the provider and model
    #[arg(long)]
    pub config: bool,

    /// Print shell integration that puts commands on your prompt line: eval "$(kn --init zsh)"
    #[arg(long, value_name = "SHELL")]
    pub init: Option<Shell>,

    /// Upgrade knock to the latest version
    #[arg(long)]
    pub upgrade: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Explain what a shell command does
    Explain {
        /// The command to explain; quote it if it contains pipes or redirects
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

impl Args {
    pub fn query(&self) -> String {
        self.query.join(" ").trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("kn").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn unquoted_words_form_one_query() {
        assert_eq!(parse(&["find", "big", "files"]).query(), "find big files");
        assert_eq!(parse(&["-v", "list ports"]).query(), "list ports");
    }

    #[test]
    fn explain_is_a_subcommand() {
        match parse(&["explain", "tar", "-xzf", "a.tgz"]).command {
            Some(Command::Explain { command }) => assert_eq!(command.join(" "), "tar -xzf a.tgz"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn raw_excludes_interactive_modes() {
        assert!(Args::try_parse_from(["kn", "--raw", "-x", "q"]).is_err());
        assert!(Args::try_parse_from(["kn", "-v", "-a", "q"]).is_err());
    }
}
