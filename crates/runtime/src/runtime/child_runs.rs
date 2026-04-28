use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Lifecycle state for a delegated child runtime launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildRunStatus {
    Starting,
    Running,
    Blocked,
    Completed,
    Failed,
    TimedOut,
    Terminating,
    Terminated,
    Cancelled,
}

impl ChildRunStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::TimedOut | Self::Terminated | Self::Cancelled
        )
    }
}

/// Requested child termination mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildRunTerminationMode {
    Graceful,
    Forceful,
}

/// Audit metadata for an explicit child termination request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRunTerminationRequest {
    pub actor: String,
    pub reason: String,
    pub mode: ChildRunTerminationMode,
    pub requested_at_ms: u64,
}

/// Observable lifecycle record for one delegated child runtime launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRunRecord {
    pub id: String,
    pub parent_session_id: String,
    pub child_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollout_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_target: Option<String>,
    pub status: ChildRunStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_heartbeat_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_progress_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_event_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_status_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub termination: Option<ChildRunTerminationRequest>,
}

impl ChildRunRecord {
    pub fn new(
        id: String,
        parent_session_id: String,
        child_session_id: String,
        workspace_root: Option<String>,
        rollout_path: Option<String>,
        launch_target: Option<String>,
    ) -> Self {
        let now = now_ms();
        Self {
            id,
            parent_session_id,
            child_session_id,
            workspace_root,
            rollout_path,
            launch_target,
            status: ChildRunStatus::Starting,
            created_at_ms: now,
            updated_at_ms: now,
            latest_heartbeat_at_ms: None,
            latest_progress_at_ms: None,
            latest_event_kind: None,
            latest_status_summary: None,
            warnings: Vec::new(),
            error_message: None,
            termination: None,
        }
    }
}

/// Process-local registry for active and recently terminal child runs.
#[derive(Debug, Clone, Default)]
pub struct ChildRunRegistry {
    inner: Arc<RwLock<HashMap<String, ChildRunRecord>>>,
}

impl ChildRunRegistry {
    pub fn register(&self, record: ChildRunRecord) {
        let mut records = self.inner.write().expect("child run registry poisoned");
        records.insert(record.id.clone(), record);
    }

    pub fn list_for_parent(&self, parent_session_id: &str) -> Vec<ChildRunRecord> {
        let mut records = self
            .inner
            .read()
            .expect("child run registry poisoned")
            .values()
            .filter(|record| record.parent_session_id == parent_session_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.created_at_ms);
        records
    }

    pub fn get_for_parent(
        &self,
        parent_session_id: &str,
        child_run_id: &str,
    ) -> Option<ChildRunRecord> {
        self.inner
            .read()
            .expect("child run registry poisoned")
            .get(child_run_id)
            .filter(|record| record.parent_session_id == parent_session_id)
            .cloned()
    }

    pub fn get(&self, child_run_id: &str) -> Option<ChildRunRecord> {
        self.inner
            .read()
            .expect("child run registry poisoned")
            .get(child_run_id)
            .cloned()
    }

    pub fn mark_running(&self, child_run_id: &str) {
        self.update(child_run_id, |record, now| {
            if !record.status.is_terminal() {
                record.status = ChildRunStatus::Running;
                record.latest_progress_at_ms = Some(now);
            }
        });
    }

    pub fn observe_heartbeat(&self, child_run_id: &str, summary: Option<String>) {
        self.update(child_run_id, |record, now| {
            if !record.status.is_terminal() {
                record.latest_heartbeat_at_ms = Some(now);
                if let Some(summary) = summary {
                    record.latest_status_summary = Some(summary);
                }
            }
        });
    }

    pub fn observe_progress(
        &self,
        child_run_id: &str,
        event_kind: impl Into<String>,
        summary: Option<String>,
    ) {
        self.update(child_run_id, |record, now| {
            if !record.status.is_terminal() {
                record.latest_progress_at_ms = Some(now);
                record.latest_event_kind = Some(event_kind.into());
                if let Some(summary) = summary {
                    record.latest_status_summary = Some(summary);
                }
            }
        });
    }

    pub fn observe_warning(&self, child_run_id: &str, warning: String) {
        self.update(child_run_id, |record, _| {
            record.warnings.push(warning);
        });
    }

    pub fn mark_terminal(
        &self,
        child_run_id: &str,
        status: ChildRunStatus,
        error_message: Option<String>,
    ) {
        self.update(child_run_id, |record, now| {
            record.status = status;
            record.error_message = error_message;
            record.latest_progress_at_ms = Some(now);
        });
    }

    pub fn request_termination(
        &self,
        parent_session_id: &str,
        child_run_id: &str,
        actor: impl Into<String>,
        mode: ChildRunTerminationMode,
        reason: impl Into<String>,
    ) -> Result<ChildRunRecord, ChildRunRegistryError> {
        let mut records = self.inner.write().expect("child run registry poisoned");
        let Some(record) = records.get_mut(child_run_id) else {
            return Err(ChildRunRegistryError::NotFound);
        };
        if record.parent_session_id != parent_session_id {
            return Err(ChildRunRegistryError::NotFound);
        }
        if record.status.is_terminal() {
            return Err(ChildRunRegistryError::AlreadyTerminal(record.clone()));
        }

        let now = now_ms();
        record.status = ChildRunStatus::Terminating;
        record.updated_at_ms = now;
        record.termination = Some(ChildRunTerminationRequest {
            actor: actor.into(),
            reason: reason.into(),
            mode,
            requested_at_ms: now,
        });
        Ok(record.clone())
    }

    pub fn termination_request(&self, child_run_id: &str) -> Option<ChildRunTerminationRequest> {
        self.inner
            .read()
            .expect("child run registry poisoned")
            .get(child_run_id)
            .and_then(|record| record.termination.clone())
    }

    #[cfg(test)]
    pub fn clear(&self) {
        self.inner
            .write()
            .expect("child run registry poisoned")
            .clear();
    }

    fn update(&self, child_run_id: &str, update: impl FnOnce(&mut ChildRunRecord, u64)) {
        let mut records = self.inner.write().expect("child run registry poisoned");
        if let Some(record) = records.get_mut(child_run_id) {
            let now = now_ms();
            update(record, now);
            record.updated_at_ms = now;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildRunRegistryError {
    NotFound,
    AlreadyTerminal(ChildRunRecord),
}

static GLOBAL_CHILD_RUN_REGISTRY: OnceLock<ChildRunRegistry> = OnceLock::new();

pub fn global_child_run_registry() -> &'static ChildRunRegistry {
    GLOBAL_CHILD_RUN_REGISTRY.get_or_init(ChildRunRegistry::default)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record(child_run_id: &str, parent_session_id: &str) -> ChildRunRecord {
        ChildRunRecord::new(
            child_run_id.to_string(),
            parent_session_id.to_string(),
            format!("child-session-{child_run_id}"),
            Some("/tmp/workspace".to_string()),
            Some("/tmp/workspace/.alan/sessions/child.jsonl".to_string()),
            Some("repo-coding".to_string()),
        )
    }

    #[test]
    fn registry_tracks_liveness_progress_and_terminal_state() {
        let registry = ChildRunRegistry::default();
        registry.register(test_record("run-1", "parent-1"));
        registry.register(test_record("run-2", "parent-2"));

        registry.mark_running("run-1");
        registry.observe_heartbeat("run-1", Some("alive".to_string()));
        registry.observe_progress(
            "run-1",
            "tool_call_started",
            Some("running bash".to_string()),
        );
        registry.observe_warning("run-1", "first warning".to_string());

        let parent_runs = registry.list_for_parent("parent-1");
        assert_eq!(parent_runs.len(), 1);
        assert_eq!(parent_runs[0].id, "run-1");

        let running = registry.get_for_parent("parent-1", "run-1").unwrap();
        assert_eq!(running.status, ChildRunStatus::Running);
        assert_eq!(
            running.latest_event_kind.as_deref(),
            Some("tool_call_started")
        );
        assert_eq!(
            running.latest_status_summary.as_deref(),
            Some("running bash")
        );
        assert_eq!(running.warnings, vec!["first warning"]);
        assert!(running.latest_heartbeat_at_ms.is_some());
        assert!(running.latest_progress_at_ms.is_some());

        registry.mark_terminal("run-1", ChildRunStatus::Completed, None);
        let terminal = registry.get("run-1").unwrap();
        registry.observe_progress("run-1", "text_delta", Some("late update".to_string()));
        let after_late_progress = registry.get("run-1").unwrap();
        assert_eq!(after_late_progress.status, ChildRunStatus::Completed);
        assert_eq!(
            after_late_progress.latest_event_kind,
            terminal.latest_event_kind
        );
        assert_eq!(
            after_late_progress.latest_status_summary,
            terminal.latest_status_summary
        );
    }

    #[test]
    fn registry_records_termination_and_rejects_unknown_or_terminal_runs() {
        let registry = ChildRunRegistry::default();
        registry.register(test_record("run-1", "parent-1"));

        assert_eq!(
            registry.request_termination(
                "parent-2",
                "run-1",
                "parent_runtime",
                ChildRunTerminationMode::Graceful,
                "wrong parent",
            ),
            Err(ChildRunRegistryError::NotFound)
        );
        assert_eq!(
            registry.request_termination(
                "parent-1",
                "missing",
                "parent_runtime",
                ChildRunTerminationMode::Graceful,
                "missing child",
            ),
            Err(ChildRunRegistryError::NotFound)
        );

        let terminating = registry
            .request_termination(
                "parent-1",
                "run-1",
                "parent_runtime",
                ChildRunTerminationMode::Forceful,
                "operator requested stop",
            )
            .unwrap();
        assert_eq!(terminating.status, ChildRunStatus::Terminating);
        assert_eq!(
            terminating.termination.as_ref().map(|request| (
                request.actor.as_str(),
                request.reason.as_str(),
                request.mode
            )),
            Some((
                "parent_runtime",
                "operator requested stop",
                ChildRunTerminationMode::Forceful
            ))
        );

        registry.mark_terminal("run-1", ChildRunStatus::Terminated, None);
        match registry.request_termination(
            "parent-1",
            "run-1",
            "parent_runtime",
            ChildRunTerminationMode::Graceful,
            "duplicate",
        ) {
            Err(ChildRunRegistryError::AlreadyTerminal(record)) => {
                assert_eq!(record.status, ChildRunStatus::Terminated);
                assert_eq!(
                    record
                        .termination
                        .as_ref()
                        .map(|request| request.reason.as_str()),
                    Some("operator requested stop")
                );
            }
            other => panic!("expected already-terminal error, got {other:?}"),
        }
    }
}
