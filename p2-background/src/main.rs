/// Background Nushell embedding example
/// Demonstrates running Nushell commands in a background thread with job control
use nu_cli::gather_parent_env_vars;
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{EngineState, Job, Stack, StateWorkingSet, ThreadJob};
use nu_protocol::{Id, PipelineData, Signals, Span, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::Duration;

/// Creates and initializes a Nushell engine with standard commands
fn create_engine() -> Result<EngineState, Box<dyn std::error::Error>> {
    // Initialize engine with standard commands
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);

    // Set up environment
    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    Ok(engine_state)
}

/// Sets up Ctrl-C handling for the application
fn setup_ctrlc_handler(engine_state: &mut EngineState) -> Arc<AtomicBool> {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    ctrlc::set_handler({
        let interrupt = interrupt.clone();
        move || {
            interrupt.store(true, Ordering::Relaxed);
        }
    })
    .expect("Error setting Ctrl-C handler");

    interrupt
}

/// Prints the result of Nushell execution in a human-readable format
fn print_result(value: Value) {
    match value {
        Value::String { val, .. } => println!("{}", val),
        Value::List { vals, .. } => {
            for val in vals {
                println!("{:?}", val);
            }
        }
        other => println!("{:?}", other),
    }
}

/// Runs a Nushell script in a background thread with job tracking
fn run_script_in_background(
    engine_state: Arc<EngineState>,
    code_snippet: &str,
) -> Result<(Id<nu_protocol::marker::Job>, std::thread::JoinHandle<()>), Box<dyn std::error::Error>>
{
    // Create a channel for the thread job
    let (sender, _receiver) = mpsc::channel();

    // Create a new signals object for this job
    let signals = Signals::empty();

    // Create a thread job
    let job = ThreadJob::new(signals.clone(), Some("Script Job".to_string()), sender);

    // Add the job to the engine's job table
    let job_id = {
        let mut jobs = engine_state.jobs.lock().unwrap();
        jobs.add_job(Job::Thread(job.clone()))
    };

    let handle = {
        let job = job.clone();
        let script_owned = code_snippet.to_string();
        let engine_state = Arc::clone(&engine_state);

        // Spawn a thread to run the Nushell script
        std::thread::spawn(move || {
            // Set the current job context to enable background job tracking in this thread
            let mut thread_local_state = (*engine_state).clone();
            thread_local_state.current_job.background_thread_job = Some(job);

            let mut stack = Stack::new();

            // Parse the script
            let mut working_set = StateWorkingSet::new(&thread_local_state);
            let block = parse(&mut working_set, None, script_owned.as_bytes(), false);
            if let Err(err) = thread_local_state.merge_delta(working_set.render()) {
                eprintln!("Error parsing script: {:?}", err);
                return;
            }

            // Execute the script
            match eval_block_with_early_return::<WithoutDebug>(
                &thread_local_state,
                &mut stack,
                &block,
                PipelineData::empty(),
            ) {
                Ok(pipeline_data) => match pipeline_data.into_value(Span::test_data()) {
                    Ok(value) => print_result(value),
                    Err(err) => eprintln!("Error converting pipeline data: {:?}", err),
                },
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            }

            // Clean up when done - remove job from the shared job table
            let mut jobs = engine_state.jobs.lock().unwrap();
            jobs.remove_job(job_id);
        })
    };

    Ok((job_id, handle))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the Nushell engine
    let mut engine_state = create_engine()?;

    // Set up Ctrl-C protection with proper signal handling
    let interrupt = setup_ctrlc_handler(&mut engine_state);

    // Get the code to execute from command line arguments
    let code_snippet = std::env::args().nth(1).expect("No code snippet provided");

    // Wrap the engine state in an Arc to share it across threads
    let engine_state = Arc::new(engine_state);

    // Run the script in a background thread
    let (job_id, background_thread) =
        run_script_in_background(Arc::clone(&engine_state), &code_snippet)?;

    // Main event loop - poll for thread completion or ctrl-c
    println!("Script running in background. Press Ctrl-C to terminate.");
    let poll_interval = Duration::from_millis(100);
    loop {
        // Check if the background thread has completed
        if background_thread.is_finished() {
            println!("Script completed successfully.");
            break;
        }

        if interrupt.load(Ordering::Relaxed) {
            println!("Received interrupt, terminating job...");
            // Kill the job through the job table
            let mut jobs = engine_state.jobs.lock().unwrap();
            let _ = jobs.kill_and_remove(job_id);
            break;
        }

        // Brief sleep to avoid CPU spin
        std::thread::sleep(poll_interval);
    }

    Ok(())
}
