use std::io::{self, BufRead};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::{eval_block, get_eval_block_with_early_return};
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span, Value};

use ctrlc;

use crossbeam_channel;

enum Event {
    Line(String),
    Interrupt,
    EOF,
}

struct ThreadPool {
    tx: crossbeam_channel::Sender<Box<dyn FnOnce() + Send + 'static>>,
    active_count: Arc<AtomicUsize>,
    completion_pair: Arc<(Mutex<()>, Condvar)>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        let (tx, rx) = crossbeam_channel::bounded::<Box<dyn FnOnce() + Send + 'static>>(0);
        let active_count = Arc::new(AtomicUsize::new(0));
        let completion_pair = Arc::new((Mutex::new(()), Condvar::new()));

        for _ in 0..size {
            let rx = rx.clone();
            let active_count = active_count.clone();
            let completion_pair = completion_pair.clone();

            thread::spawn(move || {
                while let Ok(job) = rx.recv() {
                    active_count.fetch_add(1, Ordering::SeqCst);
                    job();
                    if active_count.fetch_sub(1, Ordering::SeqCst) == 1 {
                        let (lock, cvar) = &*completion_pair;
                        let guard = lock.lock().unwrap();
                        cvar.notify_all();
                        drop(guard);
                    }
                }
            });
        }

        ThreadPool {
            tx,
            active_count,
            completion_pair,
        }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx.send(Box::new(f)).unwrap();
    }

    fn wait_for_completion(&self) {
        let (lock, cvar) = &*self.completion_pair;
        let mut guard = lock.lock().unwrap();
        while self.active_count.load(Ordering::SeqCst) > 0 {
            guard = cvar.wait(guard).unwrap();
        }
    }
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

    let (tx, rx) = mpsc::channel();
    let pool = Arc::new(ThreadPool::new(10));

    // Spawn thread to read from stdin
    let stdin_tx = tx.clone();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    if stdin_tx.send(Event::Line(line)).is_err() {
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
            Event::Line(line) => {
                let engine_state = engine_state.clone();
                let closure = closure.clone();
                let pool = pool.clone();
                let job_number = i;
                pool.execute(move || {
                    let mut stack = Stack::new();
                    println!("Thread {} starting execution", job_number);
                    let input = PipelineData::Value(Value::string(line, Span::unknown()), None);
                    match eval_closure(&engine_state, &mut stack, &closure, input, job_number) {
                        Ok(pipeline_data) => match pipeline_data.into_value(Span::unknown()) {
                            Ok(value) => match value {
                                Value::String { val, .. } => {
                                    println!("Thread {}: {}", job_number, val)
                                }
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

    println!("Waiting for all tasks to complete...");
    pool.wait_for_completion();
    println!("All tasks completed. Exiting.");

    Ok(())
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
            )
            .into(),
            span: Span::unknown(),
        });
    }

    // Add job_number as the single argument
    let var_id = block.signature.required_positional[0].var_id.unwrap();
    stack.add_var(var_id, Value::int(job_number as i64, Span::unknown()));

    let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
    eval_block_with_early_return(engine_state, stack, block, input)
}
