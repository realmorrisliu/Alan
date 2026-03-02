//! Durable task/run/schedule persistence for autonomous execution.
//!
//! This module provides:
//! - canonical Task/Run/ScheduleItem data models
//! - durable CRUD + auditable status transitions
//! - pluggable backend abstraction (JSON backend included)
//! - explicit schema version gating/migration policy

#![cfg_attr(not(test), allow(dead_code))]

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::debug;

const TASK_STORE_SCHEMA_VERSION: u32 = 1;
const TASK_STORE_FILENAME: &str = "task_store.v1.json";

/// Task lifecycle status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Run lifecycle status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Sleeping,
    Yielded,
    Succeeded,
    Failed,
    Cancelled,
}

impl RunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Sleeping => "sleeping",
            Self::Yielded => "yielded",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Schedule trigger category.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleTriggerType {
    At,
    Interval,
    RetryBackoff,
}

/// Schedule item lifecycle status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleStatus {
    Waiting,
    Due,
    Dispatching,
    Cancelled,
    Completed,
    Failed,
}

impl ScheduleStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Due => "due",
            Self::Dispatching => "dispatching",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Auditable status transition entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusTransition {
    pub from: String,
    pub to: String,
    pub changed_at: String,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl StatusTransition {
    fn new(from: &str, to: &str, actor: &str, reason: Option<String>, changed_at: String) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            changed_at,
            actor: actor.to_string(),
            reason,
        }
    }
}

/// Durable task entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskRecord {
    pub task_id: String,
    pub goal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_criteria: Option<String>,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_history: Vec<StatusTransition>,
}

impl TaskRecord {
    pub fn new(task_id: impl Into<String>, goal: impl Into<String>) -> Self {
        let now = now_rfc3339();
        Self {
            task_id: task_id.into(),
            goal: goal.into(),
            owner: None,
            constraints: None,
            success_criteria: None,
            status: TaskStatus::Open,
            created_at: now.clone(),
            updated_at: now,
            transition_history: Vec::new(),
        }
    }
}

/// Durable run entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunRecord {
    pub run_id: String,
    pub task_id: String,
    pub attempt: u32,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_wake_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_history: Vec<StatusTransition>,
}

impl RunRecord {
    pub fn new(run_id: impl Into<String>, task_id: impl Into<String>, attempt: u32) -> Self {
        let now = now_rfc3339();
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            attempt,
            status: RunStatus::Pending,
            started_at: None,
            ended_at: None,
            last_checkpoint_id: None,
            next_wake_at: None,
            created_at: now.clone(),
            updated_at: now,
            transition_history: Vec::new(),
        }
    }
}

/// Durable schedule item entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScheduleItemRecord {
    pub schedule_id: String,
    pub task_id: String,
    pub run_id: String,
    pub trigger_type: ScheduleTriggerType,
    pub next_wake_at: String,
    pub status: ScheduleStatus,
    pub attempt: u32,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_history: Vec<StatusTransition>,
}

impl ScheduleItemRecord {
    pub fn new(
        schedule_id: impl Into<String>,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        trigger_type: ScheduleTriggerType,
        next_wake_at: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        let now = now_rfc3339();
        Self {
            schedule_id: schedule_id.into(),
            task_id: task_id.into(),
            run_id: run_id.into(),
            trigger_type,
            next_wake_at: next_wake_at.into(),
            status: ScheduleStatus::Waiting,
            attempt: 0,
            idempotency_key: idempotency_key.into(),
            created_at: now.clone(),
            updated_at: now,
            transition_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskStoreState {
    #[serde(default)]
    tasks: BTreeMap<String, TaskRecord>,
    #[serde(default)]
    runs: BTreeMap<String, RunRecord>,
    #[serde(default)]
    schedules: BTreeMap<String, ScheduleItemRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskStoreSnapshot {
    schema_version: u32,
    updated_at: String,
    #[serde(default)]
    state: TaskStoreState,
}

impl TaskStoreSnapshot {
    fn from_state(state: TaskStoreState) -> Self {
        Self {
            schema_version: TASK_STORE_SCHEMA_VERSION,
            updated_at: now_rfc3339(),
            state,
        }
    }
}

/// Persistence backend abstraction.
///
/// This allows swapping JSON storage for SQLite or other engines later.
pub(crate) trait TaskStoreBackend: Send + Sync {
    fn load_snapshot(&self) -> Result<Option<TaskStoreSnapshot>>;
    fn save_snapshot(&self, snapshot: &TaskStoreSnapshot) -> Result<()>;
}

/// JSON-file backend for `TaskStore`.
#[derive(Debug, Clone)]
pub(crate) struct JsonFileTaskStoreBackend {
    path: PathBuf,
}

impl JsonFileTaskStoreBackend {
    pub fn with_file(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn with_storage_dir(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            path: storage_dir.as_ref().join(TASK_STORE_FILENAME),
        }
    }

    #[allow(dead_code)]
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".alan").join("tasks").join(TASK_STORE_FILENAME))
    }

    fn write_atomically(&self, content: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create task_store parent dir: {}",
                    parent.display()
                )
            })?;
        }

        let tmp_path = self.path.with_extension("json.tmp");
        let mut tmp_file = std::fs::File::create(&tmp_path).with_context(|| {
            format!(
                "Failed to create temp task_store file: {}",
                tmp_path.display()
            )
        })?;
        tmp_file.write_all(content.as_bytes()).with_context(|| {
            format!(
                "Failed to write temp task_store file: {}",
                tmp_path.display()
            )
        })?;
        tmp_file.sync_all().with_context(|| {
            format!(
                "Failed to fsync temp task_store file: {}",
                tmp_path.display()
            )
        })?;

        std::fs::rename(&tmp_path, &self.path).with_context(|| {
            format!(
                "Failed to atomically replace task_store file {} -> {}",
                tmp_path.display(),
                self.path.display()
            )
        })?;

        if let Some(parent) = self.path.parent() {
            sync_directory(parent)?;
        }

        Ok(())
    }
}

impl TaskStoreBackend for JsonFileTaskStoreBackend {
    fn load_snapshot(&self) -> Result<Option<TaskStoreSnapshot>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read task_store file {}", self.path.display()))?;
        let snapshot = serde_json::from_str::<TaskStoreSnapshot>(&content)
            .with_context(|| format!("Failed to parse task_store file {}", self.path.display()))?;
        Ok(Some(snapshot))
    }

    fn save_snapshot(&self, snapshot: &TaskStoreSnapshot) -> Result<()> {
        let content = serde_json::to_string_pretty(snapshot)
            .context("Failed to serialize task_store snapshot")?;
        self.write_atomically(&content)
    }
}

/// Durable task store with pluggable backend.
#[derive(Debug)]
pub(crate) struct TaskStore<B: TaskStoreBackend> {
    backend: B,
    state: RwLock<TaskStoreState>,
}

impl TaskStore<JsonFileTaskStoreBackend> {
    #[allow(dead_code)]
    pub fn new_default() -> Result<Self> {
        let path = JsonFileTaskStoreBackend::default_path()?;
        Self::new(JsonFileTaskStoreBackend::with_file(path))
    }

    #[cfg(test)]
    pub fn with_dir(storage_dir: impl AsRef<Path>) -> Result<Self> {
        Self::new(JsonFileTaskStoreBackend::with_storage_dir(storage_dir))
    }
}

impl<B: TaskStoreBackend> TaskStore<B> {
    pub fn new(backend: B) -> Result<Self> {
        let state = match backend.load_snapshot()? {
            Some(snapshot) => decode_snapshot(snapshot)?,
            None => TaskStoreState::default(),
        };
        Ok(Self {
            backend,
            state: RwLock::new(state),
        })
    }

    pub fn migration_policy() -> &'static str {
        "Strict schema-version gating: only schema_version=1 is accepted. Older/newer versions must be migrated explicitly before loading."
    }

    pub fn save_task(&self, record: TaskRecord) -> Result<()> {
        self.apply_mutation(|state| {
            state.tasks.insert(record.task_id.clone(), record);
            Ok(())
        })
    }

    pub fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.tasks.get(task_id).cloned())
    }

    #[allow(dead_code)]
    pub fn list_tasks(&self) -> Result<Vec<TaskRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.tasks.values().cloned().collect())
    }

    pub fn transition_task_status(
        &self,
        task_id: &str,
        to: TaskStatus,
        actor: &str,
        reason: Option<String>,
    ) -> Result<TaskRecord> {
        self.apply_mutation(|state| {
            let task = state
                .tasks
                .get_mut(task_id)
                .with_context(|| format!("Task not found: {task_id}"))?;
            if task.status != to {
                let now = now_rfc3339();
                task.transition_history.push(StatusTransition::new(
                    task.status.as_str(),
                    to.as_str(),
                    actor,
                    reason,
                    now.clone(),
                ));
                task.status = to;
                task.updated_at = now;
            }
            Ok(task.clone())
        })
    }

    pub fn save_run(&self, record: RunRecord) -> Result<()> {
        self.apply_mutation(|state| {
            state.runs.insert(record.run_id.clone(), record);
            Ok(())
        })
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.runs.get(run_id).cloned())
    }

    #[allow(dead_code)]
    pub fn list_runs(&self) -> Result<Vec<RunRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.runs.values().cloned().collect())
    }

    pub fn transition_run_status(
        &self,
        run_id: &str,
        to: RunStatus,
        actor: &str,
        reason: Option<String>,
    ) -> Result<RunRecord> {
        self.apply_mutation(|state| {
            let run = state
                .runs
                .get_mut(run_id)
                .with_context(|| format!("Run not found: {run_id}"))?;
            if run.status != to {
                let now = now_rfc3339();
                run.transition_history.push(StatusTransition::new(
                    run.status.as_str(),
                    to.as_str(),
                    actor,
                    reason,
                    now.clone(),
                ));
                run.status = to;
                run.updated_at = now;
            }
            Ok(run.clone())
        })
    }

    pub fn save_schedule_item(&self, record: ScheduleItemRecord) -> Result<()> {
        self.apply_mutation(|state| {
            state.schedules.insert(record.schedule_id.clone(), record);
            Ok(())
        })
    }

    pub fn get_schedule_item(&self, schedule_id: &str) -> Result<Option<ScheduleItemRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.schedules.get(schedule_id).cloned())
    }

    #[allow(dead_code)]
    pub fn list_schedule_items(&self) -> Result<Vec<ScheduleItemRecord>> {
        let guard = self.state.read().map_err(lock_poisoned)?;
        Ok(guard.schedules.values().cloned().collect())
    }

    pub fn transition_schedule_status(
        &self,
        schedule_id: &str,
        to: ScheduleStatus,
        actor: &str,
        reason: Option<String>,
    ) -> Result<ScheduleItemRecord> {
        self.apply_mutation(|state| {
            let schedule = state
                .schedules
                .get_mut(schedule_id)
                .with_context(|| format!("Schedule item not found: {schedule_id}"))?;
            if schedule.status != to {
                let now = now_rfc3339();
                schedule.transition_history.push(StatusTransition::new(
                    schedule.status.as_str(),
                    to.as_str(),
                    actor,
                    reason,
                    now.clone(),
                ));
                schedule.status = to;
                schedule.updated_at = now;
            }
            Ok(schedule.clone())
        })
    }

    fn apply_mutation<T, F>(&self, mutate: F) -> Result<T>
    where
        F: FnOnce(&mut TaskStoreState) -> Result<T>,
    {
        let mut guard = self.state.write().map_err(lock_poisoned)?;
        let mut staged = guard.clone();
        let result = mutate(&mut staged)?;
        self.persist_locked(&staged)?;
        *guard = staged;
        Ok(result)
    }

    fn persist_locked(&self, state: &TaskStoreState) -> Result<()> {
        let snapshot = TaskStoreSnapshot::from_state(state.clone());
        self.backend.save_snapshot(&snapshot)?;
        debug!(
            tasks = snapshot.state.tasks.len(),
            runs = snapshot.state.runs.len(),
            schedules = snapshot.state.schedules.len(),
            "Persisted task_store snapshot"
        );
        Ok(())
    }
}

fn decode_snapshot(snapshot: TaskStoreSnapshot) -> Result<TaskStoreState> {
    if snapshot.schema_version != TASK_STORE_SCHEMA_VERSION {
        bail!(
            "Unsupported task_store schema_version {} (current {}). {}",
            snapshot.schema_version,
            TASK_STORE_SCHEMA_VERSION,
            TaskStore::<JsonFileTaskStoreBackend>::migration_policy()
        );
    }
    Ok(snapshot.state)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn lock_poisoned<T>(err: std::sync::PoisonError<T>) -> anyhow::Error {
    anyhow::anyhow!("task_store lock poisoned: {err}")
}

fn sync_directory(path: &Path) -> Result<()> {
    let dir = std::fs::File::open(path).with_context(|| {
        format!(
            "Failed to open task_store parent dir for fsync: {}",
            path.display()
        )
    })?;
    dir.sync_all()
        .with_context(|| format!("Failed to fsync task_store parent dir: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[test]
    fn json_store_persists_task_run_schedule_across_reopen() {
        let temp = TempDir::new().unwrap();
        let store = TaskStore::with_dir(temp.path()).unwrap();

        store
            .save_task(TaskRecord::new("task-1", "Implement scheduler"))
            .unwrap();
        store
            .save_run(RunRecord::new("run-1", "task-1", 1))
            .unwrap();
        store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-1",
                "task-1",
                "run-1",
                ScheduleTriggerType::At,
                "2026-03-03T10:00:00Z",
                "idem-1",
            ))
            .unwrap();

        store
            .transition_task_status("task-1", TaskStatus::Running, "daemon", None)
            .unwrap();
        store
            .transition_run_status("run-1", RunStatus::Running, "daemon", None)
            .unwrap();
        store
            .transition_schedule_status("sch-1", ScheduleStatus::Due, "scheduler", None)
            .unwrap();

        let reopened = TaskStore::with_dir(temp.path()).unwrap();
        let task = reopened.get_task("task-1").unwrap().unwrap();
        let run = reopened.get_run("run-1").unwrap().unwrap();
        let schedule = reopened.get_schedule_item("sch-1").unwrap().unwrap();

        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(schedule.status, ScheduleStatus::Due);
        assert_eq!(task.transition_history.len(), 1);
        assert_eq!(run.transition_history.len(), 1);
        assert_eq!(schedule.transition_history.len(), 1);
    }

    #[test]
    fn status_transitions_are_auditable() {
        let temp = TempDir::new().unwrap();
        let store = TaskStore::with_dir(temp.path()).unwrap();

        store
            .save_task(TaskRecord::new("task-audit", "Audit transitions"))
            .unwrap();
        let updated = store
            .transition_task_status(
                "task-audit",
                TaskStatus::Running,
                "scheduler",
                Some("task dequeued".to_string()),
            )
            .unwrap();

        assert_eq!(updated.transition_history.len(), 1);
        let transition = &updated.transition_history[0];
        assert_eq!(transition.from, "open");
        assert_eq!(transition.to, "running");
        assert_eq!(transition.actor, "scheduler");
        assert_eq!(transition.reason.as_deref(), Some("task dequeued"));
        assert!(!transition.changed_at.is_empty());
    }

    #[test]
    fn schema_version_mismatch_is_rejected_with_explicit_policy_message() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join(TASK_STORE_FILENAME);
        let bad_snapshot = serde_json::json!({
            "schema_version": 99,
            "updated_at": "2026-03-02T00:00:00Z",
            "state": { "tasks": {}, "runs": {}, "schedules": {} }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&bad_snapshot).unwrap()).unwrap();

        let backend = JsonFileTaskStoreBackend::with_file(path);
        let err = TaskStore::new(backend).unwrap_err().to_string();
        assert!(err.contains("Unsupported task_store schema_version"));
        assert!(err.contains("Strict schema-version gating"));
    }

    #[test]
    fn sync_directory_succeeds_for_existing_directory() {
        let temp = TempDir::new().unwrap();
        sync_directory(temp.path()).unwrap();
    }

    #[derive(Clone, Default)]
    struct FailableBackend {
        shared: Arc<Mutex<Option<TaskStoreSnapshot>>>,
        fail_saves: Arc<Mutex<bool>>,
    }

    impl FailableBackend {
        fn set_fail_saves(&self, fail: bool) {
            *self.fail_saves.lock().unwrap() = fail;
        }
    }

    impl TaskStoreBackend for FailableBackend {
        fn load_snapshot(&self) -> Result<Option<TaskStoreSnapshot>> {
            Ok(self.shared.lock().unwrap().clone())
        }

        fn save_snapshot(&self, snapshot: &TaskStoreSnapshot) -> Result<()> {
            if *self.fail_saves.lock().unwrap() {
                anyhow::bail!("simulated persistence failure");
            }
            *self.shared.lock().unwrap() = Some(snapshot.clone());
            Ok(())
        }
    }

    #[test]
    fn failed_persist_does_not_leak_save_mutations_into_memory() {
        let backend = FailableBackend::default();
        backend.set_fail_saves(true);
        let store = TaskStore::new(backend).unwrap();

        let task_err = store
            .save_task(TaskRecord::new("task-fail", "should rollback"))
            .unwrap_err()
            .to_string();
        let run_err = store
            .save_run(RunRecord::new("run-fail", "task-fail", 1))
            .unwrap_err()
            .to_string();
        let schedule_err = store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-fail",
                "task-fail",
                "run-fail",
                ScheduleTriggerType::At,
                "2026-03-03T10:00:00Z",
                "idem-fail",
            ))
            .unwrap_err()
            .to_string();

        assert!(task_err.contains("simulated persistence failure"));
        assert!(run_err.contains("simulated persistence failure"));
        assert!(schedule_err.contains("simulated persistence failure"));
        assert!(store.get_task("task-fail").unwrap().is_none());
        assert!(store.get_run("run-fail").unwrap().is_none());
        assert!(store.get_schedule_item("sch-fail").unwrap().is_none());
    }

    #[test]
    fn failed_persist_does_not_leak_transition_mutations_into_memory() {
        let backend = FailableBackend::default();
        let store = TaskStore::new(backend.clone()).unwrap();

        store
            .save_task(TaskRecord::new("task-rollback", "transition rollback"))
            .unwrap();
        store
            .save_run(RunRecord::new("run-rollback", "task-rollback", 1))
            .unwrap();
        store
            .save_schedule_item(ScheduleItemRecord::new(
                "sch-rollback",
                "task-rollback",
                "run-rollback",
                ScheduleTriggerType::At,
                "2026-03-03T10:00:00Z",
                "idem-rollback",
            ))
            .unwrap();

        backend.set_fail_saves(true);

        let task_err = store
            .transition_task_status("task-rollback", TaskStatus::Running, "daemon", None)
            .unwrap_err()
            .to_string();
        let run_err = store
            .transition_run_status("run-rollback", RunStatus::Running, "daemon", None)
            .unwrap_err()
            .to_string();
        let schedule_err = store
            .transition_schedule_status("sch-rollback", ScheduleStatus::Due, "scheduler", None)
            .unwrap_err()
            .to_string();

        assert!(task_err.contains("simulated persistence failure"));
        assert!(run_err.contains("simulated persistence failure"));
        assert!(schedule_err.contains("simulated persistence failure"));

        let task = store.get_task("task-rollback").unwrap().unwrap();
        let run = store.get_run("run-rollback").unwrap().unwrap();
        let schedule = store.get_schedule_item("sch-rollback").unwrap().unwrap();

        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(run.status, RunStatus::Pending);
        assert_eq!(schedule.status, ScheduleStatus::Waiting);
        assert!(task.transition_history.is_empty());
        assert!(run.transition_history.is_empty());
        assert!(schedule.transition_history.is_empty());
    }

    #[derive(Clone, Default)]
    struct InMemoryBackend {
        shared: Arc<Mutex<Option<TaskStoreSnapshot>>>,
    }

    impl TaskStoreBackend for InMemoryBackend {
        fn load_snapshot(&self) -> Result<Option<TaskStoreSnapshot>> {
            Ok(self.shared.lock().unwrap().clone())
        }

        fn save_snapshot(&self, snapshot: &TaskStoreSnapshot) -> Result<()> {
            *self.shared.lock().unwrap() = Some(snapshot.clone());
            Ok(())
        }
    }

    #[test]
    fn backend_abstraction_allows_replacement() {
        let backend = InMemoryBackend::default();
        let store = TaskStore::new(backend.clone()).unwrap();
        store
            .save_task(TaskRecord::new("task-abs", "pluggable backend"))
            .unwrap();

        let reopened = TaskStore::new(backend).unwrap();
        let task = reopened.get_task("task-abs").unwrap().unwrap();
        assert_eq!(task.goal, "pluggable backend");
    }
}
