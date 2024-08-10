use std::io::{self, BufRead};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

mod engine;
mod run;
mod thread_pool;

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
                    if stdin_tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        // Channel will be closed when stdin_tx is dropped at the end of this function
    });

    // Set up ctrl-c handler
    let ctrlc_tx = Arc::new(Mutex::new(Some(tx)));
    let ctrlc_tx_clone = Arc::clone(&ctrlc_tx);
    ctrlc::set_handler(move || {
        println!("Received interrupt signal. Shutting down...");
        if let Some(tx) = ctrlc_tx_clone.lock().unwrap().take() {
            drop(tx); // This will close the channel
        }
    })?;

    let mut i = 0;
    while let Ok(line) = rx.recv() {
        run::line(i, line, &engine_state, &closure, &pool);
        i += 1;
    }

    println!("Waiting for all tasks to complete...");
    pool.wait_for_completion();
    println!("All tasks completed. Exiting.");

    Ok(())
}
