use nu_cli::gather_parent_env_vars;
use nu_command::add_shell_command_context;
use nu_cmd_lang::create_default_context;
use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Value};
use nu_protocol::debugger::WithoutDebug;
use nu_engine::eval_block_with_early_return;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize EngineState
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    
    // Gather parent environment variables
    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    // 2. Parse and execute code
    let code_snippet = std::env::args().nth(1).expect("No code snippet provided");
    
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(
        &mut working_set,
        None,
        code_snippet.as_bytes(),
        false,
    );
    
    engine_state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    
    // 3. Execute the parsed AST
    match eval_block_with_early_return::<WithoutDebug>(&engine_state, &mut stack, &block, PipelineData::empty()) {
        Ok(pipeline_data) => {
            // Handle successful execution
            match pipeline_data.into_value(Span::test_data()) {
                Ok(value) => match value {
                    Value::String { val, .. } => println!("{}", val),
                    Value::List { vals, .. } => {
                        for val in vals {
                            println!("{:?}", val);
                        }
                    },
                    other => println!("{:?}", other),
                },
                Err(err) => eprintln!("Error converting pipeline data: {:?}", err),
            }
        }
        Err(error) => {
            // 4. Error handling
            eprintln!("Error: {:?}", error);
        }
    }

    Ok(())
}