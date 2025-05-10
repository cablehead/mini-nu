# p3-the-works: Advanced Nushell Embedding

This example demonstrates a more complex embedding scenario for Nushell, building upon the concepts from `p1-basic` and `p2-background`. It showcases:

1.  **Custom Nushell Commands**: How to define and register your own commands written in Rust, making them available within the embedded Nushell engine.
2.  **Parsing and Executing Nushell Closures**: Taking a Nushell closure as input (e.g., from command-line arguments), parsing it, and then executing it for different inputs.
3.  **Managing Multiple Concurrent Background Jobs**: Processing multiple items (e.g., lines from `stdin`) concurrently, each in its own Nushell "job context" using `tokio` for asynchronous task management.
4.  **Robust Signal Handling and Shutdown**: Ensuring that `Ctrl+C` interrupts are handled gracefully, terminating all active Nushell jobs and their potential child processes, and allowing the application to shut down cleanly.
5.  **Using `add_cli_context`**: Incorporating additional CLI-related commands and context into the engine.

This example is suitable for applications that need to deeply integrate Nushell as a scripting or processing engine, offering extensibility and handling concurrent, potentially long-running tasks.

## Core Nushell Components and Concepts

### 1. Custom Commands (`nu-protocol/src/engine/command.rs`)

Nushell allows embedding applications to define their own commands by implementing the `Command` trait:

```rust
pub trait Command: Send + Sync + CommandClone + std::fmt::Debug + 'static {
    fn name(&self) -> &str;
    fn signature(&self) -> Signature;
    fn usage(&self) -> &str; // Corrected from description() for actual trait method for usage string
    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError>;
    // ... other methods ...
}
```
-   **Implementation**: In `src/main.rs`, the `Warble` struct implements `Command`.
-   **Registration**: Custom commands are added to the `EngineState` using a `StateWorkingSet` and `engine_state.merge_delta()`:
    ```rust
    // In add_custom_commands function
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(Warble)); // Warble is our custom command
        working_set.render()
    };
    engine_state.merge_delta(delta)?;
    ```

### 2. Parsing and Evaluating Closures

-   **Closures (`nu-protocol/src/ast/closure.rs` - conceptually, though Value::Closure holds the data)**: Nushell closures are first-class values.
    ```rust
    // From nu-protocol/src/value/mod.rs
    // Value::Closure { val: Box<Closure>, internal_span: Span }
    //
    // From nu-protocol/src/engine/closure.rs (actual struct)
    pub struct Closure {
        pub block_id: BlockId,
        pub captures: IndexMap<VarId, Value>, // More accurate type
    }
    ```
-   **Parsing a Closure String**: A string containing Nushell code for a closure is parsed into a `Block`, then evaluated to produce a `Value::Closure`.
    ```rust
    // In parse_closure function (src/main.rs)
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = nu_parser::parse(&mut working_set, None, closure_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?; // Apply parsing changes

    let mut stack = Stack::new();
    // Evaluate the block that defines the closure
    let result = nu_engine::eval_block::<WithoutDebug>(engine_state, &mut stack, &block, PipelineData::empty())?;
    // The result of evaluating a {|| ...} block is the closure itself
    let closure_val = result.into_value(Span::unknown())?.into_closure()?;
    ```
-   **Evaluating a Parsed Closure**: The `eval_closure` helper function in `src/main.rs` demonstrates how to execute a `Closure` with new input and arguments. It retrieves the `Block` associated with the closure's `block_id` and uses `nu_engine::eval_block_with_early_return`.

### 3. Job Management for Concurrent Tasks (`EngineState`, `ThreadJob`)

This example extends the job management from `p2-background` to handle multiple concurrent tasks, each processing a line from `stdin`.
-   For each line of input, a new `ThreadJob` is created and registered with the global `EngineState.jobs`.
-   A *clone* of the `EngineState` is made, and `current_job.background_thread_job` is set to this new `ThreadJob`. This localized `EngineState` is then used for evaluating the closure for that specific input line. This ensures that any external commands run by that instance of the closure are associated with the correct `ThreadJob`.
-   `tokio::spawn_blocking` is used to run the Nushell evaluation (which can be CPU-bound) in a way that doesn't block the `tokio` runtime.

### 4. Signal Handling (`ctrlc`, `Signals`, `EngineState.jobs`)

-   Similar to `p2-background`, `ctrlc` is used to detect `SIGINT`.
-   When an interrupt occurs:
    1.  The shared `AtomicBool` interrupt flag (part of `Signals` in `EngineState`) is set.
    2.  The handler iterates through all jobs in `engine_state.jobs.lock().unwrap()` and calls `kill_and_remove()` for each. This leverages Nushell's built-in mechanism to signal `ThreadJob`s and, by extension, their associated external processes.
    3.  A `tokio` shutdown signal is sent to gracefully stop the main input processing loop.

### 5. `add_cli_context` (`nu-cli/src/lib.rs`)

The `nu_cli::add_cli_context` function is used during engine setup. This function adds a suite of commands and configurations typically available in the Nushell CLI but not part of the bare `create_default_context()`, such as `less`, `tutor`, etc., and registers environment converters.

```rust
// In create_engine function (src/main.rs)
engine_state = nu_cli::add_cli_context(engine_state);
```

## Embedding Workflow

1.  **Initialize Engine**: Create `EngineState`, add default, shell, CLI, and custom commands.
2.  **Setup Signal Handling**: Configure `ctrlc` to set an interrupt flag on `EngineState.signals` and trigger job termination and application shutdown.
3.  **Parse Input Closure**: Take a Nushell closure string (e.g., from args), parse it, and evaluate it to get a `Value::Closure`.
4.  **Process Input Lines (Asynchronously with Tokio)**:
    a.  Read lines from `stdin` in a separate (standard) thread.
    b.  For each line, send it to a `tokio` MPSC channel.
    c.  The main `tokio` async `process_input_lines` function receives lines from the channel:
        i.  For each line, spawn a `tokio` task (using `tokio::task::spawn_blocking` for the potentially CPU-bound Nushell evaluation).
        ii. Inside the task executed by `spawn_blocking`:
            1.  Create a new `ThreadJob` and add it to `EngineState.jobs`.
            2.  Create a local `EngineState` clone and set its `current_job.background_thread_job` to the new `ThreadJob`.
            3.  Evaluate the parsed closure using this local `EngineState`, passing the input line and a job number.
            4.  Print results.
            5.  Remove the `ThreadJob` from `EngineState.jobs`.
5.  **Shutdown**:
    a.  On `Ctrl+C`, the handler kills all jobs and signals the `tokio` processing loop (via `shutdown_tx`) to stop accepting new work.
    b.  On `stdin` EOF, the input thread finishes, the MPSC `line_tx` channel is dropped, and the `tokio` processing loop (`line_rx.recv()`) will eventually receive `None` and terminate.
    c.  The application waits for all active `tokio` tasks (tracked by `active_jobs` counter) to complete before exiting.

## Example Usage

The application expects a Nushell closure as its first command-line argument. This closure will be applied to each line read from `stdin`. The closure in `p3-the-works` is set up to receive one argument, which the Rust code populates with a unique job/line number. `$in` can be used within the closure to access the line content.

**Example 1: Simple line processing**
Run the application:
```bash
cargo run -p p3-the-works -- '{|idx| $"Job ({$idx}) processed line: ($in | str upcase)" }'
```
Then, in the running application, type:
```
hello nushell
this is fun
```
Output will be something like:
```
Enter lines of text to process with the Nushell closure:
(Press Ctrl+C to exit)
Thread 0 starting execution
Thread 1 starting execution
Thread 0: Job (0) processed line: HELLO NUSHELL
Thread 1: Job (1) processed line: THIS IS FUN
```
(Order of "processed line" output may vary due to concurrency). Press `Ctrl+D` to end input or `Ctrl+C` to interrupt.

**Example 2: Using the custom `warble` command**
```bash
cargo run -p p3-the-works -- '{|_| warble | str join "-" }'
```
Then, type:
```
trigger warble
another one
```
Output (order of "Thread X starting" and result lines may interleave):
```
Enter lines of text to process with the Nushell closure:
(Press Ctrl+C to exit)
Thread 0 starting execution
Thread 0: w-a-r-b-l-e-,- -o-h- -m-y
Thread 1 starting execution
Thread 1: w-a-r-b-l-e-,- -o-h- -m-y
```

**Example 3: Running external commands concurrently**
```bash
cargo run -p p3-the-works -- '{|i| $"Job ({$i}) sleeping for ($in) seconds..."; ^sleep ($in | into int); $"Job ({$i}) woke up." }'
```
Then, type:
```
3
1
2
```
This will run three `sleep` commands concurrently. `Ctrl+C` will terminate them all.

## Testing

The tests verify:
-   Concurrent execution of closures with varying sleep times, checking for correct output order based on completion time.
-   Execution of the custom `warble` command.
-   Correct termination of concurrent external processes (like `sleep`) when the main application is interrupted via `SIGINT`.

```bash
cargo test -p p3-the-works
```