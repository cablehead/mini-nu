# Mini-Nu

A collection of minimal examples showing how to embed Nushell in your Rust applications.

## Overview

This repository contains multiple examples demonstrating different ways to use Nushell as an embedded engine in Rust applications. Each example is organized as a separate package in a workspace structure.

## Examples

### Basic

The simplest example showing how to execute Nushell commands from a Rust application:

```bash
cargo run -p basic -- '"Hello, world!" | str upcase'
```

Output:
```
HELLO, WORLD!
```

### Threaded

A more advanced example that runs Nushell's engine standalone across multiple threads. For each line of input, it spawns a new thread to execute the user-provided closure, passing an incrementing count as an argument.

https://github.com/user-attachments/assets/b4de1d3c-88fb-4c66-a620-bac45c0359fb

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

This demonstrates how the jobs complete in order of their sleep duration, not in the order they were submitted.

## Running Tests

Each package includes integration tests that verify its functionality:

```bash
# Run all tests
cargo test

# Test a specific package
cargo test -p basic
cargo test -p threaded
```

## Acknowledgements

- An early example from [@sophiajt](https://github.com/sophiajt) herself: https://github.com/sophiajt/nu_app