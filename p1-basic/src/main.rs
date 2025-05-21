/// Simple Nushell embedding example
/// Demonstrates the minimal implementation needed to run Nushell commands
use nu_cli::gather_parent_env_vars;
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Redirection, Stack, StateWorkingSet};
use nu_protocol::{OutDest, PipelineData, Value};

/// Creates and initializes a Nushell engine with standard commands
fn create_engine() -> Result<nu_protocol::engine::EngineState, Box<dyn std::error::Error>> {
    // Initialize engine with standard commands
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);

    // Set up environment
    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    Ok(engine_state)
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the Nushell engine
    let mut engine_state = create_engine()?;

    // Get the code to execute from command line arguments
    let code_snippet = std::env::args().nth(1).expect("No code snippet provided");

    // Parse the code into a block
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?;

    // Execute the parsed block
    let mut stack = Stack::new();
    // Ensure external commands block until their output is available
    let mut stack = stack.push_redirection(Some(Redirection::Pipe(OutDest::PipeSeparate)), None);

    match eval_block_with_early_return::<WithoutDebug>(
        &engine_state,
        &mut stack,
        &block,
        PipelineData::empty(),
    ) {
        Ok(pipeline_data) => {
            for value in pipeline_data {
                print_result(value);
            }
        }
        Err(error) => {
            eprintln!("Error: {:?}", error);
        }
    }

    Ok(())
}
