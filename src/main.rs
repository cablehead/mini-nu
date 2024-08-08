use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Value};
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize base EngineState
    let mut base_engine_state = create_default_context();
    base_engine_state = add_shell_command_context(base_engine_state);
    base_engine_state = add_cli_context(base_engine_state);

    // Gather parent environment variables
    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut base_engine_state, init_cwd.as_ref());

    // 2. Get the code snippet
    let code_snippet = std::env::args().nth(1).expect("No code snippet provided");

    // 3. Prepare the base engine state for cloning
    let base_engine_state = Arc::new(base_engine_state);

    // 4. Create two threads, each running the snippet
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let engine_state = Arc::clone(&base_engine_state);
            let snippet = code_snippet.clone();

            thread::spawn(move || {
                let mut engine_state = (*engine_state).clone(); // Clone the EngineState
                let mut working_set = StateWorkingSet::new(&engine_state);
                let block = nu_parser::parse(&mut working_set, None, snippet.as_bytes(), false);

                engine_state.merge_delta(working_set.render()).unwrap();

                let mut stack = Stack::new();

                println!("Thread {} starting execution", i);

                match eval_block_with_early_return::<WithoutDebug>(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::empty(),
                ) {
                    Ok(pipeline_data) => match pipeline_data.into_value(Span::test_data()) {
                        Ok(value) => match value {
                            Value::String { val, .. } => println!("Thread {}: {}", i, val),
                            Value::List { vals, .. } => {
                                for val in vals {
                                    println!("Thread {}: {:?}", i, val);
                                }
                            }
                            other => println!("Thread {}: {:?}", i, other),
                        },
                        Err(err) => {
                            eprintln!("Thread {}: Error converting pipeline data: {:?}", i, err)
                        }
                    },
                    Err(error) => {
                        eprintln!("Thread {}: Error: {:?}", i, error);
                    }
                }
            })
        })
        .collect();

    // 5. Wait for both threads to complete
    for thread in threads {
        thread.join().unwrap();
    }

    Ok(())
}
