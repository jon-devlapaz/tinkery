//! Who answers a reading request: [`BoardHost`] is the boundary, met by Pi (a shaping call, then
//! the decision review) or by the practice simulation.
use super::*;

pub trait BoardHost: Send + Sync {
    fn reshape(&self, request: BoardRequest, cancelled: &AtomicBool) -> Result<Guess, String>;
    fn diagnostics(&self) -> Vec<String> {
        Vec::new()
    }
}
impl BoardHost for PiHost {
    fn diagnostics(&self) -> Vec<String> {
        self.brain_history
            .try_lock()
            .map(|h| h.clone())
            .unwrap_or_default()
    }
    fn reshape(&self, request: BoardRequest, cancelled: &AtomicBool) -> Result<Guess, String> {
        let started = std::time::Instant::now();
        let prompt = reading::prompt(self.working_instructions());
        let mut decision = checks::Decision::new(
            "structure-spans-history",
            "blocking",
            "pass",
            "Readable board, exact claimed spans and application-owned history. Not a meaning judgment."
                .into(),
            started,
        );
        decision.request_key = checks::request_key(&request);
        let result: Result<Guess, String> = (|| {
            let input = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
            let text = self.complete(input, prompt, cancelled)?;
            let text = self
                .review_decisions(&request, &text, cancelled, &mut decision.normalizations)
                .unwrap_or(text);
            decision.raw = Some(text.clone());
            let guess = Guess::decode(&text, &request, &mut decision.normalizations)?;
            decision.candidate = Some(guess.clone());
            decision.coverage = checks::coverage(&request, &guess);
            Ok(guess)
        })();
        if let Err(e) = &result {
            decision.decision = "reject".into();
            decision.reason = e.clone();
        }
        decision.elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        checks::log(self, &decision);
        result
    }
}
pub(super) struct Simulated;
impl BoardHost for Simulated {
    fn reshape(&self, request: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        Ok(Guess {
            parts: GoalParts {
                who: Some("I".into()),
                outcome: Some("try Simulated sensemaking (not inferred)".into()),
                when: Some("in this practice".into()),
                why: Some("I can try without saving".into()),
                done_when: Some("this is a placeholder, not an inferred criterion".into()),
                must: vec![],
                must_not: vec!["confirming anything".into()],
            },
            other_goals: vec![],
            deferred: vec![],
            open: vec![],
            unresolved_notes: vec![],
            uncertain: false,
            framings: vec![Framing {
                text: String::new(),
                supports: request
                    .sources
                    .first()
                    .map(|s| Anchor {
                        source: s.id,
                        quote: s.text.clone(),
                        occurrence: 0,
                    })
                    .into_iter()
                    .collect(),
            }],
            outcome: String::new(),
            misfits: vec![],
            questions: if request.answered.contains(&"priority".into())
                || request.skipped.iter().any(|q| q.id == "priority")
            {
                vec![]
            } else {
                vec![Question {
                    target: None,
                    id: "priority".into(),
                    text: "Simulated: which part matters most to you?".into(),
                }]
            },
            alternatives: [
                "Investigate the proposed approach",
                "Investigate a different route to the same experience",
            ]
            .into_iter()
            .map(|label| Approach {
                label: format!("Simulated: {label}"),
                benefit: "Placeholder, not an evaluated approach.".into(),
                cost: "Not evaluated.".into(),
                undo_cost: "Unknown.".into(),
            })
            .collect(),
        })
    }
}
