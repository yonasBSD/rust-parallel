use anyhow::Context;

use awaitgroup::WaitGroup;

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader, Stderr, Stdout},
    process::Command as TokioCommand,
    sync::{Mutex, OwnedSemaphorePermit, Semaphore},
};

use tracing::{debug, trace, warn};

use std::sync::Arc;

use crate::command_line_args;

#[derive(Debug, Clone, Copy)]
enum Input {
    Stdin,

    File { file_name: &'static str },
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::Stdin => write!(f, "stdin"),
            Input::File { file_name } => write!(f, "file({})", file_name),
        }
    }
}

struct OutputWriter {
    stdout: Mutex<Stdout>,
    stderr: Mutex<Stderr>,
}

impl OutputWriter {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            stdout: Mutex::new(tokio::io::stdout()),
            stderr: Mutex::new(tokio::io::stderr()),
        })
    }

    async fn write_to_stdout(&self, mut buffer: &[u8]) {
        let mut stdout = self.stdout.lock().await;

        let result = tokio::io::copy(&mut buffer, &mut *stdout).await;
        trace!("stdout copy result = {:?}", result);
    }

    async fn write_to_stderr(&self, mut buffer: &[u8]) {
        let mut stderr = self.stderr.lock().await;

        let result = tokio::io::copy(&mut buffer, &mut *stderr).await;
        trace!("stderr copy result = {:?}", result);
    }

    async fn write(&self, stdout_buffer: &[u8], stderr_buffer: &[u8]) {
        if !stdout_buffer.is_empty() {
            self.write_to_stdout(stdout_buffer).await;
        }
        if !stderr_buffer.is_empty() {
            self.write_to_stderr(stderr_buffer).await;
        }
    }
}

#[derive(Debug)]
struct Command {
    input: Input,
    line_number: u64,
    command: String,
    shell_enabled: bool,
}

impl Command {
    async fn run(
        self,
        worker: awaitgroup::Worker,
        permit: OwnedSemaphorePermit,
        output_writer: Arc<OutputWriter>,
    ) {
        debug!(
            "begin run command = {:?} worker = {:?} permit = {:?}",
            self, worker, permit
        );

        let command_output = if self.shell_enabled {
            TokioCommand::new("/bin/sh")
                .args(["-c", &self.command])
                .output()
                .await
        } else {
            let split: Vec<_> = self.command.split_whitespace().collect();

            let [command, args @ ..] = split.as_slice() else {
                panic!("invalid command '{}'", self.command);
            };

            TokioCommand::new(command).args(args).output().await
        };

        match command_output {
            Ok(output) => {
                debug!("got command status = {}", output.status);
                output_writer.write(&output.stdout, &output.stderr).await;
            }
            Err(e) => {
                warn!("got error running command ({}): {}", self, e);
            }
        };

        debug!(
            "begin run command = {:?} worker = {:?} permit = {:?}",
            self, worker, permit
        );
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "input={},line_number={},command={},shell_enabled={}",
            self.input, self.line_number, self.command, self.shell_enabled,
        )
    }
}

pub struct CommandService {
    command_semaphore: Arc<Semaphore>,
    wait_group: WaitGroup,
    output_writer: Arc<OutputWriter>,
}

impl CommandService {
    pub fn new() -> Self {
        Self {
            command_semaphore: Arc::new(Semaphore::new(*command_line_args::instance().jobs())),
            wait_group: WaitGroup::new(),
            output_writer: OutputWriter::new(),
        }
    }

    async fn process_one_input(
        &self,
        input: Input,
        mut input_reader: BufReader<impl AsyncRead + Unpin>,
    ) -> anyhow::Result<()> {
        debug!("begin process_one_input input = {:?}", input);

        let args = command_line_args::instance();

        let mut line = String::new();
        let mut line_number = 0u64;

        loop {
            line.clear();

            let bytes_read = input_reader
                .read_line(&mut line)
                .await
                .context("read_line error")?;
            if bytes_read == 0 {
                break;
            }

            line_number += 1;

            let trimmed_line = line.trim();

            debug!("read line {}", trimmed_line);

            if trimmed_line.is_empty() || trimmed_line.starts_with("#") {
                continue;
            }

            let permit = Arc::clone(&self.command_semaphore)
                .acquire_owned()
                .await
                .context("command_semaphore.acquire_owned error")?;

            let command = Command {
                input,
                line_number,
                command: trimmed_line.to_owned(),
                shell_enabled: *args.shell_enabled(),
            };

            tokio::spawn(command.run(
                self.wait_group.worker(),
                permit,
                Arc::clone(&self.output_writer),
            ));
        }

        debug!("end process_one_input input = {:?}", input);

        Ok(())
    }

    async fn process_inputs(&self, inputs: Vec<Input>) -> anyhow::Result<()> {
        for input in inputs {
            match input {
                Input::Stdin => {
                    let input_reader = BufReader::new(tokio::io::stdin());

                    self.process_one_input(input, input_reader).await?;
                }
                Input::File { file_name } => {
                    let file = tokio::fs::File::open(file_name).await.with_context(|| {
                        format!("error opening input file file_name = '{}'", file_name)
                    })?;
                    let input_reader = BufReader::new(file);

                    self.process_one_input(input, input_reader).await?;
                }
            }
        }
        Ok(())
    }

    fn build_inputs(&self) -> Vec<Input> {
        let args = command_line_args::instance();

        if args.inputs().is_empty() {
            vec![Input::Stdin]
        } else {
            args.inputs()
                .iter()
                .map(|input_name| {
                    if input_name == "-" {
                        Input::Stdin
                    } else {
                        Input::File {
                            file_name: input_name,
                        }
                    }
                })
                .collect()
        }
    }

    pub async fn spawn_commands(self) -> anyhow::Result<WaitGroup> {
        debug!("begin spawn_commands");

        let inputs = self.build_inputs();

        self.process_inputs(inputs).await?;

        debug!("end spawn_commands");

        Ok(self.wait_group)
    }
}
