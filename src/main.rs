use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Value};
use std::io::{self, BufRead};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    engine_state = add_cli_context(engine_state);

    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    let code_snippet = std::env::args().nth(1).expect("No code snippet provided");

    let mut threads = vec![];

    for (i, line) in io::stdin().lock().lines().enumerate() {
        let line = line?;
        let mut engine_state = engine_state.clone();
        let snippet = code_snippet.clone();

        let thread = thread::spawn(move || {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block = nu_parser::parse(&mut working_set, None, snippet.as_bytes(), false);

            engine_state.merge_delta(working_set.render()).unwrap();

            let mut stack = Stack::new();

            println!("Thread {} starting execution", i);

            let input_data = PipelineData::Value(Value::string(line, Span::unknown()), None);

            match eval_block_with_early_return::<WithoutDebug>(
                &engine_state,
                &mut stack,
                &block,
                input_data,
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
        });

        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }

    Ok(())
}
