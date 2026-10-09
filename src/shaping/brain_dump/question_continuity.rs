use super::*;

pub(super) fn validate(questions: &[&str], repeated: &[String]) -> Result<(), String> {
    let mut seen = HashSet::new();
    if repeated
        .iter()
        .any(|id| !seen.insert(id) || !questions.contains(&id.as_str()))
    {
        return Err("Invalid advisory question reference; displayed reading unchanged.".into());
    }
    Ok(())
}
