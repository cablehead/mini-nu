### p4-sandbox

## "Filters-only" Nushell sandbox with no external commands

This example shows how to create a restricted Nushell environment that only
allows a specific subset of commands (filters), blocking access to external
commands and filesystem operations. This is useful for creating sandboxed
environments where you want to provide Nushell's data processing capabilities
without giving access to the host system.

---

## TL;DR

```
# From the repo root
cargo run -p p4-sandbox -- '"hello" | wrap msg | length'
# → 1

# Trying to run external commands fails
cargo run -p p4-sandbox -- '^ls'
# → Error: External commands are not supported
```

_(Full source in [`src/main.rs`](./src/main.rs).)_

---

## What this example adds

| Capability                         | Where it happens                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **Selective command registration** | `create_filters_only_engine()` explicitly registers a useful subset of filter commands like `Length`, `Wrap`, etc.             |
| **No external command access**     | By not registering the `run-external` command and other OS-specific commands, external processes like `^ls` cannot be launched |
| **Safe data processing**           | All filter operations (like `wrap`, `length`, `where`, etc.) work normally, without any ability to access the filesystem       |
| **Explicit command enumeration**   | Each command is individually registered via `ws.add_decl(Box::new(...))`, giving precise control over available functionality  |
| **Complete isolation**             | Engine is built manually without using `create_default_context()`, preventing any default commands or user config loading      |
| **Environment isolation**          | Unlike other examples, we don't call `gather_parent_env_vars()`, preventing environment variable access and leakage            |

---

## Minimal walkthrough

```rust
// 1. Create a completely empty engine state
let mut engine_state = EngineState::new();
// Note: We deliberately don't call create_default_context() for maximum isolation

// 2. Register ONLY the filter commands we want to expose
{
    use nu_command::{Each, Filter, Length, Wrap, Sort, Where /* etc */};

    let delta = {
        let mut ws = StateWorkingSet::new(&engine_state);

        // Each command is explicitly registered
        ws.add_decl(Box::new(Length));
        ws.add_decl(Box::new(Wrap));
        // ... (and more filter commands)

        ws.render()
    };

    engine_state.merge_delta(delta)?;
}

// 3. Parse and run user code (with only allowed commands available)
```

External commands like `^ls` are blocked because `run-external` is never
registered — we omit `add_shell_command_context()`, which is where OS-related
commands normally come from.

_(Parsing and execution follow the same pattern as in
[`p1-basic`](../p1-basic/README.md); see that example for full details.)_

---

## Try these scripts

| Purpose                   | Command to pass                                 |
| ------------------------- | ----------------------------------------------- |
| Basic data transformation | `'"hello" \| wrap message \| get message'`      |
| List manipulation         | `'[1 2 3] \| append 4 \| prepend 0'`            |
| Data filtering            | `'[1 2 3 4 5] \| where { $it > 3 }'`            |
| Attempt external command  | `'^ls'` (will fail - external commands blocked) |

---

## Tests (optional)

```
cargo test -p p4-sandbox
```

- Tests verify that:
  1. External commands like `^ls` are properly blocked
  2. Regular filter commands like `length` still work correctly

---

## Need a refresher?

- [← Back to `p3-the-works`](../p3-the-works/README.md)
- [← Back to `p2-background`](../p2-background/README.md)
- [← Back to `p1-basic`](../p1-basic/README.md)

---

## Further reading

- **Nushell Filter Commands** — commands that transform data.
  ([Nushell](https://www.nushell.sh/commands/categories/filters.html))
- **Security in Sandboxed Environments** — principles for building restricted
  execution environments.
  ([OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Sandboxed_Environments.html))
- **Selective Command Registration** — design patterns for command access
  control.
  ([Security Design Patterns](https://en.wikipedia.org/wiki/Secure_by_design))
