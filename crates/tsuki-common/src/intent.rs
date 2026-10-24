use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionTemplate {
    ThrottleMemory,
    ReweightCpu,
    FreezeCgroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionState {
    Proposed,
    Validated,
    AwaitingApproval,
    Applied,
    Observing,
    Committed,
    RolledBack,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationIntent {
    pub intent_id: String,
    pub template: ActionTemplate,
    pub target_cgroup: String,
    pub params: HashMap<String, serde_json::Value>,
    pub rationale: String,
    pub evidence_ids: Vec<String>,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub action_id: String,
    pub intent: RemediationIntent,
    pub state: ActionState,
    pub prior_values: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rollback_reason: Option<String>,
}
