# Basic Nushell Embedding Example

This package demonstrates the minimal code needed to embed Nushell in a Rust application, execute commands, and handle results.

## Overview

The basic example shows how to:
- Initialize a Nushell `EngineState` with default commands
- Parse Nushell code into an executable block
- Evaluate the block and process results
- Handle different Nushell value types in your Rust code

## Usage

Run the example with a Nushell command as an argument:

```bash
cargo run -p basic -- '"Hello from Nushell!" | str upcase'
```

Output:
```
HELLO FROM NUSHELL!
```

## Example Commands

Here are some other commands you can try:

```bash
# Simple math
cargo run -p basic -- "10 + 20 * 3"

# Working with data structures
cargo run -p basic -- "[1 2 3] | each {|x| $x * 2} | math sum"

# Using Nushell's built-in commands
cargo run -p basic -- "ls | where type == file | length"
```

## Nushell Implementation Details

Key Nushell components used:

1. **Engine State Creation**:
   - Uses `create_default_context()` to set up a base engine
   - Adds shell commands with `add_shell_command_context()`
   - [EngineState](https://docs.rs/nu-protocol/latest/nu_protocol/engine/struct.EngineState.html)

2. **Environment Setup**:
   - Collects parent environment variables with `gather_parent_env_vars`
   - [gather_parent_env_vars](https://docs.rs/nu-cli/latest/nu_cli/fn.gather_parent_env_vars.html)

3. **Code Parsing**:
   - Creates a `StateWorkingSet` for parsing operations
   - Parses code into an executable block with `parse()`
   - [parse](https://docs.rs/nu-parser/latest/nu_parser/fn.parse.html)

4. **Block Evaluation**:
   - Uses `eval_block_with_early_return` to execute Nushell code
   - Handles early returns and error states
   - [eval_block_with_early_return](https://docs.rs/nu-engine/latest/nu_engine/fn.eval_block_with_early_return.html)

5. **Result Handling**:
   - Processes Nushell's `PipelineData` into `Value` types
   - Demonstrates handling different value types (String, List, etc.)
   - [PipelineData](https://docs.rs/nu-protocol/latest/nu_protocol/enum.PipelineData.html), [Value](https://docs.rs/nu-protocol/latest/nu_protocol/enum.Value.html)

This implementation demonstrates the simplest pattern for embedding Nushell, focusing on single-execution flows rather than reusing the engine across multiple evaluations.

## Testing

Run the test with:

```bash
cargo test -p basic
```

The test verifies that the example can correctly execute a Nushell string transformation command.