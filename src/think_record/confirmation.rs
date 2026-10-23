//! Caller-declared confirmation provenance. It records identity claims, not authenticated
//! identity or new authority; risk classification never replaces scoped operator authorization.
use serde::{Deserialize, Serialize};

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

/// The actor permitted to write, resolved only from explicit caller context.
#[derive(Clone, Copy)]
pub(crate) enum Actor<'a> {
    /// The existing operator path; no named identity is inferred.
    Operator,
    /// An agent declared at launch, or named in an older record; never the operator.
    Agent(&'a GoalConfirmation),
}

impl<'a> Actor<'a> {
    /// The one authorization boundary shared by goal saving and every Shape write route.
    pub(crate) fn authorize(context: Option<&'a GoalConfirmation>) -> Result<Self, String> {
        context.map_or(Ok(Self::Operator), GoalConfirmation::actor)
    }

    /// Attribute an action without rewriting operator prose or claiming authenticated identity.
    pub(crate) fn reason(self, action: &str) -> String {
        match self {
            Self::Operator => format!("The operator {action}"),
            Self::Agent(context) => {
                format!("The agent {action}; caller-declared {}", context.details())
            }
        }
    }

    /// The session mode for this actor, never inferred from a terminal declaration alone.
    pub(crate) fn session_operator(self) -> &'static str {
        match self {
            Self::Operator => "human",
            Self::Agent(_) => "agent",
        }
    }

    /// The answer authority understood by the ledger.
    pub(crate) fn authority(self) -> &'static str {
        match self {
            Self::Operator => "user",
            Self::Agent(_) => "delegated",
        }
    }

    /// Structured goal-origin delegation: the agent, plus any named context the caller supplied.
    pub(crate) fn delegation(self) -> Option<serde_json::Value> {
        match self {
            Self::Agent(GoalConfirmation::Agent {
                agent,
                authorizing_operator,
                source,
                ..
            }) => Some(
                serde_json::json!({"agent":agent,"authorizing_operator":authorizing_operator,
                    "authorization_source":source,"door":"two-way","blast_radius":"small"}),
            ),
            Self::Agent(_) => Some(serde_json::json!({"agent":"agent"})),
            Self::Operator => None,
        }
    }

    /// The exact named agent context, absent on legacy operator paths.
    pub(crate) fn context(self) -> Option<&'a GoalConfirmation> {
        match self {
            Self::Operator => None,
            Self::Agent(context) => Some(context),
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
            Some("human") => "declared: human",
            Some("agent") => "declared: agent",
            None => "not declared",
            _ => return Err("--operator needs human or agent".into()),
        };
        Ok(Self::terminal_declaration(declaration))
    }

    fn terminal_declaration(declaration: &str) -> Self {
        Self::Terminal {
            declaration: declaration.into(),
            action: "typed confirm at this terminal".into(),
            identity: "declaration is not verified identity".into(),
        }
    }

    /// Keep launch kind across a fresh visit; named context from older records becomes the plain declaration.
    pub(crate) fn fresh_declaration(&self) -> Self {
        match self {
            Self::Terminal { .. } => self.clone(),
            Self::Agent { .. } => Self::terminal_declaration("declared: agent"),
            _ => Self::default(),
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
                if !["declared: human", "declared: agent", "not declared"]
                    .contains(&declaration.as_str())
                    || action != "typed confirm at this terminal"
                    || identity != "declaration is not verified identity"
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
    /// never as the operator; no declaration keeps the operator path.
    pub(crate) fn actor(&self) -> Result<Actor<'_>, String> {
        self.validate()?;
        match self {
            Self::Agent { .. } => Ok(Actor::Agent(self)),
            Self::Terminal { declaration, .. } if declaration == "declared: agent" => {
                Ok(Actor::Agent(self))
            }
            _ => Ok(Actor::Operator),
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
        let door = match class.door {
            Door::OneWay => "one-way",
            Door::TwoWay => "two-way",
        };
        let radius = match class.blast_radius {
            BlastRadius::Small => "small",
            BlastRadius::Large => "large",
        };
        format!(
            "{who}; source: {}; door: {door} ({}); blast radius: {radius} ({})",
            source.escape_debug(),
            class.door_reason.escape_debug(),
            class.blast_radius_reason.escape_debug()
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
