use super::rendering::*;
use super::support::*;
use super::validation::*;
use super::*;

pub fn validate(pattern: &RegexPattern) -> DobraResult<()> {
    validate_flags(&pattern.flags, "regex")?;
    let mut named_groups = HashSet::new();
    validate_sequence(&pattern.body, &mut named_groups)
}

pub fn validate_for_target(pattern: &RegexPattern, target: RegexTarget) -> DobraResult<()> {
    validate(pattern)?;
    validate_target_sequence(&pattern.body, target)?;

    if pattern.flags.contains(&RegexFlag::Ungreedy)
        && matches!(
            target,
            RegexTarget::Javascript | RegexTarget::Python | RegexTarget::DotNet | RegexTarget::Re2
        )
    {
        return Err(regex_error(format!(
            "flag '{}' is not supported by {}",
            RegexFlag::Ungreedy.name(),
            target.name()
        )));
    }

    Ok(())
}

pub fn render(pattern: &RegexPattern) -> DobraResult<String> {
    render_for_target(pattern, RegexTarget::Classic)
}

pub fn render_for_target(pattern: &RegexPattern, target: RegexTarget) -> DobraResult<String> {
    validate_for_target(pattern, target)?;
    let mut out = String::new();
    if !pattern.flags.is_empty() {
        out.push_str(&render_global_flags(&pattern.flags));
    }
    out.push_str(&render_sequence(&pattern.body)?);
    Ok(out)
}

pub fn compile(pattern: &RegexPattern) -> DobraResult<RuntimeRegex> {
    let rendered = render(pattern)?;
    compile_text(&rendered)
}

pub fn compile_text(rendered: &str) -> DobraResult<RuntimeRegex> {
    let engine = Regex::new(rendered)
        .map_err(|err| DobraError::runtime(format!("cannot compile regex '{rendered}': {err}")))?;
    Ok(RuntimeRegex {
        rendered: rendered.to_string(),
        engine: Rc::new(engine),
    })
}
