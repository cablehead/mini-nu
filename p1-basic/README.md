# Basic Nushell Embedding Example

This example demonstrates the core components needed to embed Nushell in a Rust application for single-execution workflows.

## Core Nushell Components

### 1. `EngineState` (`./crates/nu-protocol/src/engine/engine_state.rs`)

The central structure that holds all global state:

```rust
pub struct EngineState {
    files: Vec<CachedFile>,
    pub(super) virtual_paths: Vec<(String, VirtualPath)>,
    vars: Vec<Variable>,
    decls: Arc<Vec<Box<dyn Command + 'static>>>,
    pub(super) blocks: Arc<Vec<Arc<Block>>>,
    pub(super) modules: Arc<Vec<Arc<Module>>>,
    pub spans: Vec<Span>,
    doccomments: Doccomments,
    pub scope: ScopeFrame,
    signals: Signals,
    pub signal_handlers: Option<Handlers>,
    pub env_vars: Arc<EnvVars>,
    pub previous_env_vars: Arc<HashMap<String, Value>>,
    pub config: Arc<Config>,
    pub pipeline_externals_state: Arc<(AtomicU32, AtomicU32)>,
    pub repl_state: Arc<Mutex<ReplState>>,
    pub table_decl_id: Option<DeclId>,
    // ... additional fields ...
    pub jobs: Arc<Mutex<Jobs>>,
    pub current_job: CurrentJob,
    // ... more fields ...
}

impl EngineState {
    pub fn merge_delta(&mut self, delta: StateDelta) -> Result<(), ShellError>
    // ... other methods ...
}
```

### 2. `StateWorkingSet` (`./crates/nu-protocol/src/engine/state_working_set.rs`)

A staging area for changes to be applied to the engine state:

```rust
pub struct StateWorkingSet<'a> {
    pub permanent_state: &'a EngineState,
    pub delta: StateDelta,
    pub files: FileStack,
    pub search_predecls: bool,
    pub parse_errors: Vec<ParseError>,
    pub parse_warnings: Vec<ParseWarning>,
    pub compile_errors: Vec<CompileError>,
}

impl<'a> StateWorkingSet<'a> {
    pub fn new(permanent_state: &'a EngineState) -> Self
    pub fn render(self) -> StateDelta
    // ... other methods ...
}
```

### 3. `Stack` (`./crates/nu-protocol/src/engine/stack.rs`)

Manages runtime variable context during execution:

```rust
pub struct Stack {
    pub vars: Vec<(VarId, Value)>,
    pub env_vars: Vec<Arc<EnvVars>>,
    pub env_hidden: Arc<HashMap<String, HashSet<String>>>,
    pub active_overlays: Vec<String>,
    pub arguments: ArgumentStack,
    pub error_handlers: ErrorHandlerStack,
    pub recursion_count: u64,
    pub parent_stack: Option<Arc<Stack>>,
    pub parent_deletions: Vec<VarId>,
    pub config: Option<Arc<Config>>,
    pub(crate) out_dest: StackOutDest,
}

impl Stack {
    pub fn new() -> Self
    // ... other methods ...
}
```

### 4. `PipelineData` (`./crates/nu-protocol/src/pipeline/pipeline_data.rs`)

Represents data flowing through pipelines:

```rust
pub enum PipelineData {
    Empty,
    Value(Value, Option<PipelineMetadata>),
    ListStream(ListStream, Option<PipelineMetadata>),
    ByteStream(ByteStream, Option<PipelineMetadata>),
}

impl PipelineData {
    pub fn empty() -> PipelineData
    pub fn into_value(self, span: Span) -> Result<Value, ShellError>
    // ... other methods ...
}
```

### 5. `Value` (`./crates/nu-protocol/src/value/mod.rs`)

Core data type for Nushell values:

```rust
pub enum Value {
    Bool { val: bool, internal_span: Span },
    Int { val: i64, internal_span: Span },
    Float { val: f64, internal_span: Span },
    String { val: String, internal_span: Span },
    // ... many other variants ...
    List { vals: Vec<Value>, internal_span: Span },
    Closure { val: Box<Closure>, internal_span: Span },
    Error { error: Box<ShellError>, internal_span: Span },
    // ... more variants ...
}
```

## Embedding Workflow

The basic embedding pattern follows these steps, utilizing key Nushell functions:

### 1. Initialize Engine
Create an `EngineState` with default commands and shell capabilities.

**Key Functions:** `create_default_context` and `add_shell_command_context`
These functions create and configure the Nushell engine:
```rust
// From ./crates/nu-cmd-lang/src/default_context.rs
pub fn create_default_context() -> EngineState

// From nu_command
pub fn add_shell_command_context(engine_state: EngineState) -> EngineState
```
Usage:
```rust
// Create the base engine state with core commands
let mut engine_state = create_default_context();

// Add shell commands to interact with the OS
engine_state = add_shell_command_context(engine_state);
```

### 2. Setup Environment
Add environment variables from the host process to the `EngineState`.

**Key Function:** `gather_parent_env_vars`
Collects environment variables from the host process:
```rust
// From nu_cli
pub fn gather_parent_env_vars(engine_state: &mut EngineState, cwd: &Path)
```
Usage:
```rust
// Get the current working directory
let init_cwd = std::env::current_dir()?;

// Adds environment variables to the engine state
gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());
```

### 3. Parse Code
Use a `StateWorkingSet` and `nu_parser::parse` to convert Nushell code (a string slice) into an executable `Block`. The changes made during parsing (like new declarations or definitions) are then merged back into the `EngineState`.

**Key Function:** `nu_parser::parse`
Parses Nushell code into an executable block:
```rust
// From ./crates/nu-parser/src/parser.rs
pub fn parse(
    working_set: &mut StateWorkingSet,
    fname: Option<&str>,
    contents: &[u8],
    scoped: bool,
) -> Arc<Block>
```
Usage:
```rust
// Create a working set for parsing operations
let mut working_set = StateWorkingSet::new(&engine_state);

// Parse the code into an executable block
let block = nu_parser::parse(&mut working_set, None, code_snippet.as_bytes(), false);

// Merge the changes from parsing back into the engine state
engine_state.merge_delta(working_set.render())?;
```

### 4. Execute Code
Create a `Stack` for runtime variable context and use `nu_engine::eval_block_with_early_return` to run the parsed `Block`. This function handles Nushell's control flow (like `return`, `break`, `continue`) correctly.

**Key Function:** `eval_block_with_early_return`
Evaluates a code block with proper handling of Nushell's control flow:
```rust
// From ./crates/nu-engine/src/eval.rs
pub fn eval_block_with_early_return<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError>
```
Usage:
```rust
// Create a new stack for variable context
let mut stack = Stack::new();

// Evaluate the block with the engine state and stack
match eval_block_with_early_return::<WithoutDebug>(
    &engine_state,
    &mut stack,
    &block,
    PipelineData::empty(),
) {
    Ok(pipeline_data) => {
        // Handle successful execution...
    }
    Err(error) => {
        // Handle error...
    }
}
```

### 5. Process Results
Convert the `PipelineData` returned from evaluation into a Nushell `Value` (or stream) and handle the output as needed (e.g., print to console, pass to other parts of your application).

## Example Usage

```bash
# String manipulation
cargo run -p p1-basic -- '"Hello from Nushell!" | str upcase'

# Math operations
cargo run -p p1-basic -- "10 + 20 * 3"

# Data structures and transformations
cargo run -p p1-basic -- "[1 2 3] | each {|x| $x * 2} | math sum"

# File system operations
cargo run -p p1-basic -- "ls | where type == file | length"
```

## Testing

The tests verify that the embedded Nushell engine can correctly execute commands:

```bash
cargo test -p p1-basic
```