use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionMode {
    Manual,
    AutoPreTurn,
    AutoMidTurn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionReason {
    ExplicitRequest,
    WindowPressure,
    ContinuationPressure,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionResult {
    Success,
    Retry,
    Degraded,
    Failure,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionSkipReason {
    UnderThreshold,
    EmptySummarizeRegion,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionRequestMetadata {
    pub mode: CompactionMode,
    pub trigger: CompactionTrigger,
    pub reason: CompactionReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedCompactionOutcome {
    pub request: CompactionRequestMetadata,
    pub input_prompt_tokens: usize,
    pub output_prompt_tokens: usize,
    pub retry_count: u32,
    pub result: CompactionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailedCompactionOutcome {
    pub request: CompactionRequestMetadata,
    pub input_prompt_tokens: usize,
    pub retry_count: u32,
    pub result: CompactionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkippedCompactionOutcome {
    pub request: CompactionRequestMetadata,
    pub input_prompt_tokens: usize,
    pub reason: CompactionSkipReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompactionOutcome {
    Applied(AppliedCompactionOutcome),
    Failed(FailedCompactionOutcome),
    Skipped(SkippedCompactionOutcome),
}
