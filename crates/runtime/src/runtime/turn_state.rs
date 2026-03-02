use std::collections::{HashMap, VecDeque};

use super::agent_loop::NormalizedToolCall;
use crate::approval::{PendingConfirmation, PendingDynamicToolCall, PendingStructuredInputRequest};
use crate::tape::ContentPart;
use alan_protocol::Submission;

const MAX_QUEUED_NEXT_TURN_INPUTS: usize = 16;

#[derive(Debug, Clone)]
pub(super) enum PendingYield {
    Confirmation(PendingConfirmation),
    StructuredInput(PendingStructuredInputRequest),
    DynamicToolCall(PendingDynamicToolCall),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TurnActivityState {
    #[default]
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TurnState {
    pending: HashMap<String, PendingYield>,
    pending_tool_replay_batches: HashMap<String, Vec<NormalizedToolCall>>,
    /// Insertion order tracking for all pending items
    pending_order: Vec<String>,
    turn_activity: TurnActivityState,
    /// Submissions buffered during turn execution that need to be requeued
    /// after the turn completes (e.g., user input during tool execution).
    buffered_inband_submissions: VecDeque<Submission>,
    /// Queued context for `InputMode::NextTurn`.
    queued_next_turn_inputs: VecDeque<Vec<ContentPart>>,
}

impl TurnState {
    pub(crate) fn has_pending_interaction(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
        self.pending_tool_replay_batches.clear();
        self.pending_order.clear();
        self.turn_activity = TurnActivityState::Idle;
        self.buffered_inband_submissions.clear();
    }

    /// Queue `next_turn` input parts. Returns `Some(new_len)` on success, `None` on overflow.
    pub(crate) fn queue_next_turn_input(&mut self, parts: Vec<ContentPart>) -> Option<usize> {
        if self.queued_next_turn_inputs.len() >= MAX_QUEUED_NEXT_TURN_INPUTS {
            return None;
        }
        self.queued_next_turn_inputs.push_back(parts);
        Some(self.queued_next_turn_inputs.len())
    }

    /// Drain queued `next_turn` input parts in FIFO order.
    pub(crate) fn drain_next_turn_inputs(&mut self) -> VecDeque<Vec<ContentPart>> {
        std::mem::take(&mut self.queued_next_turn_inputs)
    }

    /// Number of queued `next_turn` payloads.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn queued_next_turn_input_count(&self) -> usize {
        self.queued_next_turn_inputs.len()
    }

    /// Drain all buffered inband submissions.
    pub(crate) fn drain_buffered_inband_submissions(&mut self) -> VecDeque<Submission> {
        std::mem::take(&mut self.buffered_inband_submissions)
    }

    /// Push a submission to the buffered inband submissions queue.
    pub(crate) fn push_buffered_inband_submission(&mut self, submission: Submission) {
        self.buffered_inband_submissions.push_back(submission);
    }

    /// Pop a submission from the buffered inband submissions queue.
    pub(crate) fn pop_buffered_inband_submission(&mut self) -> Option<Submission> {
        self.buffered_inband_submissions.pop_front()
    }

    /// Count user input submissions in the buffered queue
    pub(crate) fn buffered_inband_user_input_count(&self) -> usize {
        self.buffered_inband_submissions
            .iter()
            .filter(|submission| matches!(submission.op, alan_protocol::Op::Input { .. }))
            .count()
    }

    /// Clear buffered inband submissions and return the count
    pub(crate) fn clear_buffered_inband_submissions(&mut self) -> usize {
        let count = self.buffered_inband_submissions.len();
        self.buffered_inband_submissions.clear();
        count
    }

    /// Get the latest pending key across all pending types
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn latest_pending_key(&self) -> Option<String> {
        self.pending_order.last().cloned()
    }

    pub(crate) fn set_turn_activity(&mut self, activity: TurnActivityState) {
        self.turn_activity = activity;
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn turn_activity(&self) -> TurnActivityState {
        self.turn_activity
    }

    pub(crate) fn is_turn_active(&self) -> bool {
        !matches!(self.turn_activity, TurnActivityState::Idle)
    }

    pub(crate) fn set_confirmation(&mut self, pending: PendingConfirmation) {
        let key = pending.checkpoint_id.clone();
        self.pending
            .insert(key.clone(), PendingYield::Confirmation(pending));
        push_latest_key(&mut self.pending_order, key);
    }

    pub(crate) fn pending_confirmation(&self) -> Option<PendingConfirmation> {
        self.pending_order
            .iter()
            .rev()
            .find_map(|key| match self.pending.get(key) {
                Some(PendingYield::Confirmation(value)) => Some(value.clone()),
                _ => None,
            })
    }

    pub(crate) fn set_tool_replay_batch(
        &mut self,
        checkpoint_id: impl Into<String>,
        tool_calls: Vec<NormalizedToolCall>,
    ) {
        self.pending_tool_replay_batches
            .insert(checkpoint_id.into(), tool_calls);
    }

    pub(crate) fn take_tool_replay_batch(
        &mut self,
        checkpoint_id: &str,
    ) -> Option<Vec<NormalizedToolCall>> {
        self.pending_tool_replay_batches.remove(checkpoint_id)
    }

    pub(crate) fn set_structured_input(&mut self, pending: PendingStructuredInputRequest) {
        let key = pending.request_id.clone();
        self.pending
            .insert(key.clone(), PendingYield::StructuredInput(pending));
        push_latest_key(&mut self.pending_order, key);
    }

    pub(crate) fn set_dynamic_tool_call(&mut self, pending: PendingDynamicToolCall) {
        let key = pending.call_id.clone();
        self.pending
            .insert(key.clone(), PendingYield::DynamicToolCall(pending));
        push_latest_key(&mut self.pending_order, key);
    }

    /// Unified lookup: take any pending item by request_id.
    pub(super) fn take_pending(&mut self, request_id: &str) -> Option<PendingYield> {
        let item = self.pending.remove(request_id)?;
        remove_key(&mut self.pending_order, request_id);
        Some(item)
    }
}

fn push_latest_key(order: &mut Vec<String>, key: String) {
    remove_key(order, &key);
    order.push(key);
}

fn remove_key(order: &mut Vec<String>, key: &str) {
    if let Some(pos) = order.iter().position(|existing| existing == key) {
        order.remove(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_confirmation_set_and_pending() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-1".to_string(),
            checkpoint_type: "tool_escalation".to_string(),
            summary: "Approve?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string(), "reject".to_string()],
        });

        let latest = state.pending_confirmation().unwrap();
        assert_eq!(latest.checkpoint_id, "cp-1");

        // take_pending removes it
        let taken = state.take_pending("cp-1").unwrap();
        assert!(matches!(taken, PendingYield::Confirmation(_)));
        assert!(state.pending_confirmation().is_none());
    }

    #[test]
    fn test_clear_resets_all_pending_types() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp".to_string(),
            checkpoint_type: "tool_escalation".to_string(),
            summary: "Approve?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string()],
        });
        state.set_dynamic_tool_call(PendingDynamicToolCall {
            call_id: "d1".to_string(),
            tool_name: "lookup".to_string(),
            arguments: json!({"id":"1"}),
        });
        state.clear();
        assert!(state.pending_confirmation().is_none());
        assert!(!state.has_pending_interaction());
        assert!(matches!(state.turn_activity(), TurnActivityState::Idle));
    }

    #[test]
    fn test_turn_activity_state_roundtrip_and_clear() {
        let mut state = TurnState::default();
        assert!(matches!(state.turn_activity(), TurnActivityState::Idle));

        state.set_turn_activity(TurnActivityState::Running);
        assert!(matches!(state.turn_activity(), TurnActivityState::Running));

        state.set_turn_activity(TurnActivityState::Paused);
        assert!(matches!(state.turn_activity(), TurnActivityState::Paused));

        state.clear();
        assert!(matches!(state.turn_activity(), TurnActivityState::Idle));
    }

    #[test]
    fn test_take_pending_removes_dynamic_tool_call() {
        let mut state = TurnState::default();
        state.set_dynamic_tool_call(PendingDynamicToolCall {
            call_id: "d1".to_string(),
            tool_name: "lookup".to_string(),
            arguments: json!({"id":"1"}),
        });

        let taken = state.take_pending("d1").unwrap();
        assert!(matches!(taken, PendingYield::DynamicToolCall(_)));
        assert!(!state.has_pending_interaction());
    }

    #[test]
    fn test_latest_pending_key_tracks_cross_type_insertion_order() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-1".to_string(),
            checkpoint_type: "manual".to_string(),
            summary: "Approve?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string()],
        });
        assert_eq!(state.latest_pending_key().as_deref(), Some("cp-1"));

        state.set_dynamic_tool_call(PendingDynamicToolCall {
            call_id: "dyn-1".to_string(),
            tool_name: "lookup".to_string(),
            arguments: json!({"id":"1"}),
        });
        assert_eq!(state.latest_pending_key().as_deref(), Some("dyn-1"));

        let _ = state.take_pending("dyn-1");
        assert_eq!(state.latest_pending_key().as_deref(), Some("cp-1"));
    }

    #[test]
    fn test_turn_state_buffers_inband_submissions_fifo() {
        let mut state = TurnState::default();
        state.push_buffered_inband_submission(Submission {
            id: "s1".to_string(),
            op: alan_protocol::Op::Input {
                parts: vec![alan_protocol::ContentPart::text("one")],
                mode: alan_protocol::InputMode::Steer,
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::Resume {
                request_id: "latest".to_string(),
                content: vec![alan_protocol::ContentPart::structured(
                    serde_json::json!({"choice": "approve"}),
                )],
            },
        });

        assert_eq!(state.buffered_inband_user_input_count(), 1);
        assert_eq!(
            state
                .pop_buffered_inband_submission()
                .as_ref()
                .map(|s| s.id.as_str()),
            Some("s1")
        );
        assert_eq!(
            state
                .pop_buffered_inband_submission()
                .as_ref()
                .map(|s| s.id.as_str()),
            Some("s2")
        );
        assert!(state.pop_buffered_inband_submission().is_none());
    }

    #[test]
    fn test_turn_state_drain_buffered_inband_submissions_preserves_order() {
        let mut state = TurnState::default();
        state.push_buffered_inband_submission(Submission {
            id: "s1".to_string(),
            op: alan_protocol::Op::Input {
                parts: vec![alan_protocol::ContentPart::text("one")],
                mode: alan_protocol::InputMode::Steer,
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::Resume {
                request_id: "latest".to_string(),
                content: vec![alan_protocol::ContentPart::structured(
                    serde_json::json!({"choice": "approve"}),
                )],
            },
        });

        let drained = state.drain_buffered_inband_submissions();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained.front().map(|s| s.id.as_str()), Some("s1"));
        assert_eq!(drained.back().map(|s| s.id.as_str()), Some("s2"));
        assert!(state.pop_buffered_inband_submission().is_none());
    }

    #[test]
    fn test_clear_buffered_inband_submissions_returns_count() {
        let mut state = TurnState::default();
        state.push_buffered_inband_submission(Submission {
            id: "s1".to_string(),
            op: alan_protocol::Op::Input {
                parts: vec![alan_protocol::ContentPart::text("one")],
                mode: alan_protocol::InputMode::Steer,
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::Input {
                parts: vec![alan_protocol::ContentPart::text("two")],
                mode: alan_protocol::InputMode::Steer,
            },
        });

        let count = state.clear_buffered_inband_submissions();
        assert_eq!(count, 2);
        assert!(state.pop_buffered_inband_submission().is_none());
    }

    #[test]
    fn test_queue_next_turn_inputs_fifo_and_drain() {
        let mut state = TurnState::default();
        assert_eq!(
            state.queue_next_turn_input(vec![ContentPart::text("ctx-1")]),
            Some(1)
        );
        assert_eq!(
            state.queue_next_turn_input(vec![ContentPart::text("ctx-2")]),
            Some(2)
        );
        assert_eq!(state.queued_next_turn_input_count(), 2);

        let drained = state.drain_next_turn_inputs();
        assert_eq!(drained.len(), 2);
        assert_eq!(alan_protocol::parts_to_text(&drained[0]), "ctx-1");
        assert_eq!(alan_protocol::parts_to_text(&drained[1]), "ctx-2");
        assert_eq!(state.queued_next_turn_input_count(), 0);
    }

    #[test]
    fn test_queue_next_turn_inputs_overflow_is_rejected() {
        let mut state = TurnState::default();
        for _ in 0..MAX_QUEUED_NEXT_TURN_INPUTS {
            assert!(
                state
                    .queue_next_turn_input(vec![ContentPart::text("queued")])
                    .is_some()
            );
        }
        assert!(
            state
                .queue_next_turn_input(vec![ContentPart::text("overflow")])
                .is_none()
        );
    }

    #[test]
    fn test_tool_replay_batch_roundtrip() {
        let mut state = TurnState::default();
        let tool_calls = vec![
            NormalizedToolCall {
                id: "call-1".to_string(),
                name: "web_search".to_string(),
                arguments: json!({"query": "rust"}),
            },
            NormalizedToolCall {
                id: "call-2".to_string(),
                name: "memory_write".to_string(),
                arguments: json!({"key": "test", "value": "data"}),
            },
        ];

        state.set_tool_replay_batch("cp-1", tool_calls.clone());

        let retrieved = state.take_tool_replay_batch("cp-1").unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].id, "call-1");
        assert_eq!(retrieved[1].id, "call-2");

        // Should be removed after take
        assert!(state.take_tool_replay_batch("cp-1").is_none());
    }
}
