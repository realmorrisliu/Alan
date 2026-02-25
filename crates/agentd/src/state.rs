//! Application state management for agentd.

use crate::manager::{ManagerConfig, WorkspaceManager};
use alan_protocol::{Event, EventEnvelope, Submission};
use alan_runtime::{
    Config,
    runtime::{RuntimeEventEnvelope, WorkspaceRuntimeConfig},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
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
    pub config: Config,
    /// Agent manager
    pub workspace_manager: Arc<WorkspaceManager>,
    /// Active sessions
    pub sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    /// Session TTL in seconds
    pub session_ttl_secs: u64,
    /// Cleanup task started flag
    cleanup_started: Arc<std::sync::atomic::AtomicBool>,
    /// Whether on-disk session bindings have been recovered into memory
    sessions_recovered: Arc<std::sync::atomic::AtomicBool>,
    /// Serializes one-time recovery
    recovery_lock: Arc<Mutex<()>>,
}

/// Entry for an active session
pub struct SessionEntry {
    /// Backing agent instance ID
    pub workspace_id: String,
    /// Tool approval policy for this session runtime
    pub approval_policy: alan_protocol::ApprovalPolicy,
    /// Coarse sandbox mode for this session runtime
    pub sandbox_mode: alan_protocol::SandboxMode,
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

impl SessionEntry {
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
    /// Recover persisted session bindings (`current_session_id`) from agent state files.
    pub async fn ensure_sessions_recovered(&self) -> anyhow::Result<()> {
        if self
            .sessions_recovered
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let _guard = self.recovery_lock.lock().await;
        if self
            .sessions_recovered
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let agents = self.workspace_manager.list().await;
        let mut recovered = 0usize;

        for agent in agents {
            let instance = match self.workspace_manager.get(&agent.id).await {
                Ok(instance) => instance,
                Err(err) => {
                    warn!(agent_id = %agent.id, error = %err, "Failed to load agent during session recovery");
                    continue;
                }
            };

            let (session_id, approval_policy, sandbox_mode, rollout_path) = {
                let instance_guard = instance.read().await;
                let state_guard = instance_guard.state.read().await;
                let session_id = state_guard.current_session_id.clone();
                let approval_policy = state_guard.config.approval_policy.unwrap_or_default();
                let sandbox_mode = state_guard.config.sandbox_mode.unwrap_or_default();
                let rollout_path =
                    detect_latest_rollout_path(&instance_guard.workspace_dir.join("sessions"));
                (session_id, approval_policy, sandbox_mode, rollout_path)
            };

            let Some(session_id) = session_id else {
                continue;
            };

            let mut sessions = self.sessions.write().await;
            if sessions.contains_key(&session_id) {
                continue;
            }

            let (submission_tx, _submission_rx) = mpsc::channel(1);
            let (events_tx, _) = broadcast::channel(DEFAULT_EVENT_BROADCAST_CAPACITY);
            let now = std::time::Instant::now();
            sessions.insert(
                session_id,
                SessionEntry {
                    workspace_id: agent.id.clone(),
                    approval_policy,
                    sandbox_mode,
                    submission_tx,
                    events_tx,
                    event_log: Arc::new(RwLock::new(SessionEventLog::new(
                        DEFAULT_EVENT_REPLAY_BUFFER_CAPACITY,
                    ))),
                    event_bridge_task: None,
                    rollout_path,
                    created_at: now,
                    last_inbound_activity: now,
                    last_outbound_activity: now,
                },
            );
            recovered += 1;
        }

        self.sessions_recovered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        info!(recovered, "Recovered persisted session bindings");
        Ok(())
    }

    /// Create new application state
    ///
    /// Note: The cleanup task is NOT started automatically.
    /// Call `start_cleanup_task()` after the tokio runtime is initialized,
    /// or use `create_session()` which will lazily start it.
    pub fn new(config: Config) -> Self {
        let runtime_config = WorkspaceRuntimeConfig::from(config.clone());
        let manager_config = ManagerConfig::default();
        let workspace_manager =
            WorkspaceManager::with_runtime_config(manager_config, runtime_config);

        Self::from_parts(
            config,
            Arc::new(workspace_manager),
            DEFAULT_SESSION_TTL_SECS,
        )
    }

    /// Create new application state with custom TTL
    #[allow(dead_code)]
    pub fn with_ttl(config: Config, ttl_secs: u64) -> Self {
        let runtime_config = WorkspaceRuntimeConfig::from(config.clone());
        let manager_config = ManagerConfig::default();
        let workspace_manager =
            WorkspaceManager::with_runtime_config(manager_config, runtime_config);

        Self::from_parts(config, Arc::new(workspace_manager), ttl_secs)
    }

    pub(crate) fn from_parts(
        config: Config,
        workspace_manager: Arc<WorkspaceManager>,
        ttl_secs: u64,
    ) -> Self {
        Self {
            config,
            workspace_manager,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_secs: ttl_secs,
            cleanup_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sessions_recovered: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recovery_lock: Arc::new(Mutex::new(())),
        }
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

        let sessions = Arc::clone(&self.sessions);
        let workspace_manager = Arc::clone(&self.workspace_manager);
        let ttl = Duration::from_secs(self.session_ttl_secs);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(60),
                Duration::from_secs(60),
            );

            loop {
                interval.tick().await;

                // Check for expired sessions
                let expired_sessions: Vec<(String, String)> = {
                    let sessions_guard = sessions.read().await;
                    let mut expired = Vec::new();

                    for (session_id, entry) in sessions_guard.iter() {
                        if entry.is_expired(ttl) {
                            expired.push((session_id.clone(), entry.workspace_id.clone()));
                        }
                    }
                    expired
                };

                // Second pass: remove expired sessions
                for (session_id, workspace_id) in expired_sessions {
                    warn!(%session_id, %workspace_id, "Session expired, cleaning up");

                    // Destroy agent first, then remove session only if successful
                    match workspace_manager.destroy(&workspace_id).await {
                        Ok(()) => {
                            // Only remove session if agent was destroyed successfully
                            sessions.write().await.remove(&session_id);
                            info!(%session_id, "Expired session cleaned up");
                        }
                        Err(err) => {
                            warn!(%session_id, %workspace_id, error = %err, "Failed to destroy expired agent, keeping session for retry");
                            // Session is kept in the map for potential retry
                            // Could add failure count tracking here for eventual cleanup
                        }
                    }
                }
            }
        });
    }

    /// Create a new session and return its ID.
    ///
    /// Lazily starts the cleanup task if not already started.
    #[allow(dead_code)]
    pub async fn create_session(
        &self,
        workspace_dir: Option<std::path::PathBuf>,
    ) -> anyhow::Result<String> {
        self.create_session_from_rollout(workspace_dir, None, None, None)
            .await
    }

    /// Create a new session, optionally preloading runtime context from an existing rollout.
    pub async fn create_session_from_rollout(
        &self,
        workspace_dir: Option<std::path::PathBuf>,
        resume_rollout_path: Option<PathBuf>,
        approval_policy: Option<alan_protocol::ApprovalPolicy>,
        sandbox_mode: Option<alan_protocol::SandboxMode>,
    ) -> anyhow::Result<String> {
        self.ensure_sessions_recovered().await?;
        // Lazily start cleanup task on first session creation
        self.start_cleanup_task();
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create and start an agent.
        let mut runtime_config = WorkspaceRuntimeConfig::from(self.config.clone());
        runtime_config.workspace_dir = workspace_dir;
        runtime_config.resume_rollout_path = resume_rollout_path;
        if let Some(approval_policy) = approval_policy {
            runtime_config.agent_config.runtime_config.approval_policy = approval_policy;
        }
        if let Some(sandbox_mode) = sandbox_mode {
            runtime_config.agent_config.runtime_config.sandbox_mode = sandbox_mode;
        }
        let approval_policy = runtime_config.agent_config.runtime_config.approval_policy;
        let sandbox_mode = runtime_config.agent_config.runtime_config.sandbox_mode;
        let workspace_id = self
            .workspace_manager
            .create_and_start(runtime_config)
            .await?;
        if let Err(err) = self
            .persist_workspace_session_binding(&workspace_id, Some(session_id.clone()))
            .await
        {
            warn!(
                %workspace_id,
                %session_id,
                error = %err,
                "Failed to persist session binding to agent state"
            );
        }

        // Get the runtime handle
        let handle = self.workspace_manager.get_handle(&workspace_id).await?;
        let rollout_path = {
            let instance = self.workspace_manager.get(&workspace_id).await?;
            let instance = instance.read().await;
            detect_latest_rollout_path(&instance.workspace_dir.join("sessions"))
        };
        let (events_tx, _) = broadcast::channel(DEFAULT_EVENT_BROADCAST_CAPACITY);
        let event_log = Arc::new(RwLock::new(SessionEventLog::new(
            DEFAULT_EVENT_REPLAY_BUFFER_CAPACITY,
        )));
        let event_bridge_task = Some(Self::spawn_event_bridge(
            session_id.clone(),
            handle.event_sender.subscribe(),
            events_tx.clone(),
            Arc::clone(&event_log),
        ));

        let now = std::time::Instant::now();
        let entry = SessionEntry {
            workspace_id,
            approval_policy,
            sandbox_mode,
            submission_tx: handle.submission_tx,
            events_tx,
            event_log,
            event_bridge_task,
            rollout_path,
            created_at: now,
            last_inbound_activity: now,
            last_outbound_activity: now,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), entry);

        Ok(session_id)
    }

    /// Ensure a session's runtime is running and refresh channels/rollout path.
    pub async fn resume_session_runtime(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        let workspace_id = {
            let sessions = self.sessions.read().await;
            match sessions.get(id) {
                Some(entry) => entry.workspace_id.clone(),
                None => anyhow::bail!("Session {} not found", id),
            }
        };

        self.workspace_manager.start(&workspace_id).await?;
        let handle = self.workspace_manager.get_handle(&workspace_id).await?;
        let rollout_path = {
            let instance = self.workspace_manager.get(&workspace_id).await?;
            let instance = instance.read().await;
            detect_latest_rollout_path(&instance.workspace_dir.join("sessions"))
        };

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
        );
        entry.submission_tx = handle.submission_tx;
        entry.event_bridge_task = Some(new_bridge);
        entry.rollout_path = rollout_path;
        entry.touch_outbound();
        Ok(())
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: &str) -> Option<bool> {
        let _ = self.ensure_sessions_recovered().await;
        self.sessions.read().await.contains_key(id).then_some(true)
    }

    /// Update a session entry's rollout path.
    pub async fn set_session_rollout_path(&self, id: &str, path: Option<PathBuf>) {
        let _ = self.ensure_sessions_recovered().await;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.rollout_path = path;
        }
    }

    /// Get a mutable session entry (for updating inbound activity)
    pub async fn touch_session_inbound(&self, id: &str) {
        let _ = self.ensure_sessions_recovered().await;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.touch_inbound();
        }
    }

    /// Update outbound activity (event sent to client)
    pub async fn touch_session_outbound(&self, id: &str) {
        let _ = self.ensure_sessions_recovered().await;
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(id) {
            entry.touch_outbound();
        }
    }

    /// Remove a session
    ///
    /// First destroys the agent, then removes the session only if successful.
    /// This ensures we don't leave orphan agents if destroy fails.
    pub async fn remove_session(&self, id: &str) -> anyhow::Result<()> {
        self.ensure_sessions_recovered().await?;
        // Get workspace_id first while holding the lock briefly
        let workspace_id = {
            let sessions = self.sessions.read().await;
            sessions.get(id).map(|e| e.workspace_id.clone())
        };

        let workspace_id = match workspace_id {
            Some(id) => id,
            None => return Ok(()), // Already removed
        };

        // Destroy agent first
        if let Err(err) = self.workspace_manager.destroy(&workspace_id).await {
            warn!(
                session_id = id,
                agent_id = %workspace_id,
                error = %err,
                "Failed to destroy agent while removing session"
            );
            return Err(err);
        }

        // Only remove session if agent was destroyed successfully
        if let Some(mut entry) = self.sessions.write().await.remove(id)
            && let Some(task) = entry.event_bridge_task.take()
        {
            task.abort();
        }
        Ok(())
    }

    /// Clean up all expired sessions (can be called manually)
    #[allow(dead_code)]
    pub async fn cleanup_expired(&self) -> usize {
        let _ = self.ensure_sessions_recovered().await;
        let ttl = Duration::from_secs(self.session_ttl_secs);

        let expired: Vec<(String, String)> = {
            let sessions_guard = self.sessions.read().await;
            sessions_guard
                .iter()
                .filter(|(_, entry)| entry.is_expired(ttl))
                .map(|(session_id, entry)| (session_id.clone(), entry.workspace_id.clone()))
                .collect()
        };

        let mut removed_count = 0;
        for (session_id, _) in expired {
            match self.remove_session(&session_id).await {
                Ok(()) => removed_count += 1,
                Err(_) => {
                    // Failed to remove, will be retried on next cleanup
                }
            }
        }

        removed_count
    }
}

impl AppState {
    async fn persist_workspace_session_binding(
        &self,
        agent_id: &str,
        session_id: Option<String>,
    ) -> anyhow::Result<()> {
        let instance = self.workspace_manager.get(agent_id).await?;
        let instance_guard = instance.read().await;
        let state_arc = Arc::clone(&instance_guard.state);
        let workspace_dir = instance_guard.workspace_dir.clone();
        drop(instance_guard);

        let mut state_guard = state_arc.write().await;
        state_guard.current_session_id = session_id;
        state_guard.save(&workspace_dir)?;
        Ok(())
    }

    fn spawn_event_bridge(
        session_id: String,
        mut runtime_events_rx: broadcast::Receiver<RuntimeEventEnvelope>,
        client_events_tx: broadcast::Sender<EventEnvelope>,
        event_log: Arc<RwLock<SessionEventLog>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match runtime_events_rx.recv().await {
                    Ok(event) => {
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

    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(sessions_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
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

    latest.map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::{ManagerConfig, WorkspaceManager};
    use alan_runtime::manager::WorkspaceState as PersistedWorkspaceState;
    use alan_runtime::runtime::{RuntimeEventEnvelope, WorkspaceRuntimeConfig};
    use tempfile::TempDir;

    fn runtime_event(event: Event) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            submission_id: Some("sub-test".to_string()),
            event,
        }
    }

    fn test_state() -> AppState {
        let path = std::env::temp_dir().join(format!("agentd-state-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        test_state_with_manager(&path)
    }

    fn test_state_with_manager(base_dir: &std::path::Path) -> AppState {
        let manager = WorkspaceManager::with_runtime_config(
            ManagerConfig::with_base_dir(base_dir.to_path_buf()),
            WorkspaceRuntimeConfig::from(Config::default()),
        );
        AppState::from_parts(Config::default(), Arc::new(manager), 1)
    }

    fn test_session_entry(workspace_id: &str) -> (SessionEntry, mpsc::Receiver<Submission>) {
        let (submission_tx, submission_rx) = mpsc::channel(8);
        let (events_tx, _) = broadcast::channel(8);
        let event_log = Arc::new(RwLock::new(SessionEventLog::new(16)));
        let now = std::time::Instant::now();
        (
            SessionEntry {
                workspace_id: workspace_id.to_string(),
                approval_policy: alan_protocol::ApprovalPolicy::OnRequest,
                sandbox_mode: alan_protocol::SandboxMode::WorkspaceWrite,
                submission_tx,
                events_tx,
                event_log,
                event_bridge_task: None,
                rollout_path: None,
                created_at: now,
                last_inbound_activity: now,
                last_outbound_activity: now,
            },
            submission_rx,
        )
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
    fn session_entry_touch_updates_timestamps() {
        let (mut entry, _rx) = test_session_entry("a1");
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
        let (mut entry, _rx) = test_session_entry("a1");
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
            runtime_event(Event::MessageDelta {
                content: "hello".to_string(),
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
                    Event::MessageDelta {
                        content: format!("turn {i}"),
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
        assert_eq!(state.get_session("missing").await, None);
        state.touch_session_inbound("missing").await;
        state.touch_session_outbound("missing").await;
        state.remove_session("missing").await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_expired_removes_session_even_if_agent_id_is_stale() {
        let state = test_state();
        let (mut entry, _rx) = test_session_entry("nonexistent-agent");
        let old = std::time::Instant::now() - std::time::Duration::from_secs(10);
        entry.last_inbound_activity = old;
        entry.last_outbound_activity = old;

        state
            .sessions
            .write()
            .await
            .insert("sess-1".to_string(), entry);

        let removed = state.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert!(state.get_session("sess-1").await.is_none());
    }

    #[tokio::test]
    async fn persist_workspace_session_binding_writes_agent_state() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_manager(temp.path());
        let runtime_config = WorkspaceRuntimeConfig::from(Config::default());
        let workspace_id = state
            .workspace_manager
            .create(runtime_config)
            .await
            .unwrap();

        state
            .persist_workspace_session_binding(&workspace_id, Some("sess-bind".to_string()))
            .await
            .unwrap();

        let loaded = PersistedWorkspaceState::load(&temp.path().join(&workspace_id)).unwrap();
        assert_eq!(loaded.current_session_id.as_deref(), Some("sess-bind"));
    }

    #[tokio::test]
    async fn ensure_sessions_recovered_loads_persisted_bindings_from_disk() {
        let temp = TempDir::new().unwrap();
        let state = test_state_with_manager(temp.path());

        let agent_dir = temp.path().join("agent-recover");
        std::fs::create_dir_all(agent_dir.join("sessions")).unwrap();
        std::fs::write(agent_dir.join("sessions").join("rollout-1.jsonl"), "{}\n").unwrap();

        let mut persisted = PersistedWorkspaceState::new("agent-recover".to_string());
        persisted.current_session_id = Some("sess-recover".to_string());
        persisted.config.approval_policy = Some(alan_protocol::ApprovalPolicy::Never);
        persisted.config.sandbox_mode = Some(alan_protocol::SandboxMode::ReadOnly);
        persisted.save(&agent_dir).unwrap();

        state.ensure_sessions_recovered().await.unwrap();
        state.ensure_sessions_recovered().await.unwrap(); // idempotent

        let sessions = state.sessions.read().await;
        let entry = sessions.get("sess-recover").unwrap();
        assert_eq!(entry.workspace_id, "agent-recover");
        assert_eq!(entry.approval_policy, alan_protocol::ApprovalPolicy::Never);
        assert_eq!(entry.sandbox_mode, alan_protocol::SandboxMode::ReadOnly);
        assert!(entry.rollout_path.as_ref().is_some());
        assert_eq!(sessions.len(), 1);
    }
}
