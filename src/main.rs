use std::io::{self, BufRead};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

mod engine;
mod run;
mod thread_pool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    spawn_stdin_reader(tx.clone());

    let ctrlc_tx = tx.clone();
    ctrlc::set_handler(move || {
        println!("Received interrupt signal. Shutting down...");
        // Close the channel by taking and dropping the sender
        ctrlc_tx.lock().unwrap().take();
    })?;

    let mut engine_state = engine::create()?;

    let closure_snippet = std::env::args().nth(1).expect("No closure provided");
    let closure = engine::parse_closure(&mut engine_state, &closure_snippet)?;

    let pool = Arc::new(thread_pool::ThreadPool::new(10));

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

fn spawn_stdin_reader(tx: Arc<Mutex<Option<mpsc::Sender<String>>>>) {
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if !line
                .ok()
                .and_then(|line| tx.lock().unwrap().as_ref().map(|tx| tx.send(line).is_ok()))
                .unwrap_or(false)
            {
                break;
            }
        }
        // Close the channel by taking and dropping the sender
        tx.lock().unwrap().take();
    });
}
