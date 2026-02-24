use std::collections::{HashMap, VecDeque};

use super::agent_loop::NormalizedToolCall;
use crate::approval::{PendingConfirmation, PendingDynamicToolCall, PendingStructuredInputRequest};
use alan_protocol::Submission;

#[derive(Debug, Clone)]
enum PendingTurnItem {
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
    pending: HashMap<String, PendingTurnItem>,
    pending_tool_replay_batches: HashMap<String, Vec<NormalizedToolCall>>,
    /// Cross-type insertion order tracking for pending items
    pending_order: Vec<String>,
    confirmation_order: Vec<String>,
    structured_input_order: Vec<String>,
    dynamic_tool_call_order: Vec<String>,
    turn_activity: TurnActivityState,
    /// Submissions buffered during turn execution that need to be requeued
    /// after the turn completes (e.g., user input during tool execution).
    buffered_inband_submissions: VecDeque<Submission>,
}

impl TurnState {
    pub(crate) fn has_pending_interaction(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
        self.pending_tool_replay_batches.clear();
        self.pending_order.clear();
        self.confirmation_order.clear();
        self.structured_input_order.clear();
        self.dynamic_tool_call_order.clear();
        self.turn_activity = TurnActivityState::Idle;
        self.buffered_inband_submissions.clear();
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
            .filter(|submission| matches!(submission.op, alan_protocol::Op::UserInput { .. }))
            .count()
    }

    /// Clear buffered inband submissions and return the count
    pub(crate) fn clear_buffered_inband_submissions(&mut self) -> usize {
        let count = self.buffered_inband_submissions.len();
        self.buffered_inband_submissions.clear();
        count
    }

    /// Get the latest pending key across all pending types
    #[allow(dead_code)]
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

    // Legacy methods for backward compatibility
    #[allow(dead_code)]
    pub(crate) fn set_logical_turn_open(&mut self, is_open: bool) {
        self.turn_activity = if is_open {
            TurnActivityState::Running
        } else {
            TurnActivityState::Idle
        };
    }

    #[allow(dead_code)]
    pub(crate) fn is_logical_turn_open(&self) -> bool {
        self.is_turn_active()
    }

    pub(crate) fn set_confirmation(&mut self, pending: PendingConfirmation) {
        let key = pending.checkpoint_id.clone();
        self.pending
            .insert(key.clone(), PendingTurnItem::Confirmation(pending));
        push_latest_key(&mut self.pending_order, key.clone());
        push_latest_key(&mut self.confirmation_order, key);
    }

    pub(crate) fn pending_confirmation(&self) -> Option<PendingConfirmation> {
        latest_typed_item(&self.pending, &self.confirmation_order, |item| match item {
            PendingTurnItem::Confirmation(value) => Some(value.clone()),
            _ => None,
        })
    }

    pub(crate) fn take_confirmation(&mut self, checkpoint_id: &str) -> Option<PendingConfirmation> {
        let target_id = if checkpoint_id == "latest" {
            self.confirmation_order.last()?.clone()
        } else {
            checkpoint_id.to_string()
        };

        let item = self.pending.remove(&target_id)?;
        remove_key(&mut self.pending_order, &target_id);
        remove_key(&mut self.confirmation_order, &target_id);
        match item {
            PendingTurnItem::Confirmation(value) => Some(value),
            other => {
                self.pending.insert(target_id, other);
                None
            }
        }
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
            .insert(key.clone(), PendingTurnItem::StructuredInput(pending));
        push_latest_key(&mut self.pending_order, key.clone());
        push_latest_key(&mut self.structured_input_order, key);
    }

    pub(crate) fn pending_structured_input(&self) -> Option<PendingStructuredInputRequest> {
        latest_typed_item(
            &self.pending,
            &self.structured_input_order,
            |item| match item {
                PendingTurnItem::StructuredInput(value) => Some(value.clone()),
                _ => None,
            },
        )
    }

    pub(crate) fn take_structured_input(
        &mut self,
        request_id: &str,
    ) -> Option<PendingStructuredInputRequest> {
        let item = self.pending.remove(request_id)?;
        remove_key(&mut self.pending_order, request_id);
        remove_key(&mut self.structured_input_order, request_id);
        match item {
            PendingTurnItem::StructuredInput(value) => Some(value),
            other => {
                self.pending.insert(request_id.to_string(), other);
                None
            }
        }
    }

    pub(crate) fn set_dynamic_tool_call(&mut self, pending: PendingDynamicToolCall) {
        let key = pending.call_id.clone();
        self.pending
            .insert(key.clone(), PendingTurnItem::DynamicToolCall(pending));
        push_latest_key(&mut self.pending_order, key.clone());
        push_latest_key(&mut self.dynamic_tool_call_order, key);
    }

    pub(crate) fn pending_dynamic_tool_call(&self) -> Option<PendingDynamicToolCall> {
        latest_typed_item(
            &self.pending,
            &self.dynamic_tool_call_order,
            |item| match item {
                PendingTurnItem::DynamicToolCall(value) => Some(value.clone()),
                _ => None,
            },
        )
    }

    pub(crate) fn take_dynamic_tool_call(
        &mut self,
        call_id: &str,
    ) -> Option<PendingDynamicToolCall> {
        let item = self.pending.remove(call_id)?;
        remove_key(&mut self.pending_order, call_id);
        remove_key(&mut self.dynamic_tool_call_order, call_id);
        match item {
            PendingTurnItem::DynamicToolCall(value) => Some(value),
            other => {
                self.pending.insert(call_id.to_string(), other);
                None
            }
        }
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

fn latest_typed_item<T, F>(
    pending: &HashMap<String, PendingTurnItem>,
    order: &[String],
    mut project: F,
) -> Option<T>
where
    F: FnMut(&PendingTurnItem) -> Option<T>,
{
    order
        .iter()
        .rev()
        .find_map(|key| pending.get(key).and_then(&mut project))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_confirmation_latest_and_take() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-1".to_string(),
            checkpoint_type: "tool_approval".to_string(),
            summary: "Approve?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string(), "reject".to_string()],
        });

        let latest = state.pending_confirmation().unwrap();
        assert_eq!(latest.checkpoint_id, "cp-1");

        let taken = state.take_confirmation("latest").unwrap();
        assert_eq!(taken.checkpoint_id, "cp-1");
        assert!(state.pending_confirmation().is_none());
    }

    #[test]
    fn test_structured_input_latest_keeps_previous_request_available() {
        let mut state = TurnState::default();
        state.set_structured_input(PendingStructuredInputRequest {
            request_id: "r1".to_string(),
            title: "T1".to_string(),
            prompt: "P1".to_string(),
            questions: vec![],
        });
        state.set_structured_input(PendingStructuredInputRequest {
            request_id: "r2".to_string(),
            title: "T2".to_string(),
            prompt: "P2".to_string(),
            questions: vec![],
        });

        assert_eq!(state.pending_structured_input().unwrap().request_id, "r2");
        let old = state.take_structured_input("r1").unwrap();
        assert_eq!(old.request_id, "r1");
        assert_eq!(state.pending_structured_input().unwrap().request_id, "r2");
    }

    #[test]
    fn test_confirmation_latest_falls_back_to_previous_after_take() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-1".to_string(),
            checkpoint_type: "tool_approval".to_string(),
            summary: "Approve 1?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string()],
        });
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-2".to_string(),
            checkpoint_type: "tool_approval".to_string(),
            summary: "Approve 2?".to_string(),
            details: json!({}),
            options: vec!["approve".to_string()],
        });

        let latest = state.take_confirmation("latest").unwrap();
        assert_eq!(latest.checkpoint_id, "cp-2");
        assert_eq!(state.pending_confirmation().unwrap().checkpoint_id, "cp-1");
    }

    #[test]
    fn test_clear_resets_all_pending_types() {
        let mut state = TurnState::default();
        state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp".to_string(),
            checkpoint_type: "tool_approval".to_string(),
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
        assert!(state.pending_dynamic_tool_call().is_none());
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
    fn test_dynamic_tool_latest_keeps_previous_call_available() {
        let mut state = TurnState::default();
        state.set_dynamic_tool_call(PendingDynamicToolCall {
            call_id: "d1".to_string(),
            tool_name: "lookup".to_string(),
            arguments: json!({"id":"1"}),
        });
        state.set_dynamic_tool_call(PendingDynamicToolCall {
            call_id: "d2".to_string(),
            tool_name: "lookup".to_string(),
            arguments: json!({"id":"2"}),
        });

        assert_eq!(state.pending_dynamic_tool_call().unwrap().call_id, "d2");
        let old = state.take_dynamic_tool_call("d1").unwrap();
        assert_eq!(old.call_id, "d1");
        assert_eq!(state.pending_dynamic_tool_call().unwrap().call_id, "d2");
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

        let _ = state.take_dynamic_tool_call("dyn-1");
        assert_eq!(state.latest_pending_key().as_deref(), Some("cp-1"));
    }

    #[test]
    fn test_turn_state_buffers_inband_submissions_fifo() {
        let mut state = TurnState::default();
        state.push_buffered_inband_submission(Submission {
            id: "s1".to_string(),
            op: alan_protocol::Op::UserInput {
                content: "one".to_string(),
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::Confirm {
                checkpoint_id: "latest".to_string(),
                choice: alan_protocol::ConfirmChoice::Approve,
                modifications: None,
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
            op: alan_protocol::Op::UserInput {
                content: "one".to_string(),
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::Confirm {
                checkpoint_id: "latest".to_string(),
                choice: alan_protocol::ConfirmChoice::Approve,
                modifications: None,
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
            op: alan_protocol::Op::UserInput {
                content: "one".to_string(),
            },
        });
        state.push_buffered_inband_submission(Submission {
            id: "s2".to_string(),
            op: alan_protocol::Op::UserInput {
                content: "two".to_string(),
            },
        });

        let count = state.clear_buffered_inband_submissions();
        assert_eq!(count, 2);
        assert!(state.pop_buffered_inband_submission().is_none());
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
