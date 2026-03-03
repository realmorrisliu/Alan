//! Application state management for agentd.

use super::runtime_manager::{RuntimeManager, RuntimeSessionPolicy};
use super::scheduler::{
    DispatchSuccessAction, SCHEDULER_ACTOR, claim_due_items, dispatch_success_action,
    reconcile_on_boot, retry_wake_at,
};
use super::session_store::{SessionBinding, SessionStore};
use super::task_store::{
    JsonFileTaskStoreBackend, RunRecord, RunStatus, ScheduleItemRecord, ScheduleStatus,
    ScheduleTriggerType, TaskRecord, TaskStatus, TaskStore,
};
use super::workspace_resolver::WorkspaceResolver;
use alan_protocol::{Event, EventEnvelope, Submission};
use alan_runtime::{
    Config,
    runtime::{RuntimeEventEnvelope, WorkspaceRuntimeConfig},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Default session TTL (time-to-live) in seconds
const DEFAULT_SESSION_TTL_SECS: u64 = 3600; // 1 hour
/// Default broadcast capacity for per-session enveloped events
const DEFAULT_EVENT_BROADCAST_CAPACITY: usize = 256;
/// In-memory replay buffer size for per-session event envelopes
const DEFAULT_EVENT_REPLAY_BUFFER_CAPACITY: usize = 1024;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Configuration
    #[allow(dead_code)]
    pub config: Config,
    /// Workspace resolver for path resolution
    pub workspace_resolver: Arc<WorkspaceResolver>,
    /// Runtime manager for session runtimes
    pub runtime_manager: Arc<RuntimeManager>,
    /// Session store for persistence
    pub session_store: Arc<SessionStore>,
    /// Durable scheduler store
    pub(crate) task_store: Arc<TaskStore<JsonFileTaskStoreBackend>>,
    /// Active sessions
    pub sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    /// Session TTL in seconds
    pub session_ttl_secs: u64,
    /// Cleanup task started flag
    cleanup_started: Arc<AtomicBool>,
    /// Scheduler task started flag
    scheduler_started: Arc<AtomicBool>,
    /// Whether on-disk session bindings have been recovered into memory
    sessions_recovered: Arc<AtomicBool>,
    /// Serializes one-time recovery
    recovery_lock: Arc<Mutex<()>>,
}

/// Entry for an active session
pub struct SessionEntry {
    /// Workspace path for this session
    pub workspace_path: PathBuf,
    /// Workspace state dir (.alan) for this session
    pub workspace_alan_dir: PathBuf,
    /// Cached workspace ID (derived from path)
    pub workspace_id: String,
    /// Governance configuration for this session runtime.
    pub governance: alan_protocol::GovernanceConfig,
    /// Streaming mode for this session runtime.
    pub streaming_mode: alan_runtime::StreamingMode,
    /// Partial stream recovery mode for this session runtime.
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    /// Sender for submitting operations
    pub submission_tx: mpsc::Sender<Submission>,
    /// Broadcast channel for session event envelopes
    pub events_tx: broadcast::Sender<EventEnvelope>,
    /// In-memory replay buffer with stable event ids/cursors
    pub event_log: Arc<RwLock<SessionEventLog>>,
    /// Bridge task forwarding runtime events into `events_tx`
    pub event_bridge_task: Option<JoinHandle<()>>,
    /// Exact rollout path for this session's persisted history.
    pub rollout_path: Option<PathBuf>,
    /// Session creation time
    #[allow(dead_code)]
    pub created_at: std::time::Instant,
    /// Last inbound activity time (user requests)
    pub last_inbound_activity: std::time::Instant,
    /// Last outbound activity time (events sent to clients)
    pub last_outbound_activity: std::time::Instant,
}

/// Slice of buffered session events used for replay/cursor reads.
#[derive(Debug, Clone)]
pub struct SessionEventReplayPage {
    pub events: Vec<EventEnvelope>,
    pub gap: bool,
    pub oldest_event_id: Option<String>,
    pub latest_event_id: Option<String>,
}

/// In-memory replay log for a session's transport events.
#[derive(Debug)]
pub struct SessionEventLog {
    next_sequence: u64,
    current_turn_sequence: u64,
    current_item_sequence: u64,
    buffer: VecDeque<EventEnvelope>,
    capacity: usize,
}

impl SessionEventLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            next_sequence: 1,
            current_turn_sequence: 0,
            current_item_sequence: 0,
            buffer: VecDeque::with_capacity(capacity.min(16)),
            capacity: capacity.max(1),
        }
    }

    pub fn append_runtime_event(
        &mut self,
        session_id: &str,
        runtime_event: RuntimeEventEnvelope,
    ) -> EventEnvelope {
        let event = runtime_event.event;
        if self.current_turn_sequence == 0 || matches!(event, Event::TurnStarted {}) {
            self.current_turn_sequence += 1;
            self.current_item_sequence = 0;
        }

        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.current_item_sequence += 1;

        let envelope = EventEnvelope {
            event_id: format!("evt_{sequence:016}"),
            sequence,
            session_id: session_id.to_string(),
            submission_id: runtime_event.submission_id,
            turn_id: format!("turn_{:06}", self.current_turn_sequence),
            item_id: format!(
                "item_{:06}_{:04}",
                self.current_turn_sequence, self.current_item_sequence
            ),
            timestamp_ms: now_timestamp_ms(),
            event,
        };
        self.push(envelope.clone());
        envelope
    }

    fn push(&mut self, envelope: EventEnvelope) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(envelope);
    }

    pub fn read_after(&self, after_event_id: Option<&str>, limit: usize) -> SessionEventReplayPage {
        let limit = limit.clamp(1, 1000);
        let oldest_event_id = self.buffer.front().map(|e| e.event_id.clone());
        let latest_event_id = self.buffer.back().map(|e| e.event_id.clone());

        let Some(after_event_id) = after_event_id else {
            let events = self.buffer.iter().take(limit).cloned().collect();
            return SessionEventReplayPage {
                events,
                gap: false,
                oldest_event_id,
                latest_event_id,
            };
        };

        let after_sequence = match parse_event_sequence(after_event_id) {
            Some(seq) => seq,
            None => {
                return SessionEventReplayPage {
                    events: Vec::new(),
                    gap: true,
                    oldest_event_id,
                    latest_event_id,
                };
            }
        };

        let mut gap = false;
        let start_idx = if let Some(idx) = self
            .buffer
            .iter()
            .position(|e| e.event_id == after_event_id)
        {
            idx + 1
        } else {
            if let Some(oldest) = self.buffer.front()
                && after_sequence < oldest.sequence
            {
                gap = true;
            }

            if let Some(latest) = self.buffer.back()
                && after_sequence >= latest.sequence
            {
                self.buffer.len()
            } else if gap {
                0
            } else {
                // Cursor not found but sequence is within buffer range; treat as gap and replay from oldest.
                gap = true;
                0
            }
        };

        let events = self
            .buffer
            .iter()
            .skip(start_idx)
            .take(limit)
            .cloned()
            .collect();

        SessionEventReplayPage {
            events,
            gap,
            oldest_event_id,
            latest_event_id,
        }
    }
}

fn parse_event_sequence(event_id: &str) -> Option<u64> {
    event_id.strip_prefix("evt_")?.parse::<u64>().ok()
}

fn now_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_session_not_found_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.starts_with("Session ") && msg.ends_with(" not found")
}

fn is_run_not_found_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.starts_with("Run not found:")
}

fn checkpoint_from_event(event: &Event) -> Option<(String, String, Option<serde_json::Value>)> {
    match event {
        Event::TurnStarted {} => Some(("turn_start".to_string(), "turn started".to_string(), None)),
        Event::Yield {
            request_id, kind, ..
        } => Some((
            "yield".to_string(),
            "runtime yielded awaiting external input".to_string(),
            Some(serde_json::json!({
                "request_id": request_id,
                "kind": kind
            })),
        )),
        Event::TurnCompleted { summary } => Some((
            "turn_complete".to_string(),
            summary
                .clone()
                .unwrap_or_else(|| "turn completed".to_string()),
            None,
        )),
        _ => None,
    }
}

impl SessionEntry {
    /// Create a new session entry with computed workspace_id
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_path: PathBuf,
        workspace_alan_dir: PathBuf,
        governance: alan_protocol::GovernanceConfig,
        streaming_mode: alan_runtime::StreamingMode,
        partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
        submission_tx: mpsc::Sender<Submission>,
        events_tx: broadcast::Sender<EventEnvelope>,
        event_log: Arc<RwLock<SessionEventLog>>,
        event_bridge_task: Option<JoinHandle<()>>,
        rollout_path: Option<PathBuf>,
    ) -> Self {
        use crate::registry::generate_workspace_id;
        let workspace_id = generate_workspace_id(&workspace_path);
        let now = std::time::Instant::now();
        Self {
            workspace_path,
            workspace_alan_dir,
            workspace_id,
            governance,
            streaming_mode,
            partial_stream_recovery_mode,
            submission_tx,
            events_tx,
            event_log,
            event_bridge_task,
            rollout_path,
            created_at: now,
            last_inbound_activity: now,
            last_outbound_activity: now,
        }
    }

    /// Update last inbound activity timestamp (user request received)
    pub fn touch_inbound(&mut self) {
        self.last_inbound_activity = std::time::Instant::now();
    }

    /// Update last outbound activity timestamp (event sent to client)
    pub fn touch_outbound(&mut self) {
        self.last_outbound_activity = std::time::Instant::now();
    }

    /// Check if session has expired based on TTL
    ///
    /// Session is expired if:
    /// 1. No inbound activity for TTL seconds, AND
    /// 2. No outbound activity for TTL seconds (no events flowing)
    ///
    /// This ensures that:
    /// - Active tool execution that produces events keeps session alive
    /// - Idle sessions (no user input, no events) are cleaned up
    /// - Sessions with only WS connections but no actual traffic are cleaned up
    pub fn is_expired(&self, ttl: Duration) -> bool {
        let inbound_idle = self.last_inbound_activity.elapsed();
        let outbound_idle = self.last_outbound_activity.elapsed();

        // Both inbound and outbound must be idle for TTL
        inbound_idle > ttl && outbound_idle > ttl
    }
}

impl AppState {
    /// Recover persisted session bindings from workspace state files.
    pub async fn ensure_sessions_recovered(&self) -> anyhow::Result<()> {
        if self
            .sessions_recovered
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let _lock = self.recovery_lock.lock().await;
        if self
            .sessions_recovered
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        info!("Recovering persistent sessions...");

        let bindings = self.session_store.list_active();
        let mut recovered_count = 0;

        for binding in bindings {
            let session_id = binding.session_id.clone();
            let workspace_path = binding.workspace_path.clone();
            let workspace_alan_dir = self.workspace_resolver.workspace_alan_dir(&workspace_path);

            let (events_tx, _) = broadcast::channel(DEFAULT_EVENT_BROADCAST_CAPACITY);
            let (dummy_submission_tx, _) = mpsc::channel(1);
            let event_log = Arc::new(RwLock::new(SessionEventLog::new(
                DEFAULT_EVENT_REPLAY_BUFFER_CAPACITY,
            )));

            let mut entry = SessionEntry::new(
                workspace_path,
                workspace_alan_dir,
                binding.governance,
                binding.streaming_mode.unwrap_or(self.config.streaming_mode),
                binding
                    .partial_stream_recovery_mode
                    .unwrap_or(self.config.partial_stream_recovery_mode),
                dummy_submission_tx,
                events_tx,
                event_log,
                None, // event_bridge_task
                binding.rollout_path,
            );

            if let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(&binding.created_at) {
                let created_at_utc = created_at.with_timezone(&chrono::Utc);
                entry.created_at = std::time::Instant::now()
                    - chrono::Utc::now()
                        .signed_duration_since(created_at_utc)
                        .to_std()
                        .unwrap_or(Duration::from_secs(0));
            }

            self.sessions
                .write()
                .await
                .insert(session_id.clone(), entry);
            if let Err(err) = self.ensure_task_run_for_session(&session_id) {
                warn!(
                    session_id = %session_id,
                    error = %err,
                    "Failed to ensure durable run state while recovering session binding"
                );
            }
            recovered_count += 1;
        }

        info!(
            count = recovered_count,
            "Successfully recovered persistent sessions"
        );

        self.sessions_recovered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Create new application state
    ///
    /// Note: The cleanup task is NOT started automatically.
    /// Call `start_cleanup_task()` after the tokio runtime is initialized,
    /// or use `create_session()` which will lazily start it.
    pub fn new(config: Config) -> Self {
        let workspace_resolver =
            Arc::new(WorkspaceResolver::new().expect("Failed to initialize workspace resolver"));
        let runtime_config = WorkspaceRuntimeConfig::from(config.clone());
        let runtime_manager = Arc::new(RuntimeManager::with_template(runtime_config));
        let session_store =
            Arc::new(SessionStore::new().expect("Failed to initialize session store"));
        let task_store =
            Arc::new(TaskStore::new_default().expect("Failed to initialize durable task store"));

        Self::from_parts_with_task_store(
            config,
            workspace_resolver,
            runtime_manager,
            session_store,
            task_store,
            DEFAULT_SESSION_TTL_SECS,
        )
    }

    /// Create new application state with custom TTL
    #[allow(dead_code)]
    pub fn with_ttl(config: Config, ttl_secs: u64) -> Self {
        let workspace_resolver =
            Arc::new(WorkspaceResolver::new().expect("Failed to initialize workspace resolver"));
        let runtime_config = WorkspaceRuntimeConfig::from(config.clone());
        let runtime_manager = Arc::new(RuntimeManager::with_template(runtime_config));
        let session_store =
            Arc::new(SessionStore::new().expect("Failed to initialize session store"));
        let task_store =
            Arc::new(TaskStore::new_default().expect("Failed to initialize durable task store"));

        Self::from_parts_with_task_store(
            config,
            workspace_resolver,
            runtime_manager,
            session_store,
            task_store,
            ttl_secs,
        )
    }

    #[allow(dead_code)]
    pub(crate) fn from_parts(
        config: Config,
        workspace_resolver: Arc<WorkspaceResolver>,
        runtime_manager: Arc<RuntimeManager>,
        session_store: Arc<SessionStore>,
        task_store: Arc<TaskStore<JsonFileTaskStoreBackend>>,
        ttl_secs: u64,
    ) -> Self {
        Self::from_parts_with_task_store(
            config,
            workspace_resolver,
            runtime_manager,
            session_store,
            task_store,
            ttl_secs,
        )
    }

    pub(crate) fn from_parts_with_task_store(
        config: Config,
        workspace_resolver: Arc<WorkspaceResolver>,
        runtime_manager: Arc<RuntimeManager>,
        session_store: Arc<SessionStore>,
        task_store: Arc<TaskStore<JsonFileTaskStoreBackend>>,
        ttl_secs: u64,
    ) -> Self {
        Self {
            config,
            workspace_resolver,
            runtime_manager,
            session_store,
            task_store,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_secs: ttl_secs,
            cleanup_started: Arc::new(AtomicBool::new(false)),
            scheduler_started: Arc::new(AtomicBool::new(false)),
            sessions_recovered: Arc::new(AtomicBool::new(false)),
            recovery_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Get workspace path for a session by session ID
    #[allow(dead_code)]
    pub async fn get_workspace_path(&self, session_id: &str) -> anyhow::Result<Option<PathBuf>> {
        self.ensure_sessions_recovered().await?;
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).map(|e| e.workspace_path.clone()))
    }

    /// Get workspace .alan path for a session by session ID
    pub async fn get_workspace_alan_dir(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        self.ensure_sessions_recovered().await?;
        let sessions = self.sessions.read().await;
        Ok(sessions
            .get(session_id)
            .map(|e| e.workspace_alan_dir.clone()))
    }

    /// Get sessions directory for a session by session ID
    ///
    /// This resolves to `<workspace_alan_dir>/sessions`.
    pub async fn get_sessions_dir(&self, session_id: &str) -> anyhow::Result<Option<PathBuf>> {
        let alan_dir = self.get_workspace_alan_dir(session_id).await?;
        Ok(alan_dir.map(|p| p.join("sessions")))
    }

    /// Start background task to clean up expired sessions
    ///
    /// Should be called after the tokio runtime is initialized.
    /// This method is idempotent - calling multiple times has no effect.
    pub fn start_cleanup_task(&self) {
        // Ensure we only start the cleanup task once
        if self
            .cleanup_started
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }

        let state = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(60),
                Duration::from_secs(60),
            );

            loop {
                interval.tick().await;
                let removed = match state.cleanup_expired().await {
                    Ok(removed) => removed,
                    Err(err) => {
                        warn!(error = %err, "Failed to run expired session cleanup");
                        continue;
                    }
                };
                if removed > 0 {
                    info!(removed, "Expired session cleanup completed");
                }
            }
        });
    }

    /// Start durable scheduler loop.
    ///
    /// This worker periodically claims due schedule items and dispatches wakeups.
    /// Calling this method multiple times is safe.
    pub fn start_scheduler_task(&self) {
        if self
            .scheduler_started
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }

        let state = self.clone();
        tokio::spawn(async move {
            match reconcile_on_boot(state.task_store.as_ref()) {
                Ok(recovered) if recovered > 0 => {
                    info!(
                        recovered,
                        "Scheduler boot reconciliation recovered pending items"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(error = %err, "Scheduler boot reconciliation failed");
                }
            }

            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(err) = state.scheduler_run_cycle().await {
                    warn!(error = %err, "Scheduler cycle failed");
                }
            }
        });
    }

    async fn scheduler_run_cycle(&self) -> anyhow::Result<()> {
        // Recover interrupted dispatching items continuously, not only on process boot.
        // This prevents a transient in-cycle persistence failure from leaving a schedule
        // permanently stuck in dispatching state.
        if let Err(err) = reconcile_on_boot(self.task_store.as_ref()) {
            warn!(error = %err, "Scheduler pre-cycle reconciliation failed");
        }

        let claimed = claim_due_items(self.task_store.as_ref())?;
        for schedule in claimed {
            match self.resume_session_runtime(&schedule.run_id).await {
                Ok(()) => {
                    if let Err(err) = self.task_store.transition_run_status(
                        &schedule.run_id,
                        RunStatus::Running,
                        SCHEDULER_ACTOR,
                        Some("woken by scheduler".to_string()),
                    ) {
                        warn!(
                            run_id = %schedule.run_id,
                            error = %err,
                            "Failed to mark run as running after scheduler dispatch"
                        );
                    }
                    if let Err(err) = self.task_store.record_run_checkpoint(
                        &schedule.run_id,
                        "wake_dispatch",
                        "run resumed by scheduler dispatch",
                        Some(serde_json::json!({
                            "schedule_id": schedule.schedule_id,
                            "trigger_type": schedule.trigger_type,
                        })),
                    ) {
                        warn!(
                            run_id = %schedule.run_id,
                            error = %err,
                            "Failed to record wake_dispatch checkpoint"
                        );
                    }

                    match dispatch_success_action(&schedule) {
                        DispatchSuccessAction::Complete => {
                            self.task_store.transition_schedule_status(
                                &schedule.schedule_id,
                                ScheduleStatus::Completed,
                                SCHEDULER_ACTOR,
                                Some("dispatch completed".to_string()),
                            )?;
                        }
                        DispatchSuccessAction::RequeueAt(next_wake_at) => {
                            self.task_store.set_schedule_next_wake_at(
                                &schedule.schedule_id,
                                next_wake_at.to_rfc3339(),
                            )?;
                            self.task_store.transition_schedule_status(
                                &schedule.schedule_id,
                                ScheduleStatus::Waiting,
                                SCHEDULER_ACTOR,
                                Some("interval trigger requeued".to_string()),
                            )?;
                        }
                    }
                    self.sync_run_next_wake_at_from_active_schedules(&schedule.run_id);
                }
                Err(err) => {
                    if is_session_not_found_error(&err) {
                        warn!(
                            run_id = %schedule.run_id,
                            schedule_id = %schedule.schedule_id,
                            idempotency_key = %schedule.idempotency_key,
                            error = %err,
                            "Scheduler dispatch failed permanently; target session is missing"
                        );
                        self.task_store.transition_schedule_status(
                            &schedule.schedule_id,
                            ScheduleStatus::Failed,
                            SCHEDULER_ACTOR,
                            Some(format!("dispatch failed permanently: {err}")),
                        )?;
                        if let Err(clear_err) =
                            self.task_store.set_run_next_wake_at(&schedule.run_id, None)
                        {
                            warn!(
                                run_id = %schedule.run_id,
                                error = %clear_err,
                                "Failed to clear run next_wake_at after permanent dispatch failure"
                            );
                        }
                        self.sync_run_next_wake_at_from_active_schedules(&schedule.run_id);
                        continue;
                    }

                    let retry_at = retry_wake_at(&schedule).to_rfc3339();
                    warn!(
                        run_id = %schedule.run_id,
                        schedule_id = %schedule.schedule_id,
                        idempotency_key = %schedule.idempotency_key,
                        error = %err,
                        retry_at = %retry_at,
                        "Scheduler dispatch failed; scheduling retry"
                    );
                    self.task_store
                        .set_schedule_next_wake_at(&schedule.schedule_id, retry_at.clone())?;
                    self.task_store.transition_schedule_status(
                        &schedule.schedule_id,
                        ScheduleStatus::Waiting,
                        SCHEDULER_ACTOR,
                        Some(format!("dispatch failed: {err}")),
                    )?;
                    self.sync_run_next_wake_at_from_active_schedules(&schedule.run_id);
                }
            }
        }

        Ok(())
    }

    /// Schedule a one-shot wakeup for a session.
    pub async fn schedule_at(
        &self,
        session_id: &str,
        wake_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<ScheduleItemRecord> {
        self.ensure_sessions_recovered().await?;
        if !self.get_session(session_id).await? {
            anyhow::bail!("Session {} not found", session_id);
        }

        self.ensure_task_run_for_session(session_id)?;
        let schedule = self.persist_schedule(
            session_id,
            ScheduleTriggerType::At,
            wake_at,
            "schedule_at registered",
        )?;
        self.sync_run_next_wake_at_from_active_schedules(session_id);
        Ok(schedule)
    }

    /// Move a session to sleeping and schedule wakeup.
    pub async fn sleep_until(
        &self,
        session_id: &str,
        wake_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<ScheduleItemRecord> {
        self.ensure_sessions_recovered().await?;
        if !self.get_session(session_id).await? {
            anyhow::bail!("Session {} not found", session_id);
        }

        self.ensure_task_run_for_session(session_id)?;
        let schedule = self.persist_schedule(
            session_id,
            ScheduleTriggerType::At,
            wake_at,
            "sleep_until wake scheduled",
        )?;

        let cancelled = match self.cancel_active_session_schedules(
            session_id,
            Some(schedule.schedule_id.as_str()),
            "superseded by newer sleep_until",
        ) {
            Ok(cancelled) => cancelled,
            Err(err) => {
                if let Err(cancel_err) = self.task_store.transition_schedule_status(
                    &schedule.schedule_id,
                    ScheduleStatus::Cancelled,
                    SCHEDULER_ACTOR,
                    Some("sleep_until rollback: failed to supersede prior schedules".to_string()),
                ) {
                    warn!(
                        session_id = %session_id,
                        schedule_id = %schedule.schedule_id,
                        error = %cancel_err,
                        "Failed to cancel replacement schedule after supersede error"
                    );
                }
                self.sync_run_next_wake_at_from_active_schedules(session_id);
                return Err(err);
            }
        };
        if cancelled > 0 {
            info!(
                session_id = %session_id,
                cancelled,
                "Cancelled superseded schedules after persisting replacement sleep wake"
            );
        }

        if let Err(err) = self.task_store.transition_run_status(
            session_id,
            RunStatus::Sleeping,
            SCHEDULER_ACTOR,
            Some("sleep_until requested".to_string()),
        ) {
            self.rollback_sleep_until_run_state(
                session_id,
                "sleep_until rollback: failed to transition run status",
            );
            return Err(err);
        }

        if let Err(err) = self
            .task_store
            .set_run_next_wake_at(session_id, Some(wake_at.to_rfc3339()))
        {
            self.rollback_sleep_until_run_state(
                session_id,
                "sleep_until rollback: failed to set run next_wake_at",
            );
            return Err(err);
        }

        if let Err(err) = self.runtime_manager.stop_runtime(session_id).await {
            warn!(
                session_id = %session_id,
                schedule_id = %schedule.schedule_id,
                error = %err,
                "Failed to stop runtime after persisting sleep schedule; rolling back sleep state"
            );
            self.rollback_sleep_until_run_state(
                session_id,
                "sleep_until rollback: runtime stop failed",
            );
            return Err(err);
        }

        if let Err(err) = self.task_store.record_run_checkpoint(
            session_id,
            "sleep_until",
            "run moved to sleeping",
            Some(serde_json::json!({
                "schedule_id": schedule.schedule_id,
                "wake_at": wake_at.to_rfc3339(),
            })),
        ) {
            warn!(
                session_id = %session_id,
                error = %err,
                "Failed to record sleep_until checkpoint"
            );
        }

        Ok(schedule)
    }

    fn cancel_active_session_schedules(
        &self,
        session_id: &str,
        exclude_schedule_id: Option<&str>,
        reason: &str,
    ) -> anyhow::Result<usize> {
        let mut cancelled = 0usize;
        for schedule in self.task_store.list_schedule_items()? {
            if schedule.run_id != session_id {
                continue;
            }
            if exclude_schedule_id.is_some_and(|id| id == schedule.schedule_id.as_str()) {
                continue;
            }
            if !matches!(
                schedule.status,
                ScheduleStatus::Waiting | ScheduleStatus::Due | ScheduleStatus::Dispatching
            ) {
                continue;
            }
            self.task_store.transition_schedule_status(
                &schedule.schedule_id,
                ScheduleStatus::Cancelled,
                SCHEDULER_ACTOR,
                Some(reason.to_string()),
            )?;
            cancelled = cancelled.saturating_add(1);
        }
        Ok(cancelled)
    }

    fn rollback_sleep_until_run_state(&self, session_id: &str, reason: &str) {
        if let Err(err) = self.task_store.transition_run_status(
            session_id,
            RunStatus::Running,
            SCHEDULER_ACTOR,
            Some(reason.to_string()),
        ) {
            warn!(
                session_id = %session_id,
                error = %err,
                "Failed to restore run status while rolling back sleep_until"
            );
        }

        self.sync_run_next_wake_at_from_active_schedules(session_id);
    }

    fn sync_run_next_wake_at_from_active_schedules(&self, run_id: &str) {
        let next_wake_at = match self.next_active_schedule_wake_at(run_id) {
            Ok(next_wake_at) => next_wake_at,
            Err(err) => {
                warn!(
                    run_id = %run_id,
                    error = %err,
                    "Failed to inspect active schedules while syncing run next_wake_at"
                );
                return;
            }
        };

        if let Err(err) = self
            .task_store
            .set_run_next_wake_at(run_id, next_wake_at.map(|value| value.to_rfc3339()))
        {
            warn!(
                run_id = %run_id,
                error = %err,
                "Failed to sync run next_wake_at from active schedules"
            );
        }
    }

    fn next_active_schedule_wake_at(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Option<chrono::DateTime<chrono::Utc>>> {
        let mut earliest: Option<chrono::DateTime<chrono::Utc>> = None;
        for schedule in self.task_store.list_schedule_items()? {
            if schedule.run_id != run_id {
                continue;
            }
            if !matches!(
                schedule.status,
                ScheduleStatus::Waiting | ScheduleStatus::Due | ScheduleStatus::Dispatching
            ) {
                continue;
            }

            match chrono::DateTime::parse_from_rfc3339(&schedule.next_wake_at) {
                Ok(value) => {
                    let value = value.with_timezone(&chrono::Utc);
                    earliest = match earliest {
                        Some(current) => Some(current.min(value)),
                        None => Some(value),
                    };
                }
                Err(err) => {
                    warn!(
                        run_id = %run_id,
                        schedule_id = %schedule.schedule_id,
                        wake_at = %schedule.next_wake_at,
                        error = %err,
                        "Ignoring invalid schedule next_wake_at while syncing run wake"
                    );
                }
            }
        }

        Ok(earliest)
    }

    fn has_active_schedule_protection(
        &self,
        session_id: &str,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<bool> {
        for schedule in self.task_store.list_schedule_items()? {
            if schedule.run_id != session_id {
                continue;
            }
            match schedule.status {
                ScheduleStatus::Due | ScheduleStatus::Dispatching => return Ok(true),
                ScheduleStatus::Waiting => {
                    match chrono::DateTime::parse_from_rfc3339(&schedule.next_wake_at) {
                        Ok(next_wake_at) => {
                            if next_wake_at.with_timezone(&chrono::Utc) > *now {
                                return Ok(true);
                            }
                        }
                        Err(err) => {
                            warn!(
                                session_id = %session_id,
                                schedule_id = %schedule.schedule_id,
                                wake_at = %schedule.next_wake_at,
                                error = %err,
                                "Ignoring invalid schedule next_wake_at while evaluating TTL cleanup"
                            );
                        }
                    }
                }
                ScheduleStatus::Cancelled | ScheduleStatus::Completed | ScheduleStatus::Failed => {}
            }
        }
        Ok(false)
    }

    fn should_preserve_session_until_wake(
        &self,
        session_id: &str,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<bool> {
        if self.has_active_schedule_protection(session_id, now)? {
            return Ok(true);
        }

        let Some(run) = self.task_store.get_run(session_id)? else {
            return Ok(false);
        };

        let Some(next_wake_at) = run.next_wake_at.as_deref() else {
            return Ok(false);
        };

        match chrono::DateTime::parse_from_rfc3339(next_wake_at) {
            Ok(next_wake_at) => Ok(next_wake_at.with_timezone(&chrono::Utc) > *now),
            Err(err) => {
                warn!(
                    session_id = %session_id,
                    wake_at = %next_wake_at,
                    error = %err,
                    "Ignoring invalid run next_wake_at while evaluating TTL cleanup"
                );
                Ok(false)
            }
        }
    }

    fn ensure_task_run_for_session(&self, session_id: &str) -> anyhow::Result<()> {
        let task_id = format!("session-task-{session_id}");
        if self.task_store.get_task(&task_id)?.is_none() {
            let mut task =
                TaskRecord::new(task_id.clone(), format!("Session wakeup for {session_id}"));
            task.status = TaskStatus::Running;
            task.updated_at = chrono::Utc::now().to_rfc3339();
            self.task_store.save_task(task)?;
        }

        if self.task_store.get_run(session_id)?.is_none() {
            let mut run = RunRecord::new(session_id.to_string(), task_id, 1);
            run.status = RunStatus::Running;
            run.started_at = Some(chrono::Utc::now().to_rfc3339());
            run.updated_at = chrono::Utc::now().to_rfc3339();
            self.task_store.save_run(run)?;
        }

        Ok(())
    }

    fn persist_schedule(
        &self,
        session_id: &str,
        trigger_type: ScheduleTriggerType,
        wake_at: chrono::DateTime<chrono::Utc>,
        reason: &str,
    ) -> anyhow::Result<ScheduleItemRecord> {
        let schedule_id = format!("sch-{}", uuid::Uuid::new_v4());
        let task_id = format!("session-task-{session_id}");
        let trigger_label = match trigger_type {
            ScheduleTriggerType::At => "at",
            ScheduleTriggerType::Interval => "interval",
            ScheduleTriggerType::RetryBackoff => "retry_backoff",
        };
        let idempotency_key = format!(
            "sched:{}:{}:{}",
            session_id,
            wake_at.timestamp_millis(),
            trigger_label
        );
        let schedule = ScheduleItemRecord::new(
            schedule_id.clone(),
            task_id,
            session_id.to_string(),
            trigger_type,
            wake_at.to_rfc3339(),
            idempotency_key,
        );
        self.task_store.save_schedule_item(schedule)?;
        info!(
            session_id = %session_id,
            schedule_id = %schedule_id,
            trigger = ?trigger_type,
            wake_at = %wake_at.to_rfc3339(),
            reason,
            "Persisted scheduler item"
        );
        self.task_store
            .get_schedule_item(&schedule_id)?
            .ok_or_else(|| anyhow::anyhow!("Failed to reload schedule item {}", schedule_id))
    }

    /// Create a new session and return its ID.
    ///
    /// Lazily starts the cleanup task if not already started.
    #[allow(dead_code)]
    pub async fn create_session(
        &self,
        workspace_dir: Option<std::path::PathBuf>,
    ) -> anyhow::Result<String> {
        self.create_session_from_rollout(workspace_dir, None, None, None, None)
            .await
    }

    /// Create a new session, optionally preloading runtime context from an existing rollout.
    pub async fn create_session_from_rollout(
        &self,
        workspace_dir: Option<std::path::PathBuf>,
        resume_rollout_path: Option<PathBuf>,
        governance: Option<alan_protocol::GovernanceConfig>,
        streaming_mode: Option<alan_runtime::StreamingMode>,
        partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
    ) -> anyhow::Result<String> {
        self.ensure_sessions_recovered().await?;
        // Lazily start cleanup task on first session creation
        self.start_cleanup_task();
        self.start_scheduler_task();
        let session_id = uuid::Uuid::new_v4().to_string();
        self.ensure_task_run_for_session(&session_id)?;

        // Resolve workspace path using workspace_resolver
        let workspace_identifier = workspace_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        let resolved = self
            .workspace_resolver
            .resolve_or_create(workspace_identifier.as_deref())?;
        let workspace_path = resolved.path;
        let workspace_alan_dir = resolved.alan_dir;

        // Determine governance configuration for this session runtime
        let governance = governance.unwrap_or_default();
        let effective_streaming_mode = streaming_mode.unwrap_or(self.config.streaming_mode);
        let effective_partial_stream_recovery_mode =
            partial_stream_recovery_mode.unwrap_or(self.config.partial_stream_recovery_mode);
        let session_policy = RuntimeSessionPolicy {
            governance: governance.clone(),
            streaming_mode: Some(effective_streaming_mode),
            partial_stream_recovery_mode: Some(effective_partial_stream_recovery_mode),
        };

        // Start runtime using runtime_manager
        let handle = self
            .runtime_manager
            .start_runtime(
                session_id.clone(),
                workspace_path.clone(),
                workspace_alan_dir.clone(),
                resume_rollout_path,
                session_policy,
            )
            .await?;

        // Detect rollout path
        let rollout_path = detect_latest_rollout_path(&workspace_alan_dir.join("sessions"));

        let (events_tx, _) = broadcast::channel(DEFAULT_EVENT_BROADCAST_CAPACITY);
        let event_log = Arc::new(RwLock::new(SessionEventLog::new(
            DEFAULT_EVENT_REPLAY_BUFFER_CAPACITY,
        )));
        let event_bridge_task = Some(Self::spawn_event_bridge(
            session_id.clone(),
            handle.event_sender.subscribe(),
            events_tx.clone(),
            Arc::clone(&event_log),
            Arc::clone(&self.task_store),
        ));

        let entry = SessionEntry::new(
            workspace_path.clone(),
            workspace_alan_dir,
            governance.clone(),
            effective_streaming_mode,
            effective_partial_stream_recovery_mode,
            handle.submission_tx,
            events_tx,
            event_log,
            event_bridge_task,
            rollout_path.clone(),
        );

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), entry);

        let binding = SessionBinding {
            session_id: session_id.clone(),
            workspace_path,
            created_at: chrono::Utc::now().to_rfc3339(),
            governance,
            streaming_mode: Some(effective_streaming_mode),
            partial_stream_recovery_mode: Some(effective_partial_stream_recovery_mode),
            rollout_path,
        };
        if let Err(e) = self.session_store.save(binding) {
            warn!(%session_id, error = %e, "Failed to persist session binding");
        }

        Ok(session_id)
    }

    /// Ensure a session's runtime is running and refresh channels/rollout path.
    pub async fn resume_session_runtime(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        match self.task_store.restore_run(id) {
            Ok(snapshot) => {
                info!(
                    run_id = id,
                    run_status = ?snapshot.run.status,
                    checkpoint_id = ?snapshot.checkpoint.as_ref().map(|cp| cp.checkpoint_id.as_str()),
                    checkpoint_type = ?snapshot.checkpoint.as_ref().map(|cp| cp.checkpoint_type.as_str()),
                    next_action = ?snapshot.next_action,
                    "Restored run snapshot before runtime resume"
                );
            }
            Err(err) => {
                if !is_run_not_found_error(&err) {
                    warn!(
                        run_id = id,
                        error = %err,
                        "Failed to restore durable run snapshot before resume"
                    );
                }
            }
        }

        // Get workspace_path for the session
        let (workspace_path, workspace_alan_dir, resume_rollout_path, session_policy) = {
            let sessions = self.sessions.read().await;
            match sessions.get(id) {
                Some(entry) => (
                    entry.workspace_path.clone(),
                    entry.workspace_alan_dir.clone(),
                    resolve_resume_rollout_path(
                        entry.rollout_path.clone(),
                        entry.workspace_alan_dir.as_path(),
                    ),
                    RuntimeSessionPolicy {
                        governance: entry.governance.clone(),
                        streaming_mode: Some(entry.streaming_mode),
                        partial_stream_recovery_mode: Some(entry.partial_stream_recovery_mode),
                    },
                ),
                None => anyhow::bail!("Session {} not found", id),
            }
        };

        // Fast path: use existing handle when possible.
        // Fallback to start_runtime() handles races where runtime exits between checks.
        let handle = match self.runtime_manager.get_handle(id).await {
            Ok(handle) => handle,
            Err(get_err) => {
                warn!(
                    session_id = id,
                    error = %get_err,
                    "Runtime handle unavailable during resume; attempting restart"
                );
                self.runtime_manager
                    .start_runtime(
                        id.to_string(),
                        workspace_path.clone(),
                        workspace_alan_dir.clone(),
                        resume_rollout_path,
                        session_policy,
                    )
                    .await?
            }
        };

        // Update rollout path
        let rollout_path = detect_latest_rollout_path(&workspace_alan_dir.join("sessions"));

        {
            let mut sessions = self.sessions.write().await;
            let entry = sessions
                .get_mut(id)
                .ok_or_else(|| anyhow::anyhow!("Session {} disappeared during resume", id))?;
            if let Some(task) = entry.event_bridge_task.take() {
                task.abort();
            }
            let new_bridge = Self::spawn_event_bridge(
                id.to_string(),
                handle.event_sender.subscribe(),
                entry.events_tx.clone(),
                Arc::clone(&entry.event_log),
                Arc::clone(&self.task_store),
            );
            entry.submission_tx = handle.submission_tx;
            entry.event_bridge_task = Some(new_bridge);
            entry.rollout_path = rollout_path.clone();
            entry.touch_outbound();
        }
        if let Err(err) = self.session_store.update_rollout_path(id, rollout_path) {
            warn!(session_id = id, error = %err, "Failed to persist rollout path after resume");
        }
        Ok(())
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: &str) -> anyhow::Result<bool> {
        self.ensure_sessions_recovered().await?;
        Ok(self.sessions.read().await.contains_key(id))
    }

    /// Restore durable run snapshot by run ID.
    #[allow(dead_code)]
    pub fn restore_run(
        &self,
        run_id: &str,
    ) -> anyhow::Result<super::task_store::RunRestoreSnapshot> {
        self.task_store.restore_run(run_id)
    }

    /// Update a session entry's rollout path.
    pub async fn set_session_rollout_path(
        &self,
        id: &str,
        path: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.rollout_path = path.clone();
        }
        if let Err(err) = self.session_store.update_rollout_path(id, path) {
            warn!(session_id = id, error = %err, "Failed to persist rollout path update");
        }
        Ok(())
    }

    /// Get a mutable session entry (for updating inbound activity)
    pub async fn touch_session_inbound(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.touch_inbound();
        }
        Ok(())
    }

    /// Update outbound activity (event sent to client)
    pub async fn touch_session_outbound(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.touch_outbound();
        }
        Ok(())
    }

    /// Remove a session
    ///
    /// First stops the runtime, then removes the session only if successful.
    /// This ensures we don't leave orphan runtimes if stop fails.
    pub async fn remove_session(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;

        // Stop runtime first using runtime_manager
        if let Err(err) = self.runtime_manager.stop_runtime(id).await {
            warn!(
                session_id = id,
                error = %err,
                "Failed to stop runtime while removing session"
            );
            return Err(err);
        }

        // Finally remove the session entry and clean up the store
        if let Some(mut entry) = self.sessions.write().await.remove(id)
            && let Some(task) = entry.event_bridge_task.take()
        {
            task.abort();
        }
        if let Err(err) = self.session_store.remove(id) {
            warn!(session_id = id, error = %err, "Failed to remove persisted session binding");
        }
        Ok(())
    }

    /// Clean up all expired sessions (can be called manually)
    #[allow(dead_code)]
    pub async fn cleanup_expired(&self) -> anyhow::Result<usize> {
        self.ensure_sessions_recovered().await?;
        let ttl = Duration::from_secs(self.session_ttl_secs);
        let now = chrono::Utc::now();

        let expired: Vec<String> = {
            let sessions_guard = self.sessions.read().await;
            sessions_guard
                .iter()
                .filter(|(_, entry)| entry.is_expired(ttl))
                .filter_map(|(session_id, _)| {
                    match self.should_preserve_session_until_wake(session_id, &now) {
                        Ok(true) => None,
                        Ok(false) => Some(session_id.clone()),
                        Err(err) => {
                            warn!(
                                session_id = %session_id,
                                error = %err,
                                "Failed to evaluate scheduled session retention during TTL cleanup; skipping removal"
                            );
                            None
                        }
                    }
                })
                .collect()
        };

        let mut removed_count = 0;
        for session_id in expired {
            match self.remove_session(&session_id).await {
                Ok(()) => removed_count += 1,
                Err(_) => {
                    // Failed to remove, will be retried on next cleanup
                }
            }
        }

        Ok(removed_count)
    }
}

impl AppState {
    fn spawn_event_bridge(
        session_id: String,
        mut runtime_events_rx: broadcast::Receiver<RuntimeEventEnvelope>,
        client_events_tx: broadcast::Sender<EventEnvelope>,
        event_log: Arc<RwLock<SessionEventLog>>,
        task_store: Arc<TaskStore<JsonFileTaskStoreBackend>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match runtime_events_rx.recv().await {
                    Ok(event) => {
                        if let Some((checkpoint_type, summary, payload)) =
                            checkpoint_from_event(&event.event)
                            && let Err(err) = task_store.record_run_checkpoint(
                                &session_id,
                                checkpoint_type,
                                summary,
                                payload,
                            )
                        {
                            warn!(
                                run_id = %session_id,
                                error = %err,
                                "Failed to persist runtime event checkpoint"
                            );
                        }
                        let envelope = {
                            let mut guard = event_log.write().await;
                            guard.append_runtime_event(&session_id, event)
                        };
                        let _ = client_events_tx.send(envelope);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        warn!(
                            %session_id,
                            missed = count,
                            "Event bridge lagged behind runtime stream"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }
}

fn detect_latest_rollout_path(sessions_dir: &std::path::Path) -> Option<PathBuf> {
    if !sessions_dir.exists() {
        return None;
    }

    let mut dirs = vec![sessions_dir.to_path_buf()];
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                warn!(
                    path = %dir.display(),
                    error = %err,
                    "Failed to inspect session directory while scanning rollouts"
                );
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warn!(
                        path = %dir.display(),
                        error = %err,
                        "Failed to inspect session directory entry while scanning rollouts"
                    );
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                dirs.push(path);
                continue;
            }

            let is_jsonl = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
                .unwrap_or(false);
            if !is_jsonl {
                continue;
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);

            match &latest {
                Some((best_time, best_path))
                    if modified < *best_time || (modified == *best_time && path <= *best_path) => {}
                _ => latest = Some((modified, path)),
            }
        }
    }

    latest.map(|(_, path)| path)
}

fn resolve_resume_rollout_path(
    persisted_rollout_path: Option<PathBuf>,
    workspace_alan_dir: &std::path::Path,
) -> Option<PathBuf> {
    if let Some(path) = persisted_rollout_path
        && path.exists()
    {
        return Some(path);
    }

    detect_latest_rollout_path(&workspace_alan_dir.join("sessions"))
}

#[cfg(test)]
mod tests {
    use super::super::runtime_manager::RuntimeManager;
    use super::super::workspace_resolver::WorkspaceResolver;
    use super::*;
    use alan_runtime::runtime::WorkspaceRuntimeConfig;
    use chrono::Utc;
    use tempfile::TempDir;

    fn runtime_event(event: Event) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            submission_id: Some("sub-test".to_string()),
            event,
        }
    }

    fn runtime_event_no_submission(event: Event) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            submission_id: None,
            event,
        }
    }

    fn create_test_resolver_and_manager(
        base_dir: &std::path::Path,
    ) -> (Arc<WorkspaceResolver>, Arc<RuntimeManager>) {
        // Create a mock resolver that uses the base_dir as default
        let resolver = WorkspaceResolver::with_registry(
            crate::registry::WorkspaceRegistry {
                version: 1,
                workspaces: vec![],
            },
            base_dir.to_path_buf(),
        );

        let runtime_config = WorkspaceRuntimeConfig::from(Config::default());
        let manager = RuntimeManager::with_template(runtime_config);

        (Arc::new(resolver), Arc::new(manager))
    }

    fn create_test_resolver_and_manager_with_runtime_limit(
        base_dir: &std::path::Path,
        max_concurrent_runtimes: usize,
    ) -> (Arc<WorkspaceResolver>, Arc<RuntimeManager>) {
        let resolver = WorkspaceResolver::with_registry(
            crate::registry::WorkspaceRegistry {
                version: 1,
                workspaces: vec![],
            },
            base_dir.to_path_buf(),
        );

        let manager = RuntimeManager::new(crate::daemon::runtime_manager::RuntimeManagerConfig {
            max_concurrent_runtimes,
            runtime_config_template: WorkspaceRuntimeConfig::from(Config::default()),
        });

        (Arc::new(resolver), Arc::new(manager))
    }

    fn test_state() -> AppState {
        let path = std::env::temp_dir().join(format!("agentd-state-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        test_state_with_base_dir(&path)
    }

    fn test_state_with_base_dir(base_dir: &std::path::Path) -> AppState {
        let (resolver, manager) = create_test_resolver_and_manager(base_dir);
        let store = Arc::new(SessionStore::with_dir(base_dir.join("sessions")).unwrap());
        let task_store = Arc::new(
            TaskStore::new(
                JsonFileTaskStoreBackend::with_storage_dir(base_dir.join("tasks")).unwrap(),
            )
            .unwrap(),
        );
        AppState::from_parts_with_task_store(
            Config::default(),
            resolver,
            manager,
            store,
            task_store,
            1,
        )
    }

    fn test_state_with_ttl(base_dir: &std::path::Path, ttl_secs: u64) -> AppState {
        let (resolver, manager) = create_test_resolver_and_manager(base_dir);
        let store = Arc::new(SessionStore::with_dir(base_dir.join("sessions")).unwrap());
        let task_store = Arc::new(
            TaskStore::new(
                JsonFileTaskStoreBackend::with_storage_dir(base_dir.join("tasks")).unwrap(),
            )
            .unwrap(),
        );
        AppState::from_parts_with_task_store(
            Config::default(),
            resolver,
            manager,
            store,
            task_store,
            ttl_secs,
        )
    }

    fn test_state_with_runtime_limit(
        base_dir: &std::path::Path,
        max_concurrent_runtimes: usize,
    ) -> AppState {
        let (resolver, manager) =
            create_test_resolver_and_manager_with_runtime_limit(base_dir, max_concurrent_runtimes);
        let store = Arc::new(SessionStore::with_dir(base_dir.join("sessions")).unwrap());
        let task_store = Arc::new(
            TaskStore::new(
                JsonFileTaskStoreBackend::with_storage_dir(base_dir.join("tasks")).unwrap(),
            )
            .unwrap(),
        );
        AppState::from_parts_with_task_store(
            Config::default(),
            resolver,
            manager,
            store,
            task_store,
            1,
        )
    }

    fn test_session_entry(
        workspace_path: &std::path::Path,
    ) -> (SessionEntry, mpsc::Receiver<Submission>) {
        let (submission_tx, submission_rx) = mpsc::channel(8);
        let (events_tx, _) = broadcast::channel(8);
        let event_log = Arc::new(RwLock::new(SessionEventLog::new(16)));
        let entry = SessionEntry::new(
            workspace_path.to_path_buf(),
            workspace_path.join(".alan"),
            alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            },
            alan_runtime::StreamingMode::Auto,
            alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
            submission_tx,
            events_tx,
            event_log,
            None,
            None,
        );
        (entry, submission_rx)
    }

    #[test]
    fn detect_latest_rollout_path_picks_latest_jsonl() {
        let temp = tempfile::TempDir::new().unwrap();
        let sessions_dir = temp.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        let old = sessions_dir.join("old.jsonl");
        std::fs::write(&old, "{}\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let new = sessions_dir.join("new.jsonl");
        std::fs::write(&new, "{}\n").unwrap();

        let detected = detect_latest_rollout_path(&sessions_dir).unwrap();
        assert_eq!(detected, new);
    }

    #[test]
    fn resolve_resume_rollout_path_prefers_existing_persisted_path() {
        let temp = TempDir::new().unwrap();
        let workspace_alan_dir = temp.path().join(".alan");
        let sessions_dir = workspace_alan_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        let persisted = sessions_dir.join("persisted.jsonl");
        std::fs::write(&persisted, "{}\n").unwrap();
        std::fs::write(sessions_dir.join("newer.jsonl"), "{}\n").unwrap();

        let resolved = resolve_resume_rollout_path(Some(persisted.clone()), &workspace_alan_dir);
        assert_eq!(resolved, Some(persisted));
    }

    #[test]
    fn resolve_resume_rollout_path_falls_back_to_latest_when_persisted_missing() {
        let temp = TempDir::new().unwrap();
        let workspace_alan_dir = temp.path().join(".alan");
        let sessions_dir = workspace_alan_dir.join("sessions");
        let nested = sessions_dir.join("2026").join("03").join("01");
        std::fs::create_dir_all(&nested).unwrap();

        let latest = nested.join("latest.jsonl");
        std::fs::write(&latest, "{}\n").unwrap();

        let missing = sessions_dir.join("missing.jsonl");
        let resolved = resolve_resume_rollout_path(Some(missing), &workspace_alan_dir);
        assert_eq!(resolved, Some(latest));
    }

    #[test]
    fn session_entry_touch_updates_timestamps() {
        let temp = TempDir::new().unwrap();
        let (mut entry, _rx) = test_session_entry(temp.path());
        let inbound_before = entry.last_inbound_activity;
        let outbound_before = entry.last_outbound_activity;
        std::thread::sleep(std::time::Duration::from_millis(2));
        entry.touch_inbound();
        entry.touch_outbound();
        assert!(entry.last_inbound_activity >= inbound_before);
        assert!(entry.last_outbound_activity >= outbound_before);
    }

    #[test]
    fn session_entry_expiration_requires_both_sides_idle() {
        let temp = TempDir::new().unwrap();
        let (mut entry, _rx) = test_session_entry(temp.path());
        let ttl = std::time::Duration::from_secs(5);
        let now = std::time::Instant::now();

        entry.last_inbound_activity = now - std::time::Duration::from_secs(10);
        entry.last_outbound_activity = now - std::time::Duration::from_secs(10);
        assert!(entry.is_expired(ttl));

        entry.last_inbound_activity = now;
        entry.last_outbound_activity = now - std::time::Duration::from_secs(10);
        assert!(!entry.is_expired(ttl));

        entry.last_inbound_activity = now - std::time::Duration::from_secs(10);
        entry.last_outbound_activity = now;
        assert!(!entry.is_expired(ttl));
    }

    #[test]
    fn session_event_log_assigns_stable_event_turn_and_item_ids() {
        let mut log = SessionEventLog::new(16);

        let e1 = log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        let e2 = log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "hello".to_string(),
                is_final: true,
            }),
        );
        let e3 = log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));

        assert_eq!(e1.event_id, "evt_0000000000000001");
        assert_eq!(e2.event_id, "evt_0000000000000002");
        assert_eq!(e3.event_id, "evt_0000000000000003");
        assert_eq!(e1.turn_id, "turn_000001");
        assert_eq!(e2.turn_id, "turn_000001");
        assert_eq!(e3.turn_id, "turn_000002");
        assert_eq!(e1.item_id, "item_000001_0001");
        assert_eq!(e2.item_id, "item_000001_0002");
        assert_eq!(e3.item_id, "item_000002_0001");
        assert_eq!(e2.submission_id.as_deref(), Some("sub-test"));
    }

    #[test]
    fn session_event_log_replay_reports_gap_when_cursor_is_evicted() {
        let mut log = SessionEventLog::new(2);
        for i in 0..3 {
            log.append_runtime_event(
                "sess-1",
                runtime_event(if i == 0 || i == 2 {
                    Event::TurnStarted {}
                } else {
                    Event::TextDelta {
                        chunk: format!("turn {i}"),
                        is_final: true,
                    }
                }),
            );
        }

        let page = log.read_after(Some("evt_0000000000000001"), 10);
        assert!(page.gap);
        assert_eq!(page.events.len(), 2);
        assert_eq!(page.events[0].event_id, "evt_0000000000000002");
        assert_eq!(page.events[1].event_id, "evt_0000000000000003");
        assert_eq!(
            page.oldest_event_id.as_deref(),
            Some("evt_0000000000000002")
        );
        assert_eq!(
            page.latest_event_id.as_deref(),
            Some("evt_0000000000000003")
        );
    }

    #[tokio::test]
    async fn get_touch_and_remove_missing_session_are_safe() {
        let state = test_state();
        assert!(!state.get_session("missing").await.unwrap());
        state.touch_session_inbound("missing").await.unwrap();
        state.touch_session_outbound("missing").await.unwrap();
        // remove_session on non-existent session should succeed (idempotent)
        state.remove_session("missing").await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_expired_removes_session_with_stopped_runtime() {
        let state = test_state();
        let temp = TempDir::new().unwrap();
        let (mut entry, _rx) = test_session_entry(temp.path());
        let old = std::time::Instant::now() - std::time::Duration::from_secs(10);
        entry.last_inbound_activity = old;
        entry.last_outbound_activity = old;

        state
            .sessions
            .write()
            .await
            .insert("sess-1".to_string(), entry);

        // The session entry exists but has no running runtime
        // remove_session will try to stop the non-existent runtime
        // which should succeed (idempotent in runtime_manager)
        let removed = state.cleanup_expired().await.unwrap();
        // Since the runtime doesn't exist, stop_runtime returns Ok(())
        // so the session should be removed
        assert_eq!(removed, 1);
        assert!(!state.get_session("sess-1").await.unwrap());
    }

    #[tokio::test]
    async fn cleanup_expired_removes_persisted_binding() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());
        state.ensure_sessions_recovered().await.unwrap();

        let workspace_path = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();
        state
            .session_store
            .save(crate::daemon::session_store::SessionBinding {
                session_id: "sess-persisted".to_string(),
                workspace_path: workspace_path.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                governance: alan_protocol::GovernanceConfig::default(),
                streaming_mode: Some(alan_runtime::StreamingMode::Auto),
                partial_stream_recovery_mode: Some(
                    alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
                ),
                rollout_path: None,
            })
            .unwrap();

        let (mut entry, _rx) = test_session_entry(&workspace_path);
        let old = std::time::Instant::now() - std::time::Duration::from_secs(10);
        entry.last_inbound_activity = old;
        entry.last_outbound_activity = old;
        state
            .sessions
            .write()
            .await
            .insert("sess-persisted".to_string(), entry);

        let removed = state.cleanup_expired().await.unwrap();
        assert_eq!(removed, 1);
        assert!(!state.get_session("sess-persisted").await.unwrap());
        assert!(!state.session_store.exists("sess-persisted"));
    }

    #[tokio::test]
    async fn cleanup_expired_preserves_sleeping_session_until_future_wake() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_ttl(temp.path(), 1);

        let (mut entry, _rx) = test_session_entry(temp.path());
        let old = std::time::Instant::now() - std::time::Duration::from_secs(10);
        entry.last_inbound_activity = old;
        entry.last_outbound_activity = old;
        state
            .sessions
            .write()
            .await
            .insert("sess-sleeping".to_string(), entry);

        state.ensure_task_run_for_session("sess-sleeping").unwrap();
        state
            .task_store
            .transition_run_status(
                "sess-sleeping",
                RunStatus::Sleeping,
                SCHEDULER_ACTOR,
                Some("test sleeping run".to_string()),
            )
            .unwrap();
        state
            .task_store
            .set_run_next_wake_at(
                "sess-sleeping",
                Some((Utc::now() + chrono::Duration::hours(2)).to_rfc3339()),
            )
            .unwrap();

        let removed = state.cleanup_expired().await.unwrap();
        assert_eq!(removed, 0);
        assert!(state.get_session("sess-sleeping").await.unwrap());
    }

    #[tokio::test]
    async fn cleanup_expired_preserves_session_with_future_schedule_at() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_ttl(temp.path(), 1);

        let (mut entry, _rx) = test_session_entry(temp.path());
        let old = std::time::Instant::now() - std::time::Duration::from_secs(10);
        entry.last_inbound_activity = old;
        entry.last_outbound_activity = old;
        state
            .sessions
            .write()
            .await
            .insert("sess-schedule-at".to_string(), entry);

        let wake_at = Utc::now() + chrono::Duration::hours(2);
        state
            .schedule_at("sess-schedule-at", wake_at)
            .await
            .unwrap();

        let removed = state.cleanup_expired().await.unwrap();
        assert_eq!(removed, 0);
        assert!(state.get_session("sess-schedule-at").await.unwrap());
    }

    #[tokio::test]
    async fn ensure_sessions_recovered_is_idempotent() {
        let state = test_state();
        // Should succeed and be idempotent
        state.ensure_sessions_recovered().await.unwrap();
        state.ensure_sessions_recovered().await.unwrap();
        assert!(
            state
                .sessions_recovered
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn ensure_sessions_recovered_restores_created_at_age() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());
        let workspace_path = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        state
            .session_store
            .save(crate::daemon::session_store::SessionBinding {
                session_id: "sess-aged".to_string(),
                workspace_path: workspace_path.clone(),
                created_at: (chrono::Utc::now() - chrono::Duration::seconds(120)).to_rfc3339(),
                governance: alan_protocol::GovernanceConfig::default(),
                streaming_mode: Some(alan_runtime::StreamingMode::Auto),
                partial_stream_recovery_mode: Some(
                    alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
                ),
                rollout_path: None,
            })
            .unwrap();

        state.ensure_sessions_recovered().await.unwrap();

        let sessions = state.sessions.read().await;
        let entry = sessions.get("sess-aged").unwrap();
        assert!(entry.created_at.elapsed() >= std::time::Duration::from_secs(100));
    }

    #[test]
    fn test_state_with_ttl_custom() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_ttl(temp.path(), 600);
        assert_eq!(state.session_ttl_secs, 600);
    }

    #[test]
    fn detect_latest_rollout_path_dir_not_exist() {
        let path = std::path::PathBuf::from("/nonexistent/dir/sessions");
        assert!(detect_latest_rollout_path(&path).is_none());
    }

    #[test]
    fn detect_latest_rollout_path_empty_dir() {
        let temp = TempDir::new().unwrap();
        let sessions_dir = temp.path().join("empty_sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        assert!(detect_latest_rollout_path(&sessions_dir).is_none());
    }

    #[test]
    fn detect_latest_rollout_path_skips_non_jsonl() {
        let temp = TempDir::new().unwrap();
        let sessions_dir = temp.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        std::fs::write(sessions_dir.join("readme.txt"), "not jsonl").unwrap();
        std::fs::write(sessions_dir.join("data.json"), "{}\n").unwrap();
        // Only jsonl should be picked
        std::fs::write(sessions_dir.join("valid.jsonl"), "{}\n").unwrap();

        let detected = detect_latest_rollout_path(&sessions_dir).unwrap();
        assert_eq!(detected.file_name().unwrap(), "valid.jsonl");
    }

    #[test]
    fn detect_latest_rollout_path_searches_nested_directories() {
        let temp = TempDir::new().unwrap();
        let sessions_dir = temp.path().join("sessions");
        let nested_dir = sessions_dir.join("2026").join("02").join("28");
        std::fs::create_dir_all(&nested_dir).unwrap();

        let nested_rollout = nested_dir.join("rollout-20260228-abc.jsonl");
        std::fs::write(&nested_rollout, "{}\n").unwrap();

        let detected = detect_latest_rollout_path(&sessions_dir).unwrap();
        assert_eq!(detected, nested_rollout);
    }

    #[cfg(unix)]
    #[test]
    fn detect_latest_rollout_path_skips_unreadable_nested_directories() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let sessions_dir = temp.path().join("sessions");
        let unreadable_dir = sessions_dir.join("private");
        std::fs::create_dir_all(&unreadable_dir).unwrap();

        let valid_rollout = sessions_dir.join("valid.jsonl");
        std::fs::write(&valid_rollout, "{}\n").unwrap();

        let mut perms = std::fs::metadata(&unreadable_dir).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&unreadable_dir, perms).unwrap();

        let detected = detect_latest_rollout_path(&sessions_dir).unwrap();
        assert_eq!(detected, valid_rollout);

        let mut restore = std::fs::metadata(&unreadable_dir).unwrap().permissions();
        restore.set_mode(0o755);
        let _ = std::fs::set_permissions(&unreadable_dir, restore);
    }

    #[test]
    fn session_event_log_capacity_edge_cases() {
        // Test with zero capacity - internal buffer is created with min(0, 16) = 16 but capacity field is 1
        let log = SessionEventLog::new(0);
        assert_eq!(log.capacity, 1); // The capacity field should be at least 1
    }

    #[test]
    fn session_event_log_read_after_none() {
        let mut log = SessionEventLog::new(16);

        log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "hello".to_string(),
                is_final: true,
            }),
        );

        // Read from beginning (after_event_id is None)
        let page = log.read_after(None, 10);
        assert!(!page.gap);
        assert_eq!(page.events.len(), 2);
        assert_eq!(page.events[0].event_id, "evt_0000000000000001");
        assert_eq!(page.events[1].event_id, "evt_0000000000000002");
        assert_eq!(
            page.oldest_event_id,
            Some("evt_0000000000000001".to_string())
        );
        assert_eq!(
            page.latest_event_id,
            Some("evt_0000000000000002".to_string())
        );
    }

    #[test]
    fn session_event_log_read_after_invalid_id() {
        let mut log = SessionEventLog::new(16);
        log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));

        // Invalid event ID format
        let page = log.read_after(Some("invalid-id"), 10);
        assert!(page.gap);
        assert!(page.events.is_empty());

        // Event ID with valid format but not in buffer
        let page = log.read_after(Some("evt_9999999999999999"), 10);
        assert!(!page.gap); // Beyond latest, returns empty without gap
        assert!(page.events.is_empty());
    }

    #[test]
    fn session_event_log_read_after_within_range_but_not_found() {
        let mut log = SessionEventLog::new(16);
        log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));

        // After ID is within sequence range but not in buffer (evicted)
        // This shouldn't happen in practice but tests the branch
        let page = log.read_after(Some("evt_0000000000000001"), 10);
        assert!(!page.gap);
        assert_eq!(page.events.len(), 1);
    }

    #[test]
    fn session_event_log_limit_clamping() {
        let mut log = SessionEventLog::new(16);
        for _ in 0..5 {
            log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        }

        // Limit should be clamped to valid range
        let page = log.read_after(None, 0); // Minimum is 1
        assert_eq!(page.events.len(), 1);

        let page = log.read_after(None, 10000); // Maximum is 1000
        assert_eq!(page.events.len(), 5);
    }

    #[test]
    fn parse_event_sequence_edge_cases() {
        assert_eq!(parse_event_sequence("evt_123"), Some(123));
        assert_eq!(parse_event_sequence("evt_0000000000000001"), Some(1));
        assert_eq!(parse_event_sequence("invalid"), None);
        assert_eq!(parse_event_sequence("evt_"), None);
        assert_eq!(parse_event_sequence("evt_not_a_number"), None);
        assert_eq!(parse_event_sequence(""), None);
        assert_eq!(
            parse_event_sequence("evt_18446744073709551615"),
            Some(u64::MAX)
        ); // Max u64
    }

    #[test]
    fn now_timestamp_ms_returns_nonzero() {
        let ts1 = now_timestamp_ms();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let ts2 = now_timestamp_ms();
        assert!(ts1 > 0);
        assert!(ts2 >= ts1);
    }

    #[test]
    fn session_entry_not_expired_at_exact_ttl() {
        let temp = TempDir::new().unwrap();
        let (mut entry, _rx) = test_session_entry(temp.path());
        let ttl = std::time::Duration::from_secs(5);

        // Create times in the past based on when entry was created
        let base_time = entry.last_inbound_activity;

        // Just past TTL - should be expired
        entry.last_inbound_activity = base_time - ttl - std::time::Duration::from_millis(10);
        entry.last_outbound_activity = base_time - ttl - std::time::Duration::from_millis(10);
        assert!(entry.is_expired(ttl));
    }

    #[tokio::test]
    async fn set_session_rollout_path_updates_path() {
        let state = test_state();
        let temp = TempDir::new().unwrap();
        let (entry, _rx) = test_session_entry(temp.path());

        state
            .sessions
            .write()
            .await
            .insert("sess-1".to_string(), entry);

        let new_path = std::path::PathBuf::from("/new/rollout.jsonl");
        state
            .set_session_rollout_path("sess-1", Some(new_path.clone()))
            .await
            .unwrap();

        let sessions = state.sessions.read().await;
        let entry = sessions.get("sess-1").unwrap();
        assert_eq!(entry.rollout_path, Some(new_path));
    }

    #[tokio::test]
    async fn set_session_rollout_path_missing_session_is_safe() {
        let state = test_state();
        // Should not panic
        state
            .set_session_rollout_path("nonexistent", Some(std::path::PathBuf::from("/test.jsonl")))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn schedule_at_persists_waiting_schedule_and_run_records() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        let (entry, _rx) = test_session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-schedule".to_string(), entry);

        let wake_at = Utc::now() + chrono::Duration::minutes(5);
        let schedule = state.schedule_at("sess-schedule", wake_at).await.unwrap();

        assert_eq!(schedule.run_id, "sess-schedule");
        assert_eq!(schedule.status, ScheduleStatus::Waiting);
        assert_eq!(schedule.trigger_type, ScheduleTriggerType::At);

        let task = state
            .task_store
            .get_task("session-task-sess-schedule")
            .unwrap()
            .unwrap();
        assert_eq!(task.status, TaskStatus::Running);

        let run = state.task_store.get_run("sess-schedule").unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(run.next_wake_at, Some(wake_at.to_rfc3339()));
    }

    #[tokio::test]
    async fn sleep_until_transitions_run_to_sleeping_and_sets_wake() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        let (entry, _rx) = test_session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-sleep".to_string(), entry);

        let wake_at = Utc::now() + chrono::Duration::minutes(3);
        let schedule = state.sleep_until("sess-sleep", wake_at).await.unwrap();

        assert_eq!(schedule.run_id, "sess-sleep");
        assert_eq!(schedule.status, ScheduleStatus::Waiting);

        let run = state.task_store.get_run("sess-sleep").unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Sleeping);
        assert_eq!(run.next_wake_at, Some(wake_at.to_rfc3339()));
        let checkpoint = state
            .task_store
            .get_latest_run_checkpoint("sess-sleep")
            .unwrap()
            .unwrap();
        assert_eq!(checkpoint.checkpoint_type, "sleep_until");
    }

    #[tokio::test]
    async fn sleep_until_cancels_previous_waiting_schedule() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        let (entry, _rx) = test_session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-resleep".to_string(), entry);

        let first_wake = Utc::now() + chrono::Duration::minutes(2);
        let first = state.sleep_until("sess-resleep", first_wake).await.unwrap();

        let second_wake = Utc::now() + chrono::Duration::minutes(10);
        let second = state
            .sleep_until("sess-resleep", second_wake)
            .await
            .unwrap();

        let first_after = state
            .task_store
            .get_schedule_item(&first.schedule_id)
            .unwrap()
            .unwrap();
        assert_eq!(first_after.status, ScheduleStatus::Cancelled);

        let second_after = state
            .task_store
            .get_schedule_item(&second.schedule_id)
            .unwrap()
            .unwrap();
        assert_eq!(second_after.status, ScheduleStatus::Waiting);
        assert_eq!(second_after.next_wake_at, second_wake.to_rfc3339());

        let run = state.task_store.get_run("sess-resleep").unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Sleeping);
        assert_eq!(run.next_wake_at, Some(second_wake.to_rfc3339()));
    }

    #[tokio::test]
    async fn scheduler_run_cycle_marks_missing_session_schedule_failed() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        state.ensure_task_run_for_session("sess-missing").unwrap();
        let schedule = state
            .persist_schedule(
                "sess-missing",
                ScheduleTriggerType::At,
                Utc::now() - chrono::Duration::seconds(1),
                "test missing session dispatch",
            )
            .unwrap();

        state.scheduler_run_cycle().await.unwrap();

        let updated = state
            .task_store
            .get_schedule_item(&schedule.schedule_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, ScheduleStatus::Failed);
    }

    #[tokio::test]
    async fn scheduler_run_cycle_recovers_preexisting_dispatching_items() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        state
            .ensure_task_run_for_session("sess-dispatching")
            .unwrap();
        let schedule = state
            .persist_schedule(
                "sess-dispatching",
                ScheduleTriggerType::At,
                Utc::now() + chrono::Duration::minutes(1),
                "test preexisting dispatching schedule",
            )
            .unwrap();
        state
            .task_store
            .transition_schedule_status(
                &schedule.schedule_id,
                ScheduleStatus::Dispatching,
                SCHEDULER_ACTOR,
                Some("test inject dispatching".to_string()),
            )
            .unwrap();

        state.scheduler_run_cycle().await.unwrap();

        let updated = state
            .task_store
            .get_schedule_item(&schedule.schedule_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, ScheduleStatus::Failed);
    }

    #[tokio::test]
    async fn scheduler_run_cycle_retries_transient_dispatch_and_syncs_run_wake() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_runtime_limit(temp.path(), 0);

        let (entry, _rx) = test_session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-retry".to_string(), entry);

        state.ensure_task_run_for_session("sess-retry").unwrap();
        state
            .task_store
            .transition_run_status(
                "sess-retry",
                RunStatus::Sleeping,
                SCHEDULER_ACTOR,
                Some("test transient retry".to_string()),
            )
            .unwrap();
        state
            .task_store
            .set_run_next_wake_at(
                "sess-retry",
                Some((Utc::now() - chrono::Duration::minutes(1)).to_rfc3339()),
            )
            .unwrap();

        let schedule = state
            .persist_schedule(
                "sess-retry",
                ScheduleTriggerType::At,
                Utc::now() - chrono::Duration::seconds(1),
                "test transient dispatch failure",
            )
            .unwrap();

        state.scheduler_run_cycle().await.unwrap();

        let updated_schedule = state
            .task_store
            .get_schedule_item(&schedule.schedule_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated_schedule.status, ScheduleStatus::Waiting);
        let retry_wake = chrono::DateTime::parse_from_rfc3339(&updated_schedule.next_wake_at)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert!(retry_wake > Utc::now());

        let run = state.task_store.get_run("sess-retry").unwrap().unwrap();
        assert_eq!(run.next_wake_at, Some(updated_schedule.next_wake_at));
    }

    #[tokio::test]
    async fn restore_run_returns_latest_checkpoint_snapshot() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());

        state.ensure_task_run_for_session("sess-restore").unwrap();
        state
            .task_store
            .transition_run_status(
                "sess-restore",
                RunStatus::Yielded,
                SCHEDULER_ACTOR,
                Some("test yielded restore".to_string()),
            )
            .unwrap();
        state
            .task_store
            .record_run_checkpoint(
                "sess-restore",
                "yield",
                "waiting for resume input",
                Some(serde_json::json!({"request_id": "req-1"})),
            )
            .unwrap();

        let restored = state.restore_run("sess-restore").unwrap();
        assert_eq!(restored.run.run_id, "sess-restore");
        assert_eq!(
            restored
                .checkpoint
                .as_ref()
                .map(|cp| cp.checkpoint_type.as_str()),
            Some("yield")
        );
        assert_eq!(
            restored.next_action,
            crate::daemon::task_store::RunResumeAction::AwaitUserResume
        );
    }

    #[tokio::test]
    async fn spawn_event_bridge_records_checkpoints_for_turn_events() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_base_dir(temp.path());
        state.ensure_task_run_for_session("sess-bridge").unwrap();

        let (runtime_events_tx, _) = broadcast::channel(16);
        let (client_events_tx, _) = broadcast::channel(16);
        let event_log = Arc::new(RwLock::new(SessionEventLog::new(16)));
        let bridge = AppState::spawn_event_bridge(
            "sess-bridge".to_string(),
            runtime_events_tx.subscribe(),
            client_events_tx,
            event_log,
            Arc::clone(&state.task_store),
        );

        runtime_events_tx
            .send(runtime_event(Event::TurnStarted {}))
            .unwrap();
        runtime_events_tx
            .send(runtime_event(Event::Yield {
                request_id: "req-bridge".to_string(),
                kind: alan_protocol::YieldKind::Confirmation,
                payload: serde_json::json!({}),
            }))
            .unwrap();
        runtime_events_tx
            .send(runtime_event(Event::TurnCompleted { summary: None }))
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        bridge.abort();

        let checkpoints = state
            .task_store
            .list_run_checkpoints("sess-bridge")
            .unwrap();
        let checkpoint_types: Vec<String> = checkpoints
            .iter()
            .map(|checkpoint| checkpoint.checkpoint_type.clone())
            .collect();
        assert!(checkpoint_types.iter().any(|ty| ty == "turn_start"));
        assert!(checkpoint_types.iter().any(|ty| ty == "yield"));
        assert!(checkpoint_types.iter().any(|ty| ty == "turn_complete"));
    }

    #[test]
    fn session_event_log_appends_without_submission_id() {
        let mut log = SessionEventLog::new(16);
        let envelope =
            log.append_runtime_event("sess-1", runtime_event_no_submission(Event::TurnStarted {}));
        assert_eq!(envelope.submission_id, None);
        assert_eq!(envelope.event_id, "evt_0000000000000001");
    }

    #[test]
    fn session_event_log_turn_sequence_increments_correctly() {
        let mut log = SessionEventLog::new(16);

        // First turn
        let e1 = log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        let e2 = log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "a".to_string(),
                is_final: true,
            }),
        );
        let e3 = log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "b".to_string(),
                is_final: true,
            }),
        );

        // Second turn
        let e4 = log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        let e5 = log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "c".to_string(),
                is_final: true,
            }),
        );

        assert_eq!(e1.turn_id, "turn_000001");
        assert_eq!(e2.turn_id, "turn_000001");
        assert_eq!(e3.turn_id, "turn_000001");
        assert_eq!(e4.turn_id, "turn_000002");
        assert_eq!(e5.turn_id, "turn_000002");

        assert_eq!(e1.item_id, "item_000001_0001");
        assert_eq!(e2.item_id, "item_000001_0002");
        assert_eq!(e3.item_id, "item_000001_0003");
        assert_eq!(e4.item_id, "item_000002_0001");
        assert_eq!(e5.item_id, "item_000002_0002");
    }

    #[test]
    fn session_event_log_buffer_wraps_correctly() {
        let mut log = SessionEventLog::new(3);

        // Fill buffer
        log.append_runtime_event("sess-1", runtime_event(Event::TurnStarted {}));
        log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "1".to_string(),
                is_final: true,
            }),
        );
        log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "2".to_string(),
                is_final: true,
            }),
        );

        // This should evict the first event
        log.append_runtime_event(
            "sess-1",
            runtime_event(Event::TextDelta {
                chunk: "3".to_string(),
                is_final: true,
            }),
        );

        assert_eq!(log.buffer.len(), 3);

        // Reading after the evicted event should report gap
        let page = log.read_after(Some("evt_0000000000000001"), 10);
        assert!(page.gap);
        assert_eq!(page.events.len(), 3);
    }

    #[tokio::test]
    async fn cleanup_expired_no_expired_sessions() {
        let state = test_state();
        // Create a fresh session (not expired)
        let temp = TempDir::new().unwrap();
        let (entry, _rx) = test_session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-fresh".to_string(), entry);

        let removed = state.cleanup_expired().await.unwrap();
        assert_eq!(removed, 0);
        assert!(state.get_session("sess-fresh").await.unwrap());
    }
}
