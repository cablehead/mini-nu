# Mini-Nu

A collection of minimal examples showing how to embed Nushell in your Rust applications. These examples progressively introduce more advanced embedding features.

## Examples Overview

1.  **[p1-basic](./p1-basic/README.md)**: Demonstrates the fundamental steps to embed Nushell for executing single commands. This is the simplest starting point.
2.  **[p2-background](./p2-background/README.md)**: Builds upon `p1-basic` by adding support for running a single Nushell script in a background thread with proper job control and signal handling (Ctrl+C). This allows for interruptible long-running tasks.
3.  **[p3-the-works](./p3-the-works/README.md)**: The most advanced example, showcasing how to define custom Nushell commands, parse and execute Nushell closures, manage multiple concurrent background jobs (e.g., processing lines from stdin), and ensure robust shutdown with `tokio`.

## Running and Testing

You can run and test each example individually using Cargo.

### p1-basic

*   **Run:**
    ```bash
    cargo run -p p1-basic -- '"Hello from Nushell!" | str upcase'
    cargo run -p p1-basic -- "ls | where type == file | length"
    ```
*   **Test:**
    ```bash
    cargo test -p p1-basic
    ```
*   **Details:** [p1-basic/README.md](./p1-basic/README.md)

### p2-background

*   **Run a simple command:**
    ```bash
    cargo run -p p2-background -- '"A background task says Hello!" | str upcase'
    ```
*   **Run a command with an external process (e.g., sleep):**
    ```bash
    cargo run -p p2-background -- "^sleep 10; 'Slept for 10s'"
    ```
    (Press Ctrl+C to interrupt)
*   **Test:**
    ```bash
    cargo test -p p2-background
    ```
*   **Details:** [p2-background/README.md](./p2-background/README.md)

### p3-the-works

*   **Run with a closure processing stdin lines:**
    ```bash
    # Start the application, then type lines into stdin.
    # Each line will be processed by the closure.
    # Press Ctrl+D to close stdin, or Ctrl+C to interrupt.
    cargo run -p p3-the-works -- '{|line_num| $"Input ({$line_num}): ($in) processed!" }'
    ```
    Example input after running:
    ```
    hello
    world
    ```
*   **Run with a closure calling a custom command:**
    ```bash
    cargo run -p p3-the-works -- '{|_| warble | str upcase }'
    ```
    Example input after running:
    ```
    trigger
    ```
*   **Test:**
    ```bash
    cargo test -p p3-the-works
    ```
*   **Details:** [p3-the-works/README.md](./p3-the-works/README.md)

## Acknowledgements

An early example from [@sophiajt](https://github.com/sophiajt) herself: https://github.com/sophiajt/nu_app
