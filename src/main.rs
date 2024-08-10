use std::io::{self, BufRead};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

mod engine;
mod run;
mod thread_pool;

enum Event {
    Line(String),
    Interrupt,
    Eof,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine_state = engine::create()?;

    let closure_snippet = std::env::args().nth(1).expect("No closure provided");
    let closure = engine::parse_closure(&mut engine_state, &closure_snippet)?;

    let (tx, rx) = mpsc::channel();
    let pool = Arc::new(thread_pool::ThreadPool::new(10));

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
        let _ = stdin_tx.send(Event::Eof);
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
                run::line(i, line, &engine_state, &closure, &pool);
                i += 1;
            }
            Event::Interrupt => {
                println!("Received interrupt signal. Shutting down...");
                break;
            }
            Event::Eof => {
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
