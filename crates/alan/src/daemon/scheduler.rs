//! Daemon scheduler loop primitives.
//!
//! This module contains the deterministic parts of scheduling:
//! - boot reconciliation (`on_boot_resume`)
//! - due-item claiming and dispatch attempt bookkeeping
//! - retry/backoff wake computation

use super::task_store::{
    ScheduleItemRecord, ScheduleStatus, ScheduleTriggerType, TaskStore, TaskStoreBackend,
};
use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tracing::warn;

pub(crate) const SCHEDULER_ACTOR: &str = "scheduler";

const REASON_BOOT_RECOVER_DISPATCHING: &str =
    "on_boot_resume: recovered interrupted dispatching item";
const REASON_BOOT_WAKE_ELAPSED: &str = "on_boot_resume: wake_at already elapsed";
const REASON_WAKE_ELAPSED: &str = "wake_at reached";
const REASON_DISPATCH_STARTED: &str = "dispatch started";

/// Number of seconds between interval-trigger dispatches.
///
/// V1 keeps this fixed until interval policy fields are added to `ScheduleItemRecord`.
const INTERVAL_WAKE_SECS: i64 = 60;
/// Base backoff for dispatch retry after transient dispatch failures.
const RETRY_BASE_SECS: i64 = 5;
/// Upper bound for retry backoff.
const RETRY_MAX_SECS: i64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchSuccessAction {
    Complete,
    RequeueAt(DateTime<Utc>),
}

pub(crate) fn reconcile_on_boot<B: TaskStoreBackend>(task_store: &TaskStore<B>) -> Result<usize> {
    let mut recovered = 0usize;
    let now = Utc::now();

    for schedule in task_store.list_schedule_items()? {
        if schedule.status == ScheduleStatus::Dispatching {
            task_store.transition_schedule_status(
                &schedule.schedule_id,
                ScheduleStatus::Due,
                SCHEDULER_ACTOR,
                Some(REASON_BOOT_RECOVER_DISPATCHING.to_string()),
            )?;
            recovered += 1;
            continue;
        }

        if schedule.status == ScheduleStatus::Waiting && is_wake_due(&schedule, now) {
            task_store.transition_schedule_status(
                &schedule.schedule_id,
                ScheduleStatus::Due,
                SCHEDULER_ACTOR,
                Some(REASON_BOOT_WAKE_ELAPSED.to_string()),
            )?;
            recovered += 1;
        }
    }

    Ok(recovered)
}

/// Promote elapsed waiting items to `due`, then claim `due` items for dispatch.
///
/// Claimed items are transitioned to `dispatching` and have their `attempt` incremented.
pub(crate) fn claim_due_items<B: TaskStoreBackend>(
    task_store: &TaskStore<B>,
) -> Result<Vec<ScheduleItemRecord>> {
    let now = Utc::now();

    for schedule in task_store.list_schedule_items()? {
        if schedule.status == ScheduleStatus::Waiting && is_wake_due(&schedule, now) {
            task_store.transition_schedule_status(
                &schedule.schedule_id,
                ScheduleStatus::Due,
                SCHEDULER_ACTOR,
                Some(REASON_WAKE_ELAPSED.to_string()),
            )?;
        }
    }

    let mut claimed = Vec::new();
    for schedule in task_store.list_schedule_items()? {
        if schedule.status != ScheduleStatus::Due {
            continue;
        }

        task_store.transition_schedule_status(
            &schedule.schedule_id,
            ScheduleStatus::Dispatching,
            SCHEDULER_ACTOR,
            Some(REASON_DISPATCH_STARTED.to_string()),
        )?;
        let updated = task_store.increment_schedule_attempt(&schedule.schedule_id)?;
        claimed.push(updated);
    }

    Ok(claimed)
}

pub(crate) fn dispatch_success_action(schedule: &ScheduleItemRecord) -> DispatchSuccessAction {
    match schedule.trigger_type {
        ScheduleTriggerType::At | ScheduleTriggerType::RetryBackoff => {
            DispatchSuccessAction::Complete
        }
        ScheduleTriggerType::Interval => DispatchSuccessAction::RequeueAt(
            Utc::now() + ChronoDuration::seconds(INTERVAL_WAKE_SECS),
        ),
    }
}

pub(crate) fn retry_wake_at(schedule: &ScheduleItemRecord) -> DateTime<Utc> {
    let exponent = (schedule.attempt.saturating_sub(1)).min(16);
    let factor = 1_i64 << exponent;
    let delay = (RETRY_BASE_SECS * factor).min(RETRY_MAX_SECS);
    Utc::now() + ChronoDuration::seconds(delay)
}

fn parse_wake_at(value: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(ts) => Some(ts.with_timezone(&Utc)),
        Err(err) => {
            warn!(wake_at = value, error = %err, "Skipping schedule item with invalid next_wake_at timestamp");
            None
        }
    }
}

fn is_wake_due(schedule: &ScheduleItemRecord, now: DateTime<Utc>) -> bool {
    parse_wake_at(&schedule.next_wake_at)
        .map(|wake_at| wake_at <= now)
        .unwrap_or(false)
}

#[cfg(test)]
use super::task_store::JsonFileTaskStoreBackend;

#[cfg(test)]
pub(crate) fn create_test_store(
    storage_dir: impl AsRef<std::path::Path>,
) -> Result<TaskStore<JsonFileTaskStoreBackend>> {
    TaskStore::new(JsonFileTaskStoreBackend::with_storage_dir(storage_dir))
}

#[cfg(test)]
mod tests {
    use super::super::task_store::{RunRecord, ScheduleItemRecord, TaskRecord};
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn reconcile_on_boot_moves_dispatching_and_elapsed_waiting_to_due() {
        let tmp = TempDir::new().unwrap();
        let store = create_test_store(tmp.path()).unwrap();

        store.save_task(TaskRecord::new("task-1", "t")).unwrap();
        store
            .save_run(RunRecord::new("run-1", "task-1", 1))
            .unwrap();
        store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-dispatching",
                "task-1",
                "run-1",
                ScheduleTriggerType::At,
                (Utc::now() + ChronoDuration::seconds(600)).to_rfc3339(),
                "idem-a",
            ))
            .unwrap();
        store
            .transition_schedule_status(
                "sch-dispatching",
                ScheduleStatus::Dispatching,
                SCHEDULER_ACTOR,
                None,
            )
            .unwrap();

        store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-elapsed",
                "task-1",
                "run-1",
                ScheduleTriggerType::At,
                (Utc::now() - ChronoDuration::seconds(1)).to_rfc3339(),
                "idem-b",
            ))
            .unwrap();

        let recovered = reconcile_on_boot(&store).unwrap();
        assert_eq!(recovered, 2);
        assert_eq!(
            store
                .get_schedule_item("sch-dispatching")
                .unwrap()
                .unwrap()
                .status,
            ScheduleStatus::Due
        );
        assert_eq!(
            store
                .get_schedule_item("sch-elapsed")
                .unwrap()
                .unwrap()
                .status,
            ScheduleStatus::Due
        );
    }

    #[test]
    fn claim_due_items_claims_once_and_increments_attempt() {
        let tmp = TempDir::new().unwrap();
        let store = create_test_store(tmp.path()).unwrap();

        store
            .save_task(TaskRecord::new("task-1", "wake run"))
            .unwrap();
        store
            .save_run(RunRecord::new("run-1", "task-1", 1))
            .unwrap();
        store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-due",
                "task-1",
                "run-1",
                ScheduleTriggerType::At,
                (Utc::now() - ChronoDuration::seconds(1)).to_rfc3339(),
                "idem-due",
            ))
            .unwrap();

        let claimed = claim_due_items(&store).unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].status, ScheduleStatus::Dispatching);
        assert_eq!(claimed[0].attempt, 1);

        let claimed_again = claim_due_items(&store).unwrap();
        assert!(claimed_again.is_empty());
    }

    #[test]
    fn dispatch_success_action_and_retry_wake_are_deterministic_enough() {
        let now = Utc::now();
        let mut interval = ScheduleItemRecord::new(
            "sch-interval",
            "task-1",
            "run-1",
            ScheduleTriggerType::Interval,
            now.to_rfc3339(),
            "idem-1",
        );
        interval.attempt = 3;

        match dispatch_success_action(&interval) {
            DispatchSuccessAction::RequeueAt(wake_at) => {
                assert!(wake_at > now);
                assert!(wake_at <= now + ChronoDuration::seconds(120));
            }
            DispatchSuccessAction::Complete => panic!("interval trigger must be requeued"),
        }

        let mut retry = ScheduleItemRecord::new(
            "sch-retry",
            "task-1",
            "run-1",
            ScheduleTriggerType::RetryBackoff,
            now.to_rfc3339(),
            "idem-2",
        );
        retry.attempt = 8;

        let before = Utc::now();
        let wake_at = retry_wake_at(&retry);
        let after = Utc::now();
        assert!(wake_at >= before + ChronoDuration::seconds(RETRY_BASE_SECS));
        assert!(wake_at <= after + ChronoDuration::seconds(RETRY_MAX_SECS));
    }
}
