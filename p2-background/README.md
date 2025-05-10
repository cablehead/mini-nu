## Core Nushell Components for Job Control

### 1. `nu-protocol/src/engine/jobs.rs`

This file defines the core structures for job management including `Jobs`, `ThreadJob`, and `FrozenJob`.

```rust
// Key structures for job management
pub struct Jobs {
    next_job_id: usize,
    last_frozen_job_id: Option<JobId>,
    jobs: HashMap<JobId, Job>,
}

#[derive(Clone)]
pub struct ThreadJob {
    signals: Signals,
    pids: Arc<Mutex<HashSet<u32>>>,
    tag: Option<String>,
    pub sender: Sender<Mail>,
}

pub enum Job {
    Thread(ThreadJob),
    Frozen(FrozenJob),
}
```

Important methods:

```rust
impl ThreadJob {
    pub fn new(signals: Signals, tag: Option<String>, sender: Sender<Mail>) -> Self {
        ThreadJob {
            signals,
            pids: Arc::new(Mutex::new(HashSet::default())),
            sender,
            tag,
        }
    }

    pub fn try_add_pid(&self, pid: u32) -> bool {
        let mut pids = self.pids.lock().expect("PIDs lock was poisoned");

        // note: this signals check must occur after the pids lock has been locked.
        if self.signals.interrupted() {
            false
        } else {
            pids.insert(pid);
            true
        }
    }

    pub fn kill(&self) -> std::io::Result<()> {
        self.signals.trigger();

        let mut pids = self.pids.lock().expect("PIDs lock was poisoned");

        for pid in pids.iter() {
            kill_by_pid((*pid).into())?;
        }

        pids.clear();

        Ok(())
    }
}

impl Jobs {
    pub fn kill_and_remove(&mut self, id: JobId) -> std::io::Result<()> {
        if let Some(job) = self.jobs.get(&id) {
            let err = job.kill();

            self.remove_job(id);

            err?
        }

        Ok(())
    }

    pub fn kill_all(&mut self) -> std::io::Result<()> {
        // Implementation details...
    }
}
```

### 2. `nu-system/src/foreground.rs`

This file contains the `ForegroundChild` implementation for managing external processes.

```rust
pub struct ForegroundChild {
    inner: Child,
    #[cfg(unix)]
    pipeline_state: Option<Arc<(AtomicU32, AtomicU32)>>,

    #[cfg(unix)]
    interactive: bool,
}

impl ForegroundChild {
    #[cfg(unix)]
    pub fn spawn(
        mut command: Command,
        interactive: bool,
        background: bool,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> io::Result<Self> {
        // Implementation details for process group management...
    }

    pub fn pid(&self) -> u32 {
        self.inner.id()
    }
}
```

### 3. `nu-system/src/util.rs`

Contains the `kill_by_pid` function used to terminate processes.

```rust
pub fn kill_by_pid(pid: i64) -> io::Result<()> {
    let mut cmd = build_kill_command(true, std::iter::once(pid), None);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(io::Error::other("failed to kill process"));
    }

    Ok(())
}

pub fn build_kill_command(
    force: bool,
    pids: impl Iterator<Item = i64>,
    signal: Option<u32>,
) -> CommandSys {
    // Platform-specific implementation for Windows and Unix
}
```

### 4. `nu-command/src/system/run_external.rs`

This file contains the implementation for running external commands and manages how child processes are spawned and tracked.

```rust
pub fn run(
    &self,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // ... (command setup code) ...

    // This is how child processes are registered with thread jobs
    if let Some(thread_job) = engine_state.current_thread_job() {
        if !thread_job.try_add_pid(child.pid()) {
            kill_by_pid(child.pid().into()).map_err(|err| {
                ShellError::Io(IoError::new_internal(
                    err.kind(),
                    "Could not spawn external stdin worker",
                    nu_protocol::location!(),
                ))
            })?;
        }
    }
    
    // ... (rest of implementation) ...
}
```

### 5. `nu-protocol/src/engine/engine_state.rs`

Contains the `EngineState` structure which holds all the global state including job management.

```rust
#[derive(Clone)]
pub struct EngineState {
    // ... (other fields) ...
    pub pipeline_externals_state: Arc<(AtomicU32, AtomicU32)>,
    pub jobs: Arc<Mutex<Jobs>>,
    pub current_job: CurrentJob,
    // ... (other fields) ...
}

impl EngineState {
    // ... (methods) ...
    
    pub fn current_thread_job(&self) -> Option<ThreadJob> {
        self.current_job.background_thread_job.clone()
    }
    
    // ... (other methods) ...
}
```

## How to Embed Nushell with Job Control

This example demonstrates how to properly embed Nushell with background job control capabilities. The key aspects are:

1. **Initialize Engine with Signal Handling**: Set up the Nushell engine and configure signal handling for interrupts
   - Use `nu_cmd_lang::create_default_context()` to create the engine state
   - Set up Ctrl-C handling with `ctrlc` crate and `Signals` from Nushell
   - Share the interrupt state via an `Arc<AtomicBool>`

2. **Job Management**: Create and track background jobs
   - Create `ThreadJob` instances with proper signal tracking
   - Add jobs to the engine's job table
   - Set the thread-local job context to enable child process tracking

3. **Script Execution**: Run Nushell scripts in background threads
   - Parse input with `nu_parser::parse`
   - Execute using `eval_block_with_early_return`
   - Properly handle results and cleanup when complete

4. **Signal Propagation**: Ensure signals propagate to child processes
   - Monitor for interrupt signals in the main thread
   - Kill jobs and their child processes when interrupted
   - Properly clean up job resources

5. **Process Management**: Track and terminate child processes
   - Register external processes with the current job context
   - Propagate termination signals to all child processes
   - Clean up resources when jobs complete or are terminated

This implementation includes tests that verify:
- Simple scripts execute and complete correctly
- External processes are properly tracked and terminated when interrupted
- Signal handling works correctly for job control

## Example Usage

Run a simple command that completes quickly:
```bash
cargo run -p p2-background -- '"Hello from a background task!" | str upcase'
```

Run a command involving a long-running external process:
```bash
cargo run -p p2-background -- "^sleep 20; 'Slept for 20 seconds'"
```
You can press `Ctrl+C` to interrupt this command. The embedded Nushell engine will attempt to terminate the `sleep` process.

## Testing

The tests verify simple script execution and, importantly, that external processes started by Nushell are correctly terminated when the main application receives an interrupt signal.

```bash
cargo test -p p2-background
```
This involves:
- A test for basic command execution.
- A test that spawns a `sleep` command via the embedded Nushell, sends a `SIGINT` to the `p2-background` process, and verifies that both the main process and the child `sleep` process terminate.