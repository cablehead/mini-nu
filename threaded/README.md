# Threaded Nushell Embedding Example

This package demonstrates how to share a Nushell engine across multiple threads, enabling concurrent command execution with isolated environments.

## Overview

https://github.com/user-attachments/assets/b4de1d3c-88fb-4c66-a620-bac45c0359fb

The threaded example showcases:
- Creating and preparing a Nushell `EngineState` with a specific configuration
- Cloning the engine state across threads (similar to forking)
- Executing Nushell closures in separate thread contexts
- Independently evaluating commands in parallel

## Usage

The example reads lines from stdin and executes a user-provided closure for each line in a separate thread:

```bash
echo -e "line1\nline2\nline3" | cargo run -p threaded -- '{|i| $"Processing ($in) with job id ($i)"}'
```

A more interesting example with variable sleep times:

```bash
echo -e "300\n200\n100" | cargo run -p threaded -- '{|i| let dur = $in + "ms"; sleep ($dur | into duration); $"finished job ($i) after ($dur)" }'
```

Output:
```
Thread 0 starting execution
Thread 1 starting execution
Waiting for all tasks to complete...
Thread 2 starting execution
Thread 2: finished job 2 after 100ms
Thread 1: finished job 1 after 200ms
Thread 0: finished job 0 after 300ms
All tasks completed. Exiting.
```

Note how the jobs finish in order of their sleep duration, not in the order they were submitted, demonstrating true concurrent execution with independent Nushell environments.

## Nushell Implementation Details

Key Nushell-specific components:

1. **EngineState Preparation**: 
   - A single `EngineState` is prepared with commands, environment variables, and custom functions
   - [EngineState](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.EngineState.html)

2. **Engine Cloning & Distribution**:
   - The prepared `EngineState` is cloned for each thread
   - Each thread gets its own isolated copy to work with
   - [EngineState clone](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.EngineState.html#impl-Clone-for-EngineState)

3. **Closure Parsing**:
   - User-provided Nushell closure is parsed once
   - The parsed closure is shared across thread executions
   - [Closure](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Closure.html)

4. **Independent Stack Contexts**:
   - Each thread creates its own `Stack` for variable context
   - Thread-specific arguments are passed to the closure
   - [Stack](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.Stack.html)

5. **Custom Command Registration**:
   - Demonstrates adding a custom `warble` command to the engine
   - Uses `StateWorkingSet` to extend the engine with new commands
   - [StateWorkingSet](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.StateWorkingSet.html)

This architecture showcases how Nushell's design allows for efficient engine reuse across concurrent contexts, with each thread maintaining its own execution state.

## Testing

Run the tests with:

```bash
cargo test -p threaded
```

The test verifies that separate Nushell environments can execute concurrently, completing tasks in order of execution time rather than submission order.