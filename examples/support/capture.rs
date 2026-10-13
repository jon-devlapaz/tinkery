//! Shared screen and reading captures for explicit evaluation runs. Never saves a session.
use std::path::Path;
use tinkery::think::brain_dump::{self, BrainDump};

pub(crate) fn capture(app: &mut BrainDump, dir: &Path, name: &str) {
    let paper = app.copy_text();
    std::fs::write(dir.join(format!("{name}-state.json")),serde_json::to_string_pretty(&serde_json::json!({"sources":app.conversation().sources(),"settled":app.conversation().settled(),"changed_lines":app.conversation().changed_lines(),"notice":app.notice()})).unwrap()+"\n").unwrap();
    std::fs::write(
        dir.join(format!("{name}-paper.json")),
        serde_json::to_string_pretty(&serde_json::json!({"paper":paper})).unwrap() + "\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.md")),
        paper
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_owned()
            + "\n",
    )
    .unwrap();
    if let Some(g) = app.conversation().board() {
        std::fs::write(
            dir.join(format!("{name}-guess.json")),
            serde_json::to_string_pretty(g).unwrap() + "\n",
        )
        .unwrap();
        let mut anchors = vec![];
        for (reading, framing) in g.framings.iter().enumerate() {
            for anchor in framing.supports.iter() {
                let range = anchor
                    .range(app.conversation().sources())
                    .expect("Accepted anchor must match original");
                anchors.push(serde_json::json!({"role":"support","reading":reading+1,"anchor":anchor,"start":range.start,"end":range.end}));
            }
        }
        for anchor in &g.misfits {
            let range = anchor
                .range(app.conversation().sources())
                .expect("Accepted anchor must match original");
            anchors.push(serde_json::json!({"role":"unresolved","anchor":anchor,"start":range.start,"end":range.end}));
        }
        std::fs::write(
            dir.join(format!("{name}-anchors.json")),
            serde_json::to_string_pretty(&anchors).unwrap() + "\n",
        )
        .unwrap();
    }
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let s = brain_dump::snapshot(w, h, app, false).unwrap();
        std::fs::write(
            dir.join(format!("{name}-{w}x{h}.json")),
            serde_json::to_string_pretty(
                &serde_json::json!({"width":w,"height":h,"lines":s.lines().collect::<Vec<_>>()}),
            )
            .unwrap()
                + "\n",
        )
        .unwrap();
    }
}
