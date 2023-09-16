use clap::{Parser, ValueEnum};

use tokio::sync::OnceCell;

use tracing::debug;

pub const COMMANDS_FROM_ARGS_SEPARATOR: &str = ":::";

#[cfg(unix)]
const DEFAULT_SHELL: &str = "/bin/bash";
#[cfg(windows)]
const DEFAULT_SHELL: &str = "cmd";

/// Execute commands in parallel
///
/// By Aaron Riekenberg <aaron.riekenberg@gmail.com>
///
/// https://github.com/aaronriekenberg/rust-parallel
/// https://crates.io/crates/rust-parallel
#[derive(Parser, Debug, Default)]
#[command(verbatim_doc_comment, version)]
pub struct CommandLineArgs {
    /// Discard output for commands
    #[arg(short, long)]
    pub discard_output: Option<DiscardOutput>,

    /// Input file or - for stdin.  Defaults to stdin if no inputs are specified.
    #[arg(short, long)]
    pub input_file: Vec<String>,

    /// Maximum number of commands to run in parallel, defauts to num cpus
    #[arg(short, long, default_value_t = num_cpus::get(), value_parser = Self::parse_semaphore_permits)]
    pub jobs: usize,

    /// Use null separator for reading input files instead of newline.
    #[arg(short('0'), long)]
    pub null_separator: bool,

    /// Display progress bar.
    #[arg(short, long)]
    pub progress_bar: bool,

    /// Apply regex pattern to inputs.
    #[arg(short, long)]
    pub regex: Option<String>,

    /// Use shell mode for running commands.
    ///
    /// Each command line is passed to "<shell-path> -c" as a single argument.
    #[arg(short, long)]
    pub shell: bool,

    /// Timeout seconds for running commands.  Defaults to infinite timeout if not specified.
    #[arg(short, long, value_parser = Self::parse_timeout_seconds)]
    pub timeout_seconds: Option<f64>,

    /// Input and output channel capacity, defaults to num cpus * 2
    #[arg(long, default_value_t = num_cpus::get() * 2, value_parser = Self::parse_semaphore_permits)]
    pub channel_capacity: usize,

    /// Disable command path cache
    #[arg(long)]
    pub disable_path_cache: bool,

    /// Path to shell to use for shell mode
    #[arg(long, default_value = DEFAULT_SHELL)]
    pub shell_path: String,

    /// Optional command and initial arguments.
    ///
    /// If this contains 1 or more ::: delimiters the cartesian product
    /// of arguments from all groups are run.
    #[arg(trailing_var_arg(true))]
    pub command_and_initial_arguments: Vec<String>,
}

impl CommandLineArgs {
    pub async fn instance() -> &'static Self {
        static INSTANCE: OnceCell<CommandLineArgs> = OnceCell::const_new();

        INSTANCE
            .get_or_init(|| async move {
                let command_line_args = CommandLineArgs::parse();

                debug!("command_line_args = {:?}", command_line_args);

                command_line_args
            })
            .await
    }

    pub fn commands_from_args_mode(&self) -> bool {
        self.command_and_initial_arguments
            .iter()
            .any(|s| s == COMMANDS_FROM_ARGS_SEPARATOR)
    }

    fn parse_semaphore_permits(s: &str) -> Result<usize, String> {
        let range = 1..=tokio::sync::Semaphore::MAX_PERMITS;

        let value: usize = s.parse().map_err(|_| format!("`{s}` isn't a number"))?;
        if range.contains(&value) {
            Ok(value)
        } else {
            Err(format!("value not in range {:?}", range))
        }
    }

    fn parse_timeout_seconds(s: &str) -> Result<f64, String> {
        let value: f64 = s.parse().map_err(|_| format!("`{s}` isn't a number"))?;
        if value > 0f64 {
            Ok(value)
        } else {
            Err("value not greater than 0".to_string())
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DiscardOutput {
    /// Redirect stdout for commands to /dev/null
    Stdout,
    /// Redirect stderr for commands to /dev/null
    Stderr,
    /// Redirect stdout and stderr for commands to /dev/null
    All,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_clap_configuation() {
        use clap::CommandFactory;

        CommandLineArgs::command().debug_assert()
    }
}
