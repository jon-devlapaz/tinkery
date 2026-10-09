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

pub(super) const PLAN_EXAMPLE: &str =
    "I want a graph connecting decisions to work.\n\nSo I can find why we made those choices.";

pub(super) struct Investigation {
    pub sections: Vec<String>,
    pub label: &'static str,
}

pub(super) fn investigation(notes: &[(&str, &str)]) -> Investigation {
    let text = notes
        .iter()
        .map(|(_, text)| *text)
        .collect::<Vec<_>>()
        .join("\n\n");
    let matches = |example: &str| text.split_whitespace().eq(example.split_whitespace());
    let problem = matches(super::EXAMPLE);
    let plan = matches(PLAN_EXAMPLE);
    let sample = problem || plan;
    let (change, why, investigate, approaches) = if sample {
        (
            "Make the reasons behind important decisions accessible near the work they affect.",
            "Understand why a choice was made when revisiting the work.",
            "Determine what decision context people need and how to keep it accessible alongside their work.",
            if plan {
                "A graph connecting decisions to work is one candidate. We have not established that a graph is needed."
            } else {
                "No approach chosen. A graph, linked note, or simpler convention could be explored."
            },
        )
    } else {
        (
            "Not established yet. Separate the desired change from any proposed approach.",
            "Not established yet. What prompted these thoughts?",
            "Provisional: establish the desired outcome, boundaries, and which choices are open.",
            "Any approaches in these thoughts remain proposals here. This prototype accepts no decisions.",
        )
    };
    let thoughts = notes
        .iter()
        .map(|(title, text)| format!("### {title}\n\n{text}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    Investigation {
        label: if sample { "Working paper / sample" } else { "Working paper / template" },
        sections: vec![
            format!(
                "# Investigation draft\n\nSimulated {} / provisional. Not researched or accepted.",
                if sample { "sample" } else { "template; no interpretation generated" },
            ),
            format!("## What you want to change\n\n{change}"),
            format!("## Why it matters\n\n{why}"),
            format!("## What we should investigate\n\n{investigate}"),
            format!("## Possible approaches / not accepted\n\n{approaches}"),
            format!("## Your thoughts / unchanged\n\n{thoughts}"),
            "## Your turn\n\nWhat should we keep, cut, or reshape?\n\nEdit your notes and press F2 to reshape. Nothing is confirmed.".to_owned(),
        ],
    }
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
