//! Who answers a reading request: [`BoardHost`] is the boundary, met by Pi (a shaping call, then
//! the decision review) or by the practice simulation.
use super::*;
use crate::think::record;

/// Answers a reading request. Runs off the paint thread; it must stop promptly once
/// `cancelled` is set, and it returns a decoded but unverified [`Guess`] or one line saying
/// why there isn't one.
pub trait BoardHost: Send + Sync {
    /// Reads `request` into a guess. A real host may make provider calls; the practice host
    /// makes none.
    fn reshape(&self, request: BoardRequest, cancelled: &AtomicBool) -> Result<Guess, String>;
    /// The host's own log lines for F5 "why"; none by default.
    fn diagnostics(&self) -> Vec<String> {
        Vec::new()
    }
    /// Who this host is, for the think record. A host that names no model is `simulated`.
    fn origin(&self) -> record::Origin {
        record::Origin::simulated()
    }
}
impl BoardHost for PiHost {
    fn origin(&self) -> record::Origin {
        record::Origin {
            origin: "live".into(),
            model: Some(self.model.clone()),
            thinking: (!self.thinking.is_empty()).then(|| self.thinking.clone()),
            prompt_hash: Some(record::hash(&format!(
                "{}\n{}",
                reading::prompt(self.working_instructions()),
                reading::assemble(review::PROMPT)
            ))),
        }
    }
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
