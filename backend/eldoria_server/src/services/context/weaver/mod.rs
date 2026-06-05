use weaver_lang::Value;
use weaver_macros::weaver_command;

#[weaver_command(name = "var_exists")]
fn is_var(key: String, ctx: &mut dyn EvalContext) -> Result<Option<Value>, EvalError> {
    let mid = key.chars().position(|c| c == ':');
    let exists = if let Some(idx) = mid {
        let (namespace, variable) = key.split_at(idx);
        let namespace = namespace.replace("{{", "");
        let variable = variable.replace("}}", "");
        let maybe_exists = ctx.resolve_variable(&namespace, &variable);
        if let Ok(exists) = maybe_exists {
            exists.is_some()
        } else {
            false
        }
    } else {
        false
    };

    Ok(Some(Value::Bool(exists)))
}
