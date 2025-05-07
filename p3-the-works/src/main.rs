use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

// Core Nushell dependencies for embedding
use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Call, Closure, Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::Signals;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, Type, Value};

/// A sample custom command that demonstrates how to add commands to Nushell
#[derive(Clone)]
struct Warble;

impl Command for Warble {
    fn name(&self) -> &str {
        "warble"
    }

    fn signature(&self) -> Signature {
        Signature::build("warble")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Returns the string 'warble'"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::Value(
            Value::string("warble, oh my", Span::unknown()),
            None,
        ))
    }
}

/// Adds custom commands to the engine state.
/// This function demonstrates how to extend Nushell with custom commands.
fn add_custom_commands(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(Warble));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error adding custom commands: {err:?}");
    }

    engine_state
}

/// Creates and initializes a Nushell engine with standard and custom commands.
/// Sets up the environment variables and working directory for the engine.
fn create_engine() -> Result<EngineState, Box<dyn std::error::Error>> {
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    engine_state = add_cli_context(engine_state);
    engine_state = add_custom_commands(engine_state);

    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    Ok(engine_state)
}

/// Parses a Nushell closure from a string and returns the compiled closure.
/// This function handles parsing, rendering, and evaluation of the closure code.
fn parse_closure(
    engine_state: &mut EngineState,
    closure_snippet: &str,
) -> Result<Closure, ShellError> {
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, closure_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result =
        eval_block::<WithoutDebug>(engine_state, &mut stack, &block, PipelineData::empty())?;
    result.into_value(Span::unknown())?.into_closure()
}

/// Evaluates a Nushell closure with the given input and job number.
/// Provides the job number as a positional argument to the closure.
fn eval_closure(
    engine_state: &EngineState,
    stack: &mut Stack,
    closure: &Closure,
    input: PipelineData,
    job_number: usize,
) -> Result<PipelineData, ShellError> {
    let block = &engine_state.get_block(closure.block_id);

    // Check if the closure has exactly one required positional argument
    if block.signature.required_positional.len() != 1 {
        return Err(ShellError::NushellFailedSpanned {
            msg: "Closure must accept exactly one argument".into(),
            label: format!(
                "Found {} arguments, expected 1",
                block.signature.required_positional.len()
            ),
            span: Span::unknown(),
        });
    }

    // Add job_number as the single argument
    let var_id = block.signature.required_positional[0].var_id.unwrap();
    stack.add_var(var_id, Value::int(job_number as i64, Span::unknown()));

    eval_block_with_early_return::<WithoutDebug>(engine_state, stack, block, input)
}

/// Processes input lines from stdin and spawns Nushell tasks for each line.
/// Handles concurrent execution of multiple tasks and proper shutdown.
async fn process_input_lines(
    mut line_rx: mpsc::Receiver<String>,
    mut shutdown_rx: mpsc::Receiver<()>,
    engine_state: Arc<EngineState>,
    closure: Arc<Closure>,
    active_jobs: Arc<Mutex<usize>>,
    job_number: &mut usize,
) {
    loop {
        tokio::select! {
            biased; // Prioritize shutdown signal

            Some(_) = shutdown_rx.recv() => break, // Handle shutdown first

            result = line_rx.recv() => {
                // Handle normal channel operation
                let Some(line) = result else { break }; // Channel closed

                // Track the job and increment counter
                let current_job = *job_number;
                {
                    let mut count = active_jobs.lock().unwrap();
                    *count += 1;
                }

                // Print job start and increment counter
                println!("Thread {} starting execution", current_job);
                *job_number += 1;

                // Spawn the processing task
                let engine_state = engine_state.clone();
                let closure = closure.clone();
                let active_jobs = active_jobs.clone();

                tokio::spawn(async move {
                    // Use spawn_blocking for CPU-intensive work
                    let result = tokio::task::spawn_blocking(move || {
                        process_job(&engine_state, &closure, &line, current_job)
                    }).await;

                    // Report any errors from the blocking task
                    if let Err(e) = result {
                        eprintln!("Task error: {}", e);
                    }

                    // Decrement active job count when done
                    let mut count = active_jobs.lock().unwrap();
                    *count -= 1;
                });
            }

            else => break, // All channels closed
        }
    }
}

/// Processes a single job with the given closure in the Nushell engine.
/// Handles job tracking, execution and cleanup through Nushell's job system.
fn process_job(engine_state: &EngineState, closure: &Closure, line: &str, job_number: usize) {
    // Create a thread job for this evaluation
    let (sender, _receiver) = std::sync::mpsc::channel();

    // Get the signals from the engine state to ensure we're using the same
    // interrupt flag that's connected to Ctrl+C handling
    let signals = engine_state.signals().clone();

    // Create the job
    let job =
        nu_protocol::engine::ThreadJob::new(signals, Some(format!("Job {}", job_number)), sender);

    // Add the job to the engine's job table
    let job_id = {
        let mut jobs = engine_state.jobs.lock().unwrap();
        jobs.add_job(nu_protocol::engine::Job::Thread(job.clone()))
    };

    // Create a clone of the engine state with this job as the current job
    let mut local_engine_state = engine_state.clone();
    local_engine_state.current_job.background_thread_job = Some(job);

    // Process with the local engine state that has the job context
    let mut stack = Stack::new();
    let input = PipelineData::Value(Value::string(line, Span::unknown()), None);

    // Run the closure with the local engine state to ensure external commands
    // are registered with our job
    let result = eval_closure(&local_engine_state, &mut stack, closure, input, job_number);

    // Handle the result
    match result {
        Ok(pipeline_data) => match pipeline_data.into_value(Span::unknown()) {
            Ok(value) => match value {
                Value::String { val, .. } => println!("Thread {}: {}", job_number, val),
                Value::List { vals, .. } => {
                    for val in vals {
                        println!("Thread {}: {:?}", job_number, val);
                    }
                }
                other => println!("Thread {}: {:?}", job_number, other),
            },
            Err(err) => eprintln!(
                "Thread {}: Error converting pipeline data: {:?}",
                job_number, err
            ),
        },
        Err(error) => eprintln!("Thread {}: Error: {:?}", job_number, error),
    }

    // Remove the job from the job table when done
    {
        let mut jobs = engine_state.jobs.lock().unwrap();
        jobs.remove_job(job_id);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the engine
    let mut engine_state = create_engine()?;

    // Set up interrupt flag and attach it to engine_state
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    // Parse the closure
    let closure_snippet = std::env::args().nth(1).expect("No closure provided");
    let closure = parse_closure(&mut engine_state, &closure_snippet)?;

    // Set up tokio channels
    let (line_tx, line_rx) = mpsc::channel::<String>(100);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    // Wrap in Arc after all setup is complete
    let engine_state = Arc::new(engine_state);

    // Set up Ctrl+C handler with the shared engine_state
    let shutdown_tx_clone = shutdown_tx.clone();
    ctrlc::set_handler({
        let interrupt = interrupt.clone();
        let engine_state = engine_state.clone();

        move || {
            // Set the interrupt flag first
            interrupt.store(true, Ordering::Relaxed);

            // Kill and remove all active jobs
            let kill_result = {
                match engine_state.jobs.lock() {
                    Ok(mut jobs_guard) => {
                        let mut first_error = Ok(());

                        // Collect job IDs first
                        let job_ids: Vec<_> = jobs_guard.iter().map(|(id, _)| id).collect();

                        for id in job_ids {
                            // Call kill_and_remove for each job ID
                            if let Err(err) = jobs_guard.kill_and_remove(id) {
                                if first_error.is_ok() {
                                    first_error = Err(err);
                                }
                            }
                        }

                        first_error
                    }
                    Err(poisoned) => {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Jobs mutex poisoned: {}", poisoned),
                        ))
                    }
                }
            };

            // Signal the main loop to shut down after attempting kill/remove
            let _ = shutdown_tx_clone.try_send(());
        }
    })?;

    // Create a counter for active jobs
    let active_jobs = Arc::new(Mutex::new(0));

    // Read from stdin
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let lock = stdin.lock();

        for line in lock.lines() {
            if let Ok(line) = line {
                if line_tx.blocking_send(line).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    let closure = Arc::new(closure);
    let mut job_number = 0;

    // Process lines from stdin until EOF or interrupt
    process_input_lines(
        line_rx,
        shutdown_rx,
        engine_state,
        closure,
        active_jobs.clone(),
        &mut job_number,
    )
    .await;

    // Wait for all jobs to complete
    println!("Waiting for all tasks to complete...");

    while *active_jobs.lock().unwrap() > 0 {
        sleep(Duration::from_millis(500)).await;
    }

    println!("All tasks completed. Exiting.");
    Ok(())
}
