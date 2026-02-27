use std::{collections::VecDeque, sync::Arc};

use alan_protocol::{Event, Op, Submission};
use anyhow::Result;
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

use super::agent_loop::{RuntimeLoopState, handle_submission_with_cancel};
use super::turn_state::TurnState;
use super::turn_support::cancel_current_task;

const MAX_BROKERED_INBAND_USER_INPUTS: usize = 16;
const MAX_BUFFERED_INBAND_USER_INPUTS: usize = 16;

#[derive(Clone)]
pub(super) struct TurnInputBroker {
    inner: Arc<TurnInputBrokerInner>,
}

struct TurnInputBrokerInner {
    queue: Mutex<VecDeque<Submission>>,
    notify: Notify,
}

impl Default for TurnInputBroker {
    fn default() -> Self {
        Self {
            inner: Arc::new(TurnInputBrokerInner {
                queue: Mutex::new(VecDeque::new()),
                notify: Notify::new(),
            }),
        }
    }
}

impl TurnInputBroker {
    pub(super) async fn push(&self, submission: Submission) -> bool {
        let mut guard = self.inner.queue.lock().await;
        if matches!(submission.op, Op::Input { .. })
            && guard
                .iter()
                .filter(|queued| matches!(queued.op, Op::Input { .. }))
                .count()
                >= MAX_BROKERED_INBAND_USER_INPUTS
        {
            return false;
        }
        guard.push_back(submission);
        drop(guard);
        self.inner.notify.notify_one();
        true
    }

    pub(super) async fn recv(&self, cancel: &CancellationToken) -> Option<Submission> {
        loop {
            if let Some(submission) = self.try_pop().await {
                return Some(submission);
            }

            tokio::select! {
                _ = cancel.cancelled() => return None,
                _ = self.inner.notify.notified() => {}
            }
        }
    }

    pub(super) async fn clear(&self) {
        self.inner.queue.lock().await.clear();
    }

    pub(super) async fn drain(&self) -> VecDeque<Submission> {
        std::mem::take(&mut *self.inner.queue.lock().await)
    }

    pub(super) async fn try_recv(&self) -> Option<Submission> {
        self.try_pop().await
    }

    async fn try_pop(&self) -> Option<Submission> {
        self.inner.queue.lock().await.pop_front()
    }
}

pub(super) fn should_drive_turn_submission(op: &Op) -> bool {
    matches!(op, Op::Turn { .. } | Op::Input { .. })
}

pub(super) fn is_turn_resume_submission(op: &Op) -> bool {
    matches!(op, Op::Resume { .. })
}

pub(super) fn is_turn_inband_submission(op: &Op) -> bool {
    is_turn_resume_submission(op) || matches!(op, Op::Input { .. })
}

pub(super) async fn drive_turn_submission_with_cancel<E, F, S>(
    state: &mut RuntimeLoopState,
    initial_submission: Submission,
    broker: &TurnInputBroker,
    emit: &mut E,
    set_active_submission_id: &mut S,
    cancel: &CancellationToken,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
    S: FnMut(&str),
{
    broker.clear().await;
    let _ = state.turn_state.clear_buffered_inband_submissions();
    set_active_submission_id(&initial_submission.id);

    handle_submission_with_cancel(state, initial_submission, emit, cancel).await?;

    loop {
        let next_submission = if state.turn_state.has_pending_interaction() {
            loop {
                let Some(incoming) = broker.recv(cancel).await else {
                    if cancel.is_cancelled() && state.turn_state.has_pending_interaction() {
                        // Report and clear buffered in-turn submissions before cancel_current_task
                        // clears turn_state, otherwise the drop count under-reports.
                        emit_dropped_in_turn_submissions(emit, &mut state.turn_state, broker).await;
                        cancel_current_task(state, emit).await?;
                        return Ok(());
                    }
                    emit_dropped_in_turn_submissions(emit, &mut state.turn_state, broker).await;
                    return Ok(());
                };

                if is_turn_resume_submission(&incoming.op) {
                    break Some(incoming);
                }

                if matches!(incoming.op, Op::Input { .. })
                    && state.turn_state.buffered_inband_user_input_count()
                        >= MAX_BUFFERED_INBAND_USER_INPUTS
                {
                    emit(Event::Error {
                        message: format!(
                            "Too many queued in-turn user inputs (limit={MAX_BUFFERED_INBAND_USER_INPUTS}); dropping newest input."
                        ),
                        recoverable: true,
                    })
                    .await;
                    continue;
                }
                state.turn_state.push_buffered_inband_submission(incoming);
            }
        } else if let Some(buffered) = state.turn_state.pop_buffered_inband_submission() {
            Some(buffered)
        } else {
            broker.try_recv().await
        };

        let Some(next_submission) = next_submission else {
            break;
        };
        set_active_submission_id(&next_submission.id);

        handle_submission_with_cancel(state, next_submission, emit, cancel).await?;
    }

    Ok(())
}

async fn emit_dropped_in_turn_submissions<E, F>(
    emit: &mut E,
    turn_state: &mut TurnState,
    broker: &TurnInputBroker,
) where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let dropped_buffered = turn_state.clear_buffered_inband_submissions();
    let dropped_brokered = broker.drain().await.len();
    let dropped_total = dropped_buffered + dropped_brokered;
    if dropped_total > 0 {
        emit(Event::Error {
            message: format!(
                "Dropped {dropped_total} in-turn buffered submissions due turn cancellation or shutdown."
            ),
            recoverable: true,
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_submission_classification() {
        assert!(should_drive_turn_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("hi")],
        }));
        assert!(should_drive_turn_submission(&Op::Turn {
            parts: vec![alan_protocol::ContentPart::text("hi")],
            context: None,
        }));
        assert!(is_turn_resume_submission(&Op::Resume {
            request_id: "latest".to_string(),
            content: vec![alan_protocol::ContentPart::structured(
                serde_json::json!({"choice": "approve"})
            )],
        }));
        assert!(is_turn_inband_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("follow up")],
        }));
        assert!(is_turn_inband_submission(&Op::Resume {
            request_id: "latest".to_string(),
            content: vec![alan_protocol::ContentPart::structured(
                serde_json::json!({"choice": "approve"})
            )],
        }));
        assert!(is_turn_resume_submission(&Op::Resume {
            request_id: "r1".to_string(),
            content: vec![alan_protocol::ContentPart::structured(
                serde_json::json!({"answers": []})
            )],
        }));
        assert!(is_turn_resume_submission(&Op::Resume {
            request_id: "c1".to_string(),
            content: vec![alan_protocol::ContentPart::structured(
                serde_json::json!({"success": true})
            )],
        }));
        assert!(!is_turn_resume_submission(&Op::RegisterDynamicTools {
            tools: vec![]
        }));
        assert!(!is_turn_inband_submission(&Op::RegisterDynamicTools {
            tools: vec![]
        }));
    }

    #[tokio::test]
    async fn test_turn_input_broker_roundtrip_and_clear() {
        let broker = TurnInputBroker::default();
        assert!(
            broker
                .push(Submission {
                    id: "sub-1".to_string(),
                    op: Op::Resume {
                        request_id: "latest".to_string(),
                        content: vec![alan_protocol::ContentPart::structured(
                            serde_json::json!({"choice": "approve"}),
                        )],
                    },
                })
                .await
        );

        let cancel = CancellationToken::new();
        let got = broker.recv(&cancel).await.expect("queued submission");
        assert_eq!(got.id, "sub-1");

        assert!(
            broker
                .push(Submission {
                    id: "sub-2".to_string(),
                    op: Op::Input {
                        parts: vec![alan_protocol::ContentPart::text("follow up")],
                    },
                })
                .await
        );
        let got = broker.try_recv().await.expect("queued submission");
        assert_eq!(got.id, "sub-2");

        assert!(
            broker
                .push(Submission {
                    id: "sub-3".to_string(),
                    op: Op::Resume {
                        request_id: "r1".to_string(),
                        content: vec![alan_protocol::ContentPart::structured(
                            serde_json::json!({"answers": []}),
                        )],
                    },
                })
                .await
        );
        broker.clear().await;
        cancel.cancel();
        assert!(broker.recv(&cancel).await.is_none());
    }

    #[tokio::test]
    async fn test_turn_input_broker_caps_user_inputs_but_allows_resume_submissions() {
        let broker = TurnInputBroker::default();
        for idx in 0..MAX_BROKERED_INBAND_USER_INPUTS {
            assert!(
                broker
                    .push(Submission {
                        id: format!("u-{idx}"),
                        op: Op::Input {
                            parts: vec![alan_protocol::ContentPart::text(format!("msg {idx}"))],
                        },
                    })
                    .await
            );
        }

        assert!(
            !broker
                .push(Submission {
                    id: "u-overflow".to_string(),
                    op: Op::Input {
                        parts: vec![alan_protocol::ContentPart::text("overflow")],
                    },
                })
                .await
        );
        assert!(
            broker
                .push(Submission {
                    id: "resume-1".to_string(),
                    op: Op::Resume {
                        request_id: "latest".to_string(),
                        content: vec![alan_protocol::ContentPart::structured(
                            serde_json::json!({"choice": "approve"}),
                        )],
                    },
                })
                .await
        );
    }

    #[tokio::test]
    async fn test_emit_dropped_in_turn_submissions_reports_count() {
        let broker = TurnInputBroker::default();
        let mut turn_state = TurnState::default();
        turn_state.push_buffered_inband_submission(Submission {
            id: "u-1".to_string(),
            op: Op::Input {
                parts: vec![alan_protocol::ContentPart::text("queued")],
            },
        });
        assert!(
            broker
                .push(Submission {
                    id: "c-1".to_string(),
                    op: Op::Resume {
                        request_id: "latest".to_string(),
                        content: vec![alan_protocol::ContentPart::structured(
                            serde_json::json!({"choice": "approve"}),
                        )],
                    },
                })
                .await
        );

        let mut events = Vec::new();
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        emit_dropped_in_turn_submissions(&mut emit, &mut turn_state, &broker).await;

        assert_eq!(turn_state.clear_buffered_inband_submissions(), 0);
        assert!(broker.try_recv().await.is_none());
        assert!(events.iter().any(|event| matches!(
            event,
            Event::Error { message, recoverable }
                if *recoverable && message.contains("Dropped 2 in-turn buffered submissions")
        )));
    }
}
