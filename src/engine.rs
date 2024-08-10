use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Call, Closure};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
struct Warble;

impl Command for Warble {
    fn name(&self) -> &str {
        "warble"
    }

    fn signature(&self) -> Signature {
        Signature::build("warble")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
        "Returns the string 'warble'"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::Value(
            Value::string("warble, oh my", Span::unknown()),
            None,
        ))
    }
}

fn add_custom_commands(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(Warble));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error adding custom commands: {err:?}");
    }

    engine_state
}

pub fn create() -> Result<EngineState, Box<dyn std::error::Error>> {
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    engine_state = add_cli_context(engine_state);
    engine_state = add_custom_commands(engine_state);

    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    Ok(engine_state)
}

pub fn parse_closure(
    engine_state: &mut EngineState,
    closure_snippet: &str,
) -> Result<Closure, ShellError> {
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, closure_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result =
        eval_block::<WithoutDebug>(engine_state, &mut stack, &block, PipelineData::empty())?;
    result.into_value(Span::unknown())?.into_closure()
}
