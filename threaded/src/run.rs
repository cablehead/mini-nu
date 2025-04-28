use std::sync::Arc;

use nu_engine::get_eval_block_with_early_return;
use nu_protocol::engine::Closure;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

use crate::thread_pool;

pub fn line(
    job_number: usize,
    line: String,
    engine_state: &EngineState,
    closure: &Closure,
    pool: &Arc<thread_pool::ThreadPool>,
) {
    let engine_state = engine_state.clone();
    let closure = closure.clone();
    pool.execute(move || {
        let mut stack = Stack::new();
        println!("Thread {} starting execution", job_number);
        let input = PipelineData::Value(Value::string(line, Span::unknown()), None);
        match eval_closure(&engine_state, &mut stack, &closure, input, job_number) {
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
                Err(err) => {
                    eprintln!(
                        "Thread {}: Error converting pipeline data: {:?}",
                        job_number, err
                    )
                }
            },
            Err(error) => {
                eprintln!("Thread {}: Error: {:?}", job_number, error);
            }
        }
    });
}

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

    let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
    eval_block_with_early_return(engine_state, stack, block, input)
}
