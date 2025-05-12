// p4-sandbox/src/main.rs
//
// "Filters-only" Nushell sandbox (no externals, no FS/network commands).
//
// Usage example:
//   cargo run -p p4-sandbox -- '"hello" | wrap msg | length'
//   cargo run -p p4-sandbox -- '^ls'        # → exits with error
//
// ---------------------------------------------------------------------

use nu_engine::eval_block_with_early_return;
use nu_parser::parse;

use nu_protocol::ast::Block;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{format_shell_error, PipelineData, ShellError, Span, Value};
use std::sync::Arc;

/// Bootstrap a Nushell `EngineState` that exposes _only_ the "filters" command
/// collection listed at <https://www.nushell.sh/commands/categories/filters.html>.
fn create_filters_only_engine() -> Result<EngineState, Box<dyn std::error::Error>> {
    // 1. Create a minimal engine state with nothing pre-registered
    let mut engine_state = EngineState::new();

    // Unlike other examples, we don't use create_default_context()
    // This gives us complete control over which commands are available and
    // ensures no configs or environment variables are loaded.

    // 2. Register filter commands explicitly. Anything not added here is unavailable to scripts
    //    (including `run-external`, `open`, etc.).
    {
        // Import core filter commands
        use nu_cmd_lang::Collect;
        use nu_command::{
            Append, DropColumn, Each, Enumerate, Filter, Find, First, Flatten, Get, Last, Length,
            Prepend, Reject, Reverse, Select, Skip, Sort, Take, Uniq, Where, Wrap,
        };

        let delta = {
            let mut ws = StateWorkingSet::new(&engine_state);

            // -----------------------------------------------------------------
            //  Only commands explicitly registered here are available in the sandbox.
            //  We register a subset of the most useful filter commands.
            // -----------------------------------------------------------------
            ws.add_decl(Box::new(Append));
            ws.add_decl(Box::new(Collect));
            ws.add_decl(Box::new(DropColumn));
            ws.add_decl(Box::new(Each));
            ws.add_decl(Box::new(Enumerate));
            ws.add_decl(Box::new(Filter));
            ws.add_decl(Box::new(Find));
            ws.add_decl(Box::new(First));
            ws.add_decl(Box::new(Flatten));
            ws.add_decl(Box::new(Get));
            ws.add_decl(Box::new(Last));
            ws.add_decl(Box::new(Length));
            ws.add_decl(Box::new(Prepend));
            ws.add_decl(Box::new(Reject));
            ws.add_decl(Box::new(Reverse));
            ws.add_decl(Box::new(Select));
            ws.add_decl(Box::new(Skip));
            ws.add_decl(Box::new(Sort));
            ws.add_decl(Box::new(Take));
            ws.add_decl(Box::new(Uniq));
            ws.add_decl(Box::new(Where));
            ws.add_decl(Box::new(Wrap));
            // -----------------------------------------------------------------

            ws.render()
        };

        engine_state.merge_delta(delta)?;
    }

    // 3. Unlike other examples, we deliberately do not use:
    //    - create_default_context() to load the base engine
    //    - gather_parent_env_vars() to expose environment variables
    //
    // This creates a completely isolated sandbox with no access to the host
    // environment, filesystem, or network.

    Ok(engine_state)
}

/// Helper: pretty-print pipeline result for this demo CLI.
fn print_result(v: Value) {
    match v {
        Value::String { val, .. } => println!("{val}"),
        Value::List { vals, .. } => {
            for x in vals {
                println!("{x:?}");
            }
        }
        Value::Int { val, .. } => println!("{val}"),
        other => println!("{other:?}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Grab the script (single CLI arg).
    let script = std::env::args().nth(1).expect("No Nushell code given");

    // Boot the engine.
    let mut engine = create_filters_only_engine()?;

    // Parse the user script and surface any parse/compile errors early.
    let (block, working_set) = parse_checked_block(&engine, &script)?;
    engine.merge_delta(working_set.render())?;

    // Run.
    let mut stack = Stack::new();
    let out = eval_block_with_early_return::<WithoutDebug>(
        &engine,
        &mut stack,
        &block,
        PipelineData::empty(),
    )?
    .into_value(Span::unknown())?;

    print_result(out);
    Ok(())
}

/// Parses a Nushell script and handles parse or compile errors with helpful messages.
/// Returns a parsed Block and the associated working set on success, or a formatted ShellError on failure.
fn parse_checked_block<'a>(
    engine_state: &'a EngineState,
    code: &str,
) -> Result<(Arc<Block>, StateWorkingSet<'a>), Box<dyn std::error::Error>> {
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, code.as_bytes(), false);

    if let Some(err) = working_set.parse_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Parse error".into(),
            msg: err.to_string(), // Not Debug!
            span: Some(err.span()),
            help: Some("Is the command you're trying to use available in the sandbox?".into()),
            inner: vec![],
        };
        return Err(format_shell_error(&working_set, &shell_error).into());
    }

    if let Some(err) = working_set.compile_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Compile error".into(),
            msg: err.to_string(), // Again, not Debug
            span: None,           // CompileError doesn’t expose span
            help: Some("This may reference a command not available in this sandbox.".into()),
            inner: vec![],
        };
        return Err(format_shell_error(&working_set, &shell_error).into());
    }

    Ok((block, working_set))
}
