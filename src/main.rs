use ctrlc;
use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::{eval_block, get_eval_block_with_early_return};
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::io::{self, BufRead};
use std::sync::mpsc::{channel};
use std::thread;

enum Event {
    LineRead(String),
    Interrupt,
    EOF,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    engine_state = add_cli_context(engine_state);
    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());
    let closure_snippet = std::env::args().nth(1).expect("No closure provided");
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = parse(&mut working_set, None, closure_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?;
    let mut stack = Stack::new();
    let result =
        eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, PipelineData::empty())?;
    let closure: Closure = result.into_value(Span::unknown())?.into_closure()?;

    let (tx, rx) = channel();

    // Spawn thread to read from stdin
    let stdin_tx = tx.clone();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    if stdin_tx.send(Event::LineRead(line)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = stdin_tx.send(Event::EOF);
    });

    // Set up ctrl-c handler
    let ctrlc_tx = tx.clone();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(Event::Interrupt);
    })?;

    let mut i = 0;
    loop {
        match rx.recv()? {
            Event::LineRead(line) => {
                let engine_state = engine_state.clone();
                let closure = closure.clone();
                thread::spawn(move || {
                    let mut stack = Stack::new();
                    println!("Thread {} starting execution", i);
                    let input = PipelineData::Value(Value::string(line, Span::unknown()), None);
                    match eval_closure(&engine_state, &mut stack, &closure, input) {
                        Ok(pipeline_data) => match pipeline_data.into_value(Span::unknown()) {
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
                i += 1;
            }
            Event::Interrupt => {
                println!("Received interrupt signal. Shutting down...");
                break;
            }
            Event::EOF => {
                println!("Reached end of input. Shutting down...");
                break;
            }
        }
    }

    Ok(())
}

fn eval_closure(
    engine_state: &EngineState,
    stack: &mut Stack,
    closure: &Closure,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let block = &engine_state.get_block(closure.block_id);
    for (_, var_id) in block.signature.required_positional.iter().enumerate() {
        if let Some(var_id) = var_id.var_id {
            stack.add_var(var_id, Value::string("", Span::unknown()));
        } else {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Error while evaluating closure".into(),
                label: "closure argument missing var_id".into(),
                span: Span::unknown(),
            });
        }
    }
    let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
    eval_block_with_early_return(engine_state, stack, block, input)
}
