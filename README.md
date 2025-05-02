# Mini-Nu

A collection of minimal examples showing how to embed Nushell in your Rust applications.

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

### Backround
