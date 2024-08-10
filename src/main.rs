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

    let shared_tx = Arc::new(Mutex::new(Some(tx)));

    // Spawn thread to read from stdin
    let stdin_tx = shared_tx.clone();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    let send_result = stdin_tx.lock().unwrap().as_ref().map(|tx| tx.send(line));
                    if let Some(Err(_)) = send_result {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        // Close the channel by taking and dropping the sender
        stdin_tx.lock().unwrap().take();
    });

    // Set up ctrl-c handler
    let ctrlc_tx = shared_tx.clone();
    ctrlc::set_handler(move || {
        println!("Received interrupt signal. Shutting down...");
        // Close the channel by taking and dropping the sender
        ctrlc_tx.lock().unwrap().take();
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
