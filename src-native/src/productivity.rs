//! Local Nook utilities. The caller supplies epoch milliseconds; this module never runs a clock.
use serde::{Deserialize, Serialize};
use crate::nook::PresentationPreference;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError},
    thread,
};

pub const MAX_NOTE_BYTES: usize = 16 * 1024;
pub const MAX_TASKS: usize = 256;
pub const MAX_TASK_TEXT_BYTES: usize = 512;
pub const MAX_FILES: usize = 128;
pub const MAX_STATE_BYTES: usize = 256 * 1024;
pub const SAVE_DEBOUNCE_MS: i64 = 500;
const FILE_NAME: &str = "nook-productivity.json";
const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub text: String,
    pub complete: bool,
    pub favorite: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReference {
    pub path: PathBuf,
    pub added_at_ms: i64,
}

impl FileReference {
    /// Missing is derived at display time; the shelf never deletes or opens referenced files.
    pub fn missing(&self) -> bool { !self.path.exists() }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Timer {
    Idle,
    Running { deadline_ms: i64 },
    Paused { remaining_ms: i64 },
    Completed { acknowledged: bool },
}

impl Timer {
    pub fn remaining_ms(&self, now_ms: i64) -> i64 {
        match self {
            Self::Running { deadline_ms } => deadline_ms.saturating_sub(now_ms).max(0),
            Self::Paused { remaining_ms } => *remaining_ms,
            _ => 0,
        }
    }
    pub fn start(&mut self, now_ms: i64, duration_ms: i64) -> Result<(), String> {
        if duration_ms <= 0 { return Err("Timer duration must be positive".into()); }
        let deadline_ms = now_ms.checked_add(duration_ms).ok_or("Timer deadline overflow")?;
        *self = Self::Running { deadline_ms };
        Ok(())
    }
    pub fn pause(&mut self, now_ms: i64) -> bool {
        if matches!(self, Self::Running { .. }) && self.remaining_ms(now_ms) > 0 {
            *self = Self::Paused { remaining_ms: self.remaining_ms(now_ms) };
            true
        } else { false }
    }
    pub fn resume(&mut self, now_ms: i64) -> bool {
        if let Self::Paused { remaining_ms } = self {
            if let Some(deadline_ms) = now_ms.checked_add(*remaining_ms) {
                *self = Self::Running { deadline_ms };
                return true;
            }
        }
        false
    }
    pub fn cancel(&mut self) { *self = Self::Idle; }
    /// Returns true only for the transition to a pending completion. To avoid
    /// duplicate alerts after restart, acknowledge and persist that state,
    /// then show the alert only after the matching Saved revision arrives.
    pub fn reconcile(&mut self, now_ms: i64) -> bool {
        if matches!(self, Self::Running { deadline_ms } if now_ms >= *deadline_ms) {
            *self = Self::Completed { acknowledged: false };
            true
        } else { false }
    }
    pub fn acknowledge(&mut self) -> bool {
        if let Self::Completed { acknowledged: false } = self {
            *self = Self::Completed { acknowledged: true };
            true
        } else { false }
    }
    pub fn pending_completion(&self) -> bool {
        matches!(self, Self::Completed { acknowledged: false })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductivityState {
    pub version: u32,
    pub note: String,
    pub tasks: Vec<Task>,
    pub next_task_id: u64,
    pub timer: Timer,
    pub files: Vec<FileReference>,
    #[serde(default)]
    pub presentation: PresentationPreference,
}

impl Default for ProductivityState {
    fn default() -> Self {
        Self { version: VERSION, note: String::new(), tasks: Vec::new(), next_task_id: 1,
            timer: Timer::Idle, files: Vec::new(), presentation: PresentationPreference::default() }
    }
}

impl ProductivityState {
    pub fn set_note(&mut self, text: String) -> Result<(), String> {
        if text.len() > MAX_NOTE_BYTES { return Err("Note is too long".into()); }
        self.note = text;
        Ok(())
    }
    pub fn add_task(&mut self, text: String) -> Result<u64, String> {
        check_task_text(&text)?;
        if self.tasks.len() >= MAX_TASKS { return Err("Task limit reached".into()); }
        let id = self.next_task_id;
        self.next_task_id = id.checked_add(1).ok_or("Task ID limit reached")?;
        self.tasks.push(Task { id, text, complete: false, favorite: false });
        Ok(id)
    }
    pub fn edit_task(&mut self, id: u64, text: String) -> Result<bool, String> {
        check_task_text(&text)?;
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.text = text;
            Ok(true)
        } else { Ok(false) }
    }
    pub fn set_task_complete(&mut self, id: u64, complete: bool) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.complete = complete;
            true
        } else { false }
    }
    pub fn set_task_favorite(&mut self, id: u64, favorite: bool) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.favorite = favorite;
            true
        } else { false }
    }
    pub fn remove_task(&mut self, id: u64) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|task| task.id != id);
        self.tasks.len() != before
    }
    pub fn clear_completed(&mut self) -> usize {
        let before = self.tasks.len();
        self.tasks.retain(|task| !task.complete);
        before - self.tasks.len()
    }
    pub fn add_file(&mut self, path: PathBuf, added_at_ms: i64) -> Result<bool, String> {
        let path = shelf_path(&path)?;
        if self.files.iter().any(|file| same_path(&file.path, &path)) { return Ok(false); }
        if self.files.len() >= MAX_FILES { return Err("File shelf limit reached".into()); }
        self.files.push(FileReference { path, added_at_ms });
        Ok(true)
    }
    pub fn remove_file(&mut self, path: &Path) -> bool {
        let before = self.files.len();
        self.files.retain(|file| !same_path(&file.path, path));
        self.files.len() != before
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION { return Err(format!("Unsupported productivity version {}", self.version)); }
        if self.note.len() > MAX_NOTE_BYTES || self.tasks.len() > MAX_TASKS || self.files.len() > MAX_FILES {
            return Err("Productivity limits exceeded".into());
        }
        let mut ids = std::collections::HashSet::new();
        for task in &self.tasks {
            check_task_text(&task.text)?;
            if task.id == 0 || task.id >= self.next_task_id || !ids.insert(task.id) { return Err("Invalid task ID".into()); }
        }
        if self.next_task_id == 0 { return Err("Invalid next task ID".into()); }
        let mut paths = std::collections::HashSet::new();
        for file in &self.files {
            let path = shelf_path(&file.path)?;
            if !paths.insert(path_key(&path)) { return Err("Duplicate file reference".into()); }
        }
        if matches!(&self.timer, Timer::Paused { remaining_ms } if *remaining_ms <= 0) {
            return Err("Invalid paused timer".into());
        }
        self.presentation.validate()?;
        Ok(())
    }
}

fn check_task_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() || text.len() > MAX_TASK_TEXT_BYTES {
        Err("Task text must be nonempty and at most 512 bytes".into())
    } else { Ok(()) }
}

fn shelf_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || path.as_os_str().len() > 4096 ||
        path.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir)) {
        return Err("File reference must be a bounded absolute path without traversal".into());
    }
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}
fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().into_owned();
    if cfg!(windows) { value.to_lowercase() } else { value }
}
fn same_path(a: &Path, b: &Path) -> bool {
    match (shelf_path(a), shelf_path(b)) {
        (Ok(a), Ok(b)) => path_key(&a) == path_key(&b),
        _ => false,
    }
}

#[derive(Debug)]
pub struct LoadResult {
    pub state: ProductivityState,
    pub recovered_from_backup: bool,
    pub recovered_from_temporary: bool,
    /// Nonempty when the primary file was corrupt or unreadable.
    pub warning: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Store { directory: PathBuf }

impl Store {
    pub fn new(directory: &Path) -> Result<Self, String> {
        if !directory.is_dir() { return Err("Productivity directory does not exist".into()); }
        let directory = directory.canonicalize().map_err(|error| error.to_string())?;
        Ok(Self { directory })
    }
    fn primary(&self) -> PathBuf { self.directory.join(FILE_NAME) }
    fn backup(&self) -> PathBuf { self.directory.join(format!("{FILE_NAME}.bak")) }
    fn temporary(&self) -> PathBuf { self.directory.join(format!("{FILE_NAME}.tmp")) }
    pub fn load(&self) -> Result<LoadResult, String> {
        let primary = self.primary();
        if !primary.exists() && !self.backup().exists() && !self.temporary().exists() {
            return Ok(LoadResult { state: ProductivityState::default(), recovered_from_backup: false,
                recovered_from_temporary: false, warning: None });
        }
        match read_state(&primary) {
            Ok(state) => Ok(LoadResult { state, recovered_from_backup: false,
                recovered_from_temporary: false, warning: None }),
            Err(error) => {
                // After primary -> backup but before temporary -> primary, the
                // validated temporary file is the newer committed snapshot.
                let primary_missing = matches!(fs::symlink_metadata(&primary),
                    Err(ref missing) if missing.kind() == std::io::ErrorKind::NotFound);
                if primary_missing {
                    match read_state(&self.temporary()) {
                        Ok(state) => Ok(LoadResult { state, recovered_from_backup: false,
                            recovered_from_temporary: true, warning: Some(error) }),
                        Err(temporary_error) => match read_state(&self.backup()) {
                            Ok(state) => Ok(LoadResult { state, recovered_from_backup: true,
                                recovered_from_temporary: false,
                                warning: Some(format!("Primary: {error}; temporary: {temporary_error}")) }),
                            Err(backup_error) => Err(format!("Primary: {error}; temporary: {temporary_error}; backup: {backup_error}")),
                        },
                    }
                } else {
                    match read_state(&self.backup()) {
                        Ok(state) => Ok(LoadResult { state, recovered_from_backup: true,
                            recovered_from_temporary: false, warning: Some(error) }),
                        Err(backup_error) => match read_state(&self.temporary()) {
                            Ok(state) => Ok(LoadResult { state, recovered_from_backup: false,
                                recovered_from_temporary: true,
                                warning: Some(format!("Primary: {error}; backup: {backup_error}")) }),
                            Err(temporary_error) => Err(format!("Primary: {error}; backup: {backup_error}; temporary: {temporary_error}")),
                        },
                    }
                }
            }
        }
    }
    pub fn save(&self, state: &ProductivityState) -> Result<(), String> {
        state.validate()?;
        let bytes = serde_json::to_vec(state).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_STATE_BYTES { return Err("Productivity state exceeds size limit".into()); }
        let target = self.primary();
        let backup = self.backup();
        let temporary = self.temporary();
        // Fixed path in the canonical profile directory; a previous interrupted write
        // can be recovered by load() before this save replaces its stale temp file.
        if temporary.exists() { fs::remove_file(&temporary).map_err(|e| e.to_string())?; }
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)
            .map_err(|error| format!("Cannot create temporary productivity file: {error}"))?;
        let write_result = file.write_all(&bytes).and_then(|_| file.sync_all());
        drop(file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot write productivity file: {error}"));
        }
        // Only replace a backup with a known-good primary. A recovered backup stays intact.
        let prepare = (|| -> Result<(), String> {
            if target.exists() {
                if read_state(&target).is_ok() {
                    if backup.exists() { fs::remove_file(&backup).map_err(|e| e.to_string())?; }
                    fs::rename(&target, &backup).map_err(|e| e.to_string())
                } else {
                    fs::remove_file(&target).map_err(|e| e.to_string())
                }
            } else {
                Ok(())
            }
        })();
        if let Err(error) = prepare {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot prepare productivity backup: {error}"));
        }
        if let Err(error) = fs::rename(&temporary, &target) {
            if !target.exists() && backup.exists() { let _ = fs::copy(&backup, &target); }
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot replace productivity file: {error}"));
        }
        if let Ok(directory) = File::open(&self.directory) { let _ = directory.sync_all(); }
        Ok(())
    }
}

fn read_state(path: &Path) -> Result<ProductivityState, String> {
    let identity = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !identity.file_type().is_file() {
        return Err("Productivity path is not a regular file".into());
    }
    let file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_STATE_BYTES as u64 {
        return Err("Productivity file exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_STATE_BYTES + 1) as u64).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_STATE_BYTES { return Err("Productivity file exceeds size limit".into()); }
    let state: ProductivityState = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    state.validate()?;
    Ok(state)
}

pub enum SaveCommand {
    Load { reply: Sender<Result<LoadResult, String>> },
    Save { revision: u64, state: ProductivityState },
    Flush { reply: Sender<Result<Option<u64>, String>> },
    Stop { reply: Sender<Result<Option<u64>, String>> },
}
#[derive(Debug, PartialEq, Eq)]
pub enum SaveEvent { Saved { revision: u64 }, Failed { revision: u64, error: String } }

pub struct SaveWorker {
    pub commands: SyncSender<SaveCommand>,
    pub events: Receiver<SaveEvent>,
    handle: Option<thread::JoinHandle<()>>,
}
impl SaveWorker {
    pub fn start(store: Store) -> Self {
        let (commands, input) = mpsc::sync_channel(16);
        let (output, events) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut highest_saved = None;
            let mut highest_seen = None;
            let mut last_error = None;
            while let Ok(command) = input.recv() {
                match command {
                    SaveCommand::Load { reply } => { let _ = reply.send(store.load()); }
                    SaveCommand::Save { revision, state } => {
                        if highest_seen.is_some_and(|highest| revision < highest) ||
                            highest_saved.is_some_and(|highest| revision <= highest) {
                            let _ = output.send(SaveEvent::Failed {
                                revision, error: "Stale productivity revision".into(),
                            });
                            continue;
                        }
                        highest_seen = Some(revision);
                        let result = store.save(&state);
                        match result {
                            Ok(()) => { highest_saved = Some(revision); last_error = None;
                                let _ = output.send(SaveEvent::Saved { revision }); }
                            Err(error) => { last_error = Some(error.clone());
                                let _ = output.send(SaveEvent::Failed { revision, error }); }
                        }
                    }
                    SaveCommand::Flush { reply } => {
                        let _ = reply.send(last_error.clone().map_or(Ok(highest_saved), Err));
                    }
                    SaveCommand::Stop { reply } => {
                        if let Some(error) = &last_error {
                            let _ = reply.send(Err(error.clone()));
                        } else {
                            let _ = reply.send(Ok(highest_saved));
                            break;
                        }
                    }
                }
            }
        });
        Self { commands, events, handle: Some(handle) }
    }
    /// Use this on the UI thread so a full worker queue cannot block a frame.
    pub fn try_save(&self, revision: u64, state: ProductivityState) -> Result<(), TrySendError<SaveCommand>> {
        self.commands.try_send(SaveCommand::Save { revision, state })
    }
    /// Call outside a frame (for example during shutdown) after sending Stop and receiving its reply.
    pub fn join(&mut self) -> thread::Result<()> {
        if let Some(handle) = self.handle.take() { handle.join() } else { Ok(()) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControllerEvent {
    Loaded { recovered: bool, warning: Option<String> },
    LoadFailed(String),
    Saved(u64),
    SaveFailed { revision: u64, error: String },
    TimerCompleted,
    StopFailed(String),
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopStage { Idle, WaitingSave, AwaitFlush, QueueStop, AwaitStop, Done }

/// Own one controller per profile. Poll is channel-only and never performs file I/O.
pub struct Controller {
    pub state: ProductivityState,
    worker: SaveWorker,
    load_reply: Option<Receiver<Result<LoadResult, String>>>,
    stop_reply: Option<Receiver<Result<Option<u64>, String>>>,
    stop_stage: StopStage,
    loaded: bool,
    writable: bool,
    revision: u64,
    enqueued_revision: u64,
    saved_revision: u64,
    dirty_at_ms: Option<i64>,
    completion_revision: Option<u64>,
    last_error: Option<String>,
}

impl Controller {
    /// The profile directory must already exist; the initial read runs on the worker.
    pub fn start(profile_dir: &Path) -> Result<Self, String> {
        let worker = SaveWorker::start(Store::new(profile_dir)?);
        let (reply, load_reply) = mpsc::channel();
        worker.commands.try_send(SaveCommand::Load { reply })
            .map_err(|_| "Cannot queue productivity load".to_string())?;
        Ok(Self { state: ProductivityState::default(), worker, load_reply: Some(load_reply),
            stop_reply: None, stop_stage: StopStage::Idle, loaded: false, writable: false,
            revision: 0, enqueued_revision: 0, saved_revision: 0, dirty_at_ms: None,
            completion_revision: None, last_error: None })
    }
    pub fn loading(&self) -> bool { !self.loaded }
    pub fn writable(&self) -> bool { self.loaded && self.writable && self.stop_stage == StopStage::Idle }
    pub fn error(&self) -> Option<&str> { self.last_error.as_deref() }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn saved_revision(&self) -> u64 { self.saved_revision }
    /// True from an accepted stop request through the Stop acknowledgement.
    pub fn stop_requested(&self) -> bool { self.stop_stage != StopStage::Idle }
    pub fn ready_to_stop(&self) -> bool { self.stop_stage == StopStage::Done }
    pub fn needs_repaint(&self) -> bool {
        self.loading() || self.dirty_at_ms.is_some() ||
            self.enqueued_revision > self.saved_revision || self.completion_revision.is_some() ||
            matches!(self.stop_stage, StopStage::WaitingSave | StopStage::AwaitFlush |
                StopStage::QueueStop | StopStage::AwaitStop)
    }
    /// Call after each successful state edit. Failed initial load never enables writes.
    pub fn mark_changed(&mut self, now_ms: i64) -> Result<u64, String> {
        if !self.writable() { return Err("Productivity state is not writable".into()); }
        self.revision = self.revision.checked_add(1).ok_or("Productivity revision limit reached")?;
        self.dirty_at_ms = Some(now_ms);
        Ok(self.revision)
    }
    /// A completion event is emitted only after the acknowledged timer is saved.
    pub fn reconcile_timer(&mut self, now_ms: i64) -> Result<bool, String> {
        if !self.writable() { return Ok(false); }
        let due = self.state.timer.reconcile(now_ms);
        if self.state.timer.pending_completion() {
            self.state.timer.acknowledge();
            let revision = self.mark_changed(now_ms)?;
            self.completion_revision = Some(revision);
            self.dirty_at_ms = Some(now_ms.saturating_sub(SAVE_DEBOUNCE_MS));
        }
        Ok(due)
    }
    /// Poll at roughly 100 ms while needs_repaint is true, and at timer deadline.
    pub fn poll(&mut self, now_ms: i64) -> Vec<ControllerEvent> {
        let mut events = Vec::new();
        if let Some(reply) = &self.load_reply {
            match reply.try_recv() {
                Ok(Ok(loaded)) => {
                    self.state = loaded.state;
                    self.loaded = true;
                    self.writable = true;
                    self.last_error = loaded.warning.clone();
                    events.push(ControllerEvent::Loaded {
                        recovered: loaded.recovered_from_backup || loaded.recovered_from_temporary,
                        warning: loaded.warning,
                    });
                    self.load_reply = None;
                    if let Err(error) = self.reconcile_timer(now_ms) {
                        self.last_error = Some(error.clone());
                        events.push(ControllerEvent::SaveFailed { revision: self.revision, error });
                    }
                }
                Ok(Err(error)) => {
                    self.loaded = true;
                    self.writable = false;
                    self.last_error = Some(error.clone());
                    self.load_reply = None;
                    events.push(ControllerEvent::LoadFailed(error));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    let error = "Productivity load worker disconnected".to_string();
                    self.loaded = true;
                    self.writable = false;
                    self.last_error = Some(error.clone());
                    self.load_reply = None;
                    events.push(ControllerEvent::LoadFailed(error));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        while let Ok(event) = self.worker.events.try_recv() {
            match event {
                SaveEvent::Saved { revision } => {
                    self.saved_revision = self.saved_revision.max(revision);
                    self.last_error = None;
                    events.push(ControllerEvent::Saved(revision));
                    if self.completion_revision.is_some_and(|needed| revision >= needed) {
                        self.completion_revision = None;
                        if matches!(&self.state.timer, Timer::Completed { acknowledged: true }) {
                            events.push(ControllerEvent::TimerCompleted);
                        }
                    }
                }
                SaveEvent::Failed { revision, error } => {
                    if revision == self.enqueued_revision {
                        self.enqueued_revision = self.saved_revision;
                        self.dirty_at_ms = Some(now_ms);
                    }
                    self.last_error = Some(error.clone());
                    events.push(ControllerEvent::SaveFailed { revision, error: error.clone() });
                    if self.stop_stage == StopStage::WaitingSave {
                        self.stop_failure(error, &mut events);
                    }
                }
            }
        }
        if self.writable && matches!(self.stop_stage, StopStage::Idle | StopStage::WaitingSave) &&
            self.revision > self.enqueued_revision &&
            self.dirty_at_ms.is_some_and(|dirty|
                self.stop_stage == StopStage::WaitingSave ||
                now_ms.saturating_sub(dirty) >= SAVE_DEBOUNCE_MS) {
            if self.worker.try_save(self.revision, self.state.clone()).is_ok() {
                self.enqueued_revision = self.revision;
                self.dirty_at_ms = None;
            }
        }
        self.advance_stop(&mut events);
        events
    }
    /// Start a nonblocking quit/update handshake; poll for Stopped or StopFailed.
    pub fn request_stop(&mut self) -> Result<(), String> {
        if !self.loaded { return Err("Productivity load is still pending".into()); }
        if self.stop_stage != StopStage::Idle { return Err("Productivity stop already pending".into()); }
        // A failed initial load has no edits to flush. Stop the worker without
        // replacing the corrupt file with the in-memory default state.
        if !self.writable && self.revision != 0 {
            return Err("Productivity state has unsaved edits".into());
        }
        if self.saved_revision < self.revision {
            if let Some(error) = &self.last_error { return Err(error.clone()); }
        }
        self.stop_stage = StopStage::WaitingSave;
        Ok(())
    }
    /// Optional thread cleanup after Stopped; call outside an interactive frame.
    pub fn join_stopped(&mut self) -> Result<(), String> {
        if !self.ready_to_stop() { return Err("Productivity worker has not acknowledged Stop".into()); }
        self.worker.join().map_err(|_| "Productivity worker panicked".into())
    }
    fn advance_stop(&mut self, events: &mut Vec<ControllerEvent>) {
        match self.stop_stage {
            StopStage::WaitingSave if self.saved_revision >= self.revision => {
                let (reply, receiver) = mpsc::channel();
                match self.worker.commands.try_send(SaveCommand::Flush { reply }) {
                    Ok(()) => {
                        self.stop_reply = Some(receiver);
                        self.stop_stage = StopStage::AwaitFlush;
                    }
                    Err(TrySendError::Disconnected(_)) =>
                        self.stop_failure("Productivity stop worker disconnected".into(), events),
                    Err(TrySendError::Full(_)) => {}
                }
            }
            StopStage::QueueStop => {
                let (reply, receiver) = mpsc::channel();
                match self.worker.commands.try_send(SaveCommand::Stop { reply }) {
                    Ok(()) => {
                        self.stop_reply = Some(receiver);
                        self.stop_stage = StopStage::AwaitStop;
                    }
                    Err(TrySendError::Disconnected(_)) =>
                        self.stop_failure("Productivity stop worker disconnected".into(), events),
                    Err(TrySendError::Full(_)) => {}
                }
            }
            StopStage::AwaitFlush | StopStage::AwaitStop => {
                let result = self.stop_reply.as_ref().map(|reply| reply.try_recv());
                match result {
                    Some(Ok(Ok(saved))) if saved.unwrap_or(0) >= self.revision => {
                        self.stop_reply = None;
                        if self.stop_stage == StopStage::AwaitFlush {
                            self.stop_stage = StopStage::QueueStop;
                        } else {
                            self.stop_stage = StopStage::Done;
                            events.push(ControllerEvent::Stopped);
                        }
                    }
                    Some(Ok(Ok(_))) => self.stop_failure("Productivity flush omitted latest revision".into(), events),
                    Some(Ok(Err(error))) => self.stop_failure(error, events),
                    Some(Err(mpsc::TryRecvError::Disconnected)) =>
                        self.stop_failure("Productivity stop worker disconnected".into(), events),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn stop_failure(&mut self, error: String, events: &mut Vec<ControllerEvent>) {
        self.last_error = Some(error.clone());
        self.stop_stage = StopStage::Idle;
        self.stop_reply = None;
        events.push(ControllerEvent::StopFailed(error));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    fn test_store() -> (PathBuf, Store) {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("nook-productivity-{}-{nonce}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let store = Store::new(&dir).unwrap();
        (dir, store)
    }
    fn poll_until(controller: &mut Controller, now_ms: i64,
        predicate: impl Fn(&[ControllerEvent]) -> bool) -> Vec<ControllerEvent> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let events = controller.poll(now_ms);
            if predicate(&events) { return events; }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("Timed out waiting for productivity worker event");
    }
    #[test]
    fn task_crud_and_bounds() {
        let mut state = ProductivityState::default();
        assert!(state.set_note("x".repeat(MAX_NOTE_BYTES + 1)).is_err());
        let id = state.add_task("do it".into()).unwrap();
        assert_eq!(state.edit_task(id, "done".into()), Ok(true));
        assert!(state.set_task_complete(id, true));
        assert!(state.set_task_favorite(id, true));
        assert_eq!(state.clear_completed(), 1);
        assert!(!state.remove_task(id));
    }
    #[test]
    fn timer_deadline_and_acknowledgement_survive_serialization() {
        let mut timer = Timer::Idle;
        timer.start(100, 500).unwrap();
        assert!(timer.pause(200));
        assert_eq!(timer.remaining_ms(999), 400);
        assert!(timer.resume(1_000));
        let mut timer: Timer = serde_json::from_slice(&serde_json::to_vec(&timer).unwrap()).unwrap();
        assert!(!timer.reconcile(1_399));
        assert!(timer.reconcile(1_400));
        assert!(!timer.reconcile(1_401));
        assert!(timer.pending_completion());
        assert!(timer.acknowledge());
        assert!(!timer.acknowledge());
    }
    #[test]
    fn save_load_and_corrupt_primary_recover_backup() {
        let (dir, store) = test_store();
        let mut state = ProductivityState::default();
        state.set_note("first".into()).unwrap();
        store.save(&state).unwrap();
        state.set_note("second".into()).unwrap();
        store.save(&state).unwrap();
        assert_eq!(store.load().unwrap().state.note, "second");
        fs::write(store.primary(), b"broken json").unwrap();
        let recovered = store.load().unwrap();
        assert!(recovered.recovered_from_backup);
        assert_eq!(recovered.state.note, "first");
        store.save(&recovered.state).unwrap();
        assert_eq!(store.load().unwrap().state.note, "first");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn interrupted_temp_can_be_recovered_and_bad_files_are_preserved() {
        let (dir, store) = test_store();
        let mut state = ProductivityState::default();
        state.set_note("from temp".into()).unwrap();
        fs::write(store.temporary(), serde_json::to_vec(&state).unwrap()).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.recovered_from_temporary);
        assert_eq!(loaded.state.note, "from temp");
        store.save(&loaded.state).unwrap();
        assert_eq!(store.load().unwrap().state.note, "from temp");
        fs::write(store.primary(), b"broken").unwrap();
        assert!(store.load().is_err());
        assert_eq!(fs::read(store.primary()).unwrap(), b"broken");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn missing_primary_prefers_newer_valid_temporary_over_backup() {
        let (dir, store) = test_store();
        let mut old = ProductivityState::default();
        old.set_note("backup A".into()).unwrap();
        let mut newer = ProductivityState::default();
        newer.set_note("temporary B".into()).unwrap();
        fs::write(store.backup(), serde_json::to_vec(&old).unwrap()).unwrap();
        fs::write(store.temporary(), serde_json::to_vec(&newer).unwrap()).unwrap();

        let loaded = store.load().unwrap();
        assert!(loaded.recovered_from_temporary);
        assert!(!loaded.recovered_from_backup);
        assert_eq!(loaded.state.note, "temporary B");
        store.save(&loaded.state).unwrap();
        assert_eq!(store.load().unwrap().state.note, "temporary B");
        assert!(!store.temporary().exists());
        assert_eq!(read_state(&store.backup()).unwrap().note, "backup A");

        newer.set_note("later C".into()).unwrap();
        store.save(&newer).unwrap();
        assert_eq!(store.load().unwrap().state.note, "later C");
        assert_eq!(read_state(&store.backup()).unwrap().note, "temporary B");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn missing_primary_uses_backup_when_temporary_is_invalid() {
        let (dir, store) = test_store();
        let mut old = ProductivityState::default();
        old.set_note("backup A".into()).unwrap();
        fs::write(store.backup(), serde_json::to_vec(&old).unwrap()).unwrap();
        fs::write(store.temporary(), b"invalid temporary").unwrap();

        let loaded = store.load().unwrap();
        assert!(loaded.recovered_from_backup);
        assert!(!loaded.recovered_from_temporary);
        assert_eq!(loaded.state.note, "backup A");
        assert_eq!(fs::read(store.temporary()).unwrap(), b"invalid temporary");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn worker_rejects_stale_revision_and_flushes() {
        let (dir, store) = test_store();
        let mut worker = SaveWorker::start(store.clone());
        let mut state = ProductivityState::default();
        state.set_note("latest".into()).unwrap();
        worker.commands.send(SaveCommand::Save { revision: 2, state }).unwrap();
        worker.commands.send(SaveCommand::Save { revision: 1, state: ProductivityState::default() }).unwrap();
        assert_eq!(worker.events.recv().unwrap(), SaveEvent::Saved { revision: 2 });
        assert!(matches!(worker.events.recv().unwrap(), SaveEvent::Failed { revision: 1, .. }));
        let (tx, rx) = mpsc::channel();
        worker.commands.send(SaveCommand::Flush { reply: tx }).unwrap();
        assert_eq!(rx.recv().unwrap().unwrap(), Some(2));
        assert_eq!(store.load().unwrap().state.note, "latest");
        let (tx, rx) = mpsc::channel();
        worker.commands.send(SaveCommand::Stop { reply: tx }).unwrap();
        let _ = rx.recv().unwrap();
        worker.join().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn old_version_one_state_defaults_to_nook_without_losing_data() {
        let mut old = ProductivityState::default();
        old.set_note("existing note".into()).unwrap();
        let id = old.add_task("existing task".into()).unwrap();
        old.set_task_favorite(id, true);
        old.add_file(std::env::temp_dir().join("missing-nook-reference"), 42).unwrap();
        let mut value = serde_json::to_value(&old).unwrap();
        value.as_object_mut().unwrap().remove("presentation");
        let migrated: ProductivityState = serde_json::from_value(value).unwrap();
        assert_eq!(migrated.version, 1);
        assert_eq!(migrated.note, "existing note");
        assert_eq!(migrated.tasks, old.tasks);
        assert_eq!(migrated.files, old.files);
        assert_eq!(migrated.presentation.mode, crate::nook::PresentationMode::Nook);
        assert!(!migrated.presentation.pinned);
    }
    #[test]
    fn failed_initial_load_stops_without_overwriting_corruption_or_repainting_forever() {
        let (dir, store) = test_store();
        fs::write(store.primary(), b"corrupt content").unwrap();
        let mut controller = Controller::start(&dir).unwrap();
        assert!(controller.request_stop().is_err());
        assert!(!controller.stop_requested());
        let events = poll_until(&mut controller, 0, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::LoadFailed(_))));
        assert!(matches!(events.first(), Some(ControllerEvent::LoadFailed(_))));
        assert!(!controller.loading());
        assert!(!controller.writable());
        assert!(!controller.needs_repaint());
        assert!(controller.mark_changed(1).is_err());
        controller.request_stop().unwrap();
        assert!(controller.stop_requested());
        poll_until(&mut controller, 1, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Stopped)));
        assert!(controller.ready_to_stop());
        assert!(!controller.needs_repaint());
        controller.join_stopped().unwrap();
        assert_eq!(fs::read(store.primary()).unwrap(), b"corrupt content");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn latest_revision_is_saved_before_stop_acknowledgement() {
        let (dir, store) = test_store();
        let mut controller = Controller::start(&dir).unwrap();
        poll_until(&mut controller, 0, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Loaded { .. })));
        controller.state.set_note("first".into()).unwrap();
        controller.mark_changed(0).unwrap();
        controller.state.set_note("latest".into()).unwrap();
        controller.mark_changed(100).unwrap();
        controller.request_stop().unwrap();
        poll_until(&mut controller, 100, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Stopped)));
        assert!(controller.ready_to_stop());
        assert_eq!(controller.saved_revision(), controller.revision());
        controller.join_stopped().unwrap();
        assert_eq!(store.load().unwrap().state.note, "latest");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn timer_completion_emits_only_after_acknowledgement_is_saved() {
        let (dir, store) = test_store();
        let mut controller = Controller::start(&dir).unwrap();
        poll_until(&mut controller, 0, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Loaded { .. })));
        controller.state.set_note("Keep my prior work".into()).unwrap();
        let task_id = controller.state.add_task("Review the saved plan".into()).unwrap();
        controller.state.set_task_favorite(task_id, true);
        controller.state.presentation.pinned = true;
        let prior_note = controller.state.note.clone();
        let prior_tasks = controller.state.tasks.clone();
        let prior_presentation = controller.state.presentation.clone();
        controller.state.timer.start(0, 10).unwrap();
        controller.mark_changed(0).unwrap();
        assert!(controller.reconcile_timer(10).unwrap());
        let needed = controller.revision();
        poll_until(&mut controller, 10, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::TimerCompleted)));
        assert!(controller.saved_revision() >= needed);
        let saved = store.load().unwrap().state;
        assert!(matches!(saved.timer, Timer::Completed { acknowledged: true }));
        assert_eq!(saved.note, prior_note);
        assert_eq!(saved.tasks, prior_tasks);
        assert_eq!(saved.presentation, prior_presentation);
        assert_eq!(controller.state.note, prior_note);
        assert_eq!(controller.state.tasks, prior_tasks);
        assert_eq!(controller.state.presentation, prior_presentation);
        assert!(!controller.poll(11).iter().any(|event|
            matches!(event, ControllerEvent::TimerCompleted)));
        controller.request_stop().unwrap();
        poll_until(&mut controller, 11, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Stopped)));
        controller.join_stopped().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failed_save_cancels_stop_and_worker_accepts_repaired_revision() {
        let (dir, store) = test_store();
        let mut controller = Controller::start(&dir).unwrap();
        poll_until(&mut controller, 0, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Loaded { .. })));
        controller.state.note = "x".repeat(MAX_NOTE_BYTES + 1);
        controller.mark_changed(0).unwrap();
        poll_until(&mut controller, SAVE_DEBOUNCE_MS, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::SaveFailed { .. })));
        assert!(controller.request_stop().is_err());
        controller.state.set_note("repaired".into()).unwrap();
        let repaired = controller.mark_changed(SAVE_DEBOUNCE_MS).unwrap();
        poll_until(&mut controller, SAVE_DEBOUNCE_MS * 2, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Saved(revision) if *revision == repaired)));
        controller.request_stop().unwrap();
        poll_until(&mut controller, SAVE_DEBOUNCE_MS * 2, |events|
            events.iter().any(|event| matches!(event, ControllerEvent::Stopped)));
        controller.join_stopped().unwrap();
        assert_eq!(store.load().unwrap().state.note, "repaired");
        fs::remove_dir_all(dir).unwrap();
    }
}
