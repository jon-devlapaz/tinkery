pub(super) fn sections(note: &str) -> Vec<String> {
    let example = note
        .split_whitespace()
        .eq(super::EXAMPLE.split_whitespace());
    let (goal, outcome, question) = if example {
        (
            "Keep the reasons behind important decisions close to the work they affect.".to_owned(),
            "Open a work item and see what was chosen, why, and which evidence informed it.",
            "Which decisions are worth keeping close to the work?",
        )
    } else {
        (
            format!(
                "Explore a small, useful version of this idea:\n\n{}",
                note.trim()
            ),
            "One concrete change, with a clear way to tell whether it helped. The details still need shaping.",
            "What would be different for the person this helps?",
        )
    };
    vec![
        "# A first shape".to_owned(),
        format!("## Tentative goal\n\n{goal}"),
        format!("## Possible outcome\n\n{outcome}"),
        format!("## One open question\n\n{question}"),
    ]
}

pub(super) fn source_label(note: Option<&str>) -> &'static str {
    match note {
        Some(note)
            if note
                .split_whitespace()
                .eq(super::EXAMPLE.split_whitespace()) =>
        {
            "Working paper / sample"
        }
        Some(_) => "Working paper / template",
        None => "Working paper",
    }
}
