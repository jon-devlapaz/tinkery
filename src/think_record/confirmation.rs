//! Caller-declared goal confirmation provenance. It records identity claims, not authenticated
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

/// Who the caller says confirmed this goal. No declaration means unrecorded, never operator.
/// This records goal confirmation only: neither seed confirmation nor implementation approval.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalConfirmation {
    /// Identity was not supplied, including records made before this field existed.
    Unrecorded {},
    /// The caller explicitly names the operator who confirmed on their own authority.
    Operator {
        /// The operator's declared name; never inferred from login or keyboard events.
        operator: String,
        /// The actual confirmation/authorization reference, unchanged.
        source: String,
        /// Both caller-declared risk axes and reasons.
        class: ConfirmationClass,
    },
    /// An agent acted for a named operator under the cited scoped authorization.
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

impl Default for GoalConfirmation {
    fn default() -> Self {
        Self::Unrecorded {}
    }
}

impl GoalConfirmation {
    /// Checks completeness and delegation eligibility, not whether the cited source grants authority.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let nonblank = |text: &str| !text.trim().is_empty();
        let class = match self {
            Self::Unrecorded {} => return Ok(()),
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

    /// True only for an explicitly declared agent; unrecorded identity is not operator identity.
    pub(crate) fn is_agent(&self) -> bool {
        matches!(self, Self::Agent { .. })
    }

    /// Escaped attribution for technical display; stored source words remain unchanged.
    pub(crate) fn details(&self) -> String {
        // Debug escaping keeps control/bidi characters out of display without changing metadata.
        format!("{self:?}")
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
