//! Caller-declared confirmation provenance and the one boundary that turns it into an [`Actor`].
//! It records identity claims, not authenticated identity or new authority; risk classification
//! never replaces scoped operator authorization. Shared code: it names no phase, and its serde
//! words are the stored format, so they never change.
use serde::{Deserialize, Serialize};

// The terminal declarations and actions `GoalConfirmation::validate` accepts, spelled once.
const DECLARED_HUMAN: &str = "declared: human";
const DECLARED_AGENT: &str = "declared: agent";
const NOT_DECLARED: &str = "not declared";
const TYPED_CONFIRM: &str = "typed confirm at this terminal";
const TEND_SAVE: &str = "pressed F2 to save the displayed Tend goal";
const UNVERIFIED: &str = "declaration is not verified identity";

/// Whether an action is cheap to undo, as the caller classified it.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    /// Hard to undo or starts real work; an agent cannot confirm it.
    OneWay,
    /// Cheap to undo before anything runs.
    TwoWay,
}

/// The caller's classification of the affected scope, not a verified measurement.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlastRadius {
    /// A bounded, isolated change.
    Small,
    /// Shared contracts, real operator data or another consequential boundary.
    Large,
}

/// Both risk axes and their reasons, retained as claims rather than execution permission.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationClass {
    /// Whether the action can be cheaply undone.
    pub door: Door,
    /// Why the caller chose that door.
    pub door_reason: String,
    /// How widely the action can affect things.
    pub blast_radius: BlastRadius,
    /// Why the caller chose that radius.
    pub blast_radius_reason: String,
}

impl ConfirmationClass {
    /// Both axes and their reasons as one escaped line, for display only.
    pub(crate) fn summary(&self) -> String {
        let door = match self.door {
            Door::OneWay => "one-way",
            Door::TwoWay => "two-way",
        };
        let radius = match self.blast_radius {
            BlastRadius::Small => "small",
            BlastRadius::Large => "large",
        };
        format!(
            "door: {door} ({}); blast radius: {radius} ({})",
            self.door_reason.escape_debug(),
            self.blast_radius_reason.escape_debug()
        )
    }
}

/// Who the caller says performed a confirmation. No declaration means unrecorded, never operator.
/// Think and Shape record this separately; supplying it for a goal never confirms a seed or approves work.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalConfirmation {
    /// Identity was not supplied, including records made before this field existed.
    Unrecorded {},
    /// A launch declaration and terminal action, never authenticated identity or delegation.
    Terminal {
        /// Exactly `declared: human`, `declared: agent`, or `not declared`.
        declaration: String,
        /// The confirmation action observed at this terminal, not who supplied its keys.
        action: String,
        /// Explicitly disclaims verified identity.
        identity: String,
    },
    /// The caller explicitly names the operator who confirmed on their own authority.
    Operator {
        /// The operator's declared name; never inferred from login or keyboard events.
        operator: String,
        /// The actual confirmation/authorization reference, unchanged.
        source: String,
        /// Both caller-declared risk axes and reasons.
        class: ConfirmationClass,
    },
    /// An agent named with its authorizing operator and cited authorization, as callers and older records supply.
    Agent {
        /// The acting agent's declared name, not the model that produced the reading.
        agent: String,
        /// The operator whose authorization the caller cites.
        authorizing_operator: String,
        /// The actual scoped delegation reference, unchanged; not proof by itself.
        source: String,
        /// Only two-way, small-radius actions are eligible for delegated confirmation.
        class: ConfirmationClass,
    },
}

/// Who may write, resolved once from explicit caller context before any durable action. Only
/// [`Actor::OPERATOR`] and [`GoalConfirmation::actor`] make one, so holding an `Actor` means the
/// context was validated; an agent is never the operator.
#[derive(Clone, Copy)]
pub struct Actor<'a>(Who<'a>);

#[derive(Clone, Copy)]
enum Who<'a> {
    /// The existing operator path; no named identity is inferred.
    Operator,
    /// An agent declared at launch, or named in an older record; never the operator.
    Agent(&'a GoalConfirmation),
}

impl<'a> Actor<'a> {
    /// The operator, for paths with no caller-declared context; no named identity is inferred.
    pub const OPERATOR: Actor<'static> = Actor(Who::Operator);

    /// Resolve optional caller context once at a phase entry: no context is the operator path.
    pub fn authorize(context: Option<&'a GoalConfirmation>) -> Result<Self, String> {
        context.map_or(Ok(Actor::OPERATOR), GoalConfirmation::actor)
    }

    /// Attribute an action without rewriting operator prose or claiming authenticated identity.
    pub(crate) fn reason(self, action: &str) -> String {
        match self.0 {
            Who::Operator => format!("The operator {action}"),
            Who::Agent(context) => {
                format!("The agent {action}; caller-declared {}", context.details())
            }
        }
    }

    /// The session mode for this actor, never inferred from a terminal declaration alone.
    pub(crate) fn session_operator(self) -> &'static str {
        match self.0 {
            Who::Operator => "human",
            Who::Agent(_) => "agent",
        }
    }

    /// The answer authority understood by the ledger.
    pub(crate) fn authority(self) -> &'static str {
        match self.0 {
            Who::Operator => "user",
            Who::Agent(_) => "delegated",
        }
    }

    /// Structured goal-origin delegation: the agent, plus any named context the caller supplied.
    pub(crate) fn delegation(self) -> Option<serde_json::Value> {
        match self.0 {
            Who::Agent(GoalConfirmation::Agent {
                agent,
                authorizing_operator,
                source,
                ..
            }) => Some(
                serde_json::json!({"agent":agent,"authorizing_operator":authorizing_operator,
                    "authorization_source":source,"door":"two-way","blast_radius":"small"}),
            ),
            Who::Agent(_) => Some(serde_json::json!({"agent":"agent"})),
            Who::Operator => None,
        }
    }

    /// The exact agent context; none on the operator path.
    pub(crate) fn context(self) -> Option<&'a GoalConfirmation> {
        match self.0 {
            Who::Operator => None,
            Who::Agent(context) => Some(context),
        }
    }
}

impl Default for GoalConfirmation {
    fn default() -> Self {
        Self::Unrecorded {}
    }
}

impl GoalConfirmation {
    /// Records a launch declaration only; an absent declaration stays unknown. An agent
    /// declaration makes every write the agent's; it is never verified identity.
    pub fn terminal(operator: Option<&str>) -> Result<Self, String> {
        let declaration = match operator {
            Some("human") => DECLARED_HUMAN,
            Some("agent") => DECLARED_AGENT,
            None => NOT_DECLARED,
            _ => return Err("--operator needs human or agent".into()),
        };
        Ok(Self::terminal_declaration(declaration))
    }

    fn terminal_declaration(declaration: &str) -> Self {
        Self::Terminal {
            declaration: declaration.into(),
            action: TYPED_CONFIRM.into(),
            identity: UNVERIFIED.into(),
        }
    }

    /// The same declaration, recorded as pressing F2 to save a displayed Tend goal. Named
    /// context is kept unchanged; only a terminal declaration names a terminal action.
    pub(crate) fn tend_save(self) -> Self {
        match self {
            Self::Terminal {
                declaration,
                identity,
                ..
            } => Self::Terminal {
                declaration,
                action: TEND_SAVE.into(),
                identity,
            },
            named => named,
        }
    }

    /// Checks completeness and delegation eligibility, not whether the cited source grants authority.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let nonblank = |text: &str| !text.trim().is_empty();
        let class = match self {
            Self::Unrecorded {} => return Ok(()),
            Self::Terminal {
                declaration,
                action,
                identity,
            } => {
                if ![DECLARED_HUMAN, DECLARED_AGENT, NOT_DECLARED].contains(&declaration.as_str())
                    || ![TYPED_CONFIRM, TEND_SAVE].contains(&action.as_str())
                    || identity != UNVERIFIED
                {
                    return Err("Invalid terminal confirmation declaration or action.".into());
                }
                return Ok(());
            }
            Self::Operator {
                operator,
                source,
                class,
            } => {
                if !nonblank(operator) || !nonblank(source) {
                    return Err("Confirmation needs the operator and actual source.".into());
                }
                class
            }
            Self::Agent {
                agent,
                authorizing_operator,
                source,
                class,
            } => {
                if !nonblank(agent) || !nonblank(authorizing_operator) || !nonblank(source) {
                    return Err("Agent confirmation needs the agent, authorizing operator and scoped source.".into());
                }
                if class.door != Door::TwoWay || class.blast_radius != BlastRadius::Small {
                    return Err(
                        "An agent may confirm only a two-way door with small blast radius.".into(),
                    );
                }
                class
            }
        };
        if !nonblank(&class.door_reason) || !nonblank(&class.blast_radius_reason) {
            return Err("Confirmation classification needs a reason for each risk axis.".into());
        }
        Ok(())
    }

    /// Resolve who writes before any durable action. An agent declaration writes as the agent,
    /// never as the operator; no declaration keeps the operator path. Invalid context refuses.
    pub fn actor(&self) -> Result<Actor<'_>, String> {
        self.validate()?;
        match self {
            Self::Agent { .. } => Ok(Actor(Who::Agent(self))),
            Self::Terminal { declaration, .. } if declaration == DECLARED_AGENT => {
                Ok(Actor(Who::Agent(self)))
            }
            _ => Ok(Actor::OPERATOR),
        }
    }

    /// Escaped attribution for technical display; stored source words remain unchanged.
    pub(crate) fn details(&self) -> String {
        if let Self::Terminal {
            declaration,
            action,
            identity,
        } = self
        {
            return format!("{declaration}; {action}; {identity}");
        }
        let (who, source, class) = match self {
            Self::Unrecorded {} => return "unrecorded".into(),
            Self::Operator {
                operator,
                source,
                class,
            } => (
                format!("operator {}", operator.escape_debug()),
                source,
                class,
            ),
            Self::Agent {
                agent,
                authorizing_operator,
                source,
                class,
            } => (
                format!(
                    "agent {}; authorized by operator {}",
                    agent.escape_debug(),
                    authorizing_operator.escape_debug()
                ),
                source,
                class,
            ),
            Self::Terminal { .. } => return "unrecorded".into(),
        };
        format!(
            "{who}; source: {}; {}",
            source.escape_debug(),
            class.summary()
        )
    }

    /// A goal-only receipt that never turns an unknown or agent keypress into the operator's.
    pub(crate) fn source(&self) -> String {
        let attribution = if matches!(self, Self::Unrecorded {}) {
            "unrecorded".to_owned()
        } else {
            self.details()
        };
        format!(
            "Goal confirmed in Tinkery with typed confirm and Enter; caller-declared attribution: {attribution}; \
             goal only, not seed or implementation approval."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tend_save_keeps_the_declaration_and_stays_valid_and_never_claims_a_human() {
        for (operator, declaration, writes_as) in [
            (Some("agent"), "declared: agent", "agent"),
            (Some("human"), "declared: human", "human"),
            (None, "not declared", "human"),
        ] {
            let saved = GoalConfirmation::terminal(operator).unwrap().tend_save();
            assert_eq!(
                serde_json::to_value(&saved).unwrap(),
                serde_json::json!({"by": "terminal", "declaration": declaration,
                    "action": "pressed F2 to save the displayed Tend goal",
                    "identity": "declaration is not verified identity"})
            );
            assert_eq!(saved.actor().unwrap().session_operator(), writes_as);
        }
        let named = GoalConfirmation::Agent {
            agent: "TEST-agent".into(),
            authorizing_operator: "TEST-operator".into(),
            source: "TEST scoped reference".into(),
            class: ConfirmationClass {
                door: Door::TwoWay,
                door_reason: "TEST reversible".into(),
                blast_radius: BlastRadius::Small,
                blast_radius_reason: "TEST isolated".into(),
            },
        };
        assert_eq!(named.clone().tend_save(), named);
    }
}
