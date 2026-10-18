//! Owned navigation queries off the simulation thread; no synchronous wait API.
use crate::NavigationMap;
use glam::Vec3;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::JoinHandle;

#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    Path {
        from: Vec3,
        to: Vec3,
    },
    SmoothPath {
        from: Vec3,
        to: Vec3,
    },
    Collision {
        from: Vec3,
        to: Vec3,
    },
    Candidates {
        from: Vec3,
        targets: Vec<Vec3>,
    },
    #[cfg(test)]
    Panic,
}

#[derive(Debug)]
pub enum Output {
    Path(Vec<Vec3>),
    Collision(Vec3),
    Candidates(Vec<Result<Vec<Vec3>, String>>),
}

#[derive(Clone, Debug)]
pub struct Request {
    pub entity: u64,
    pub id: u64,
    pub map_id: u32,
    pub query: Query,
}

#[derive(Debug)]
pub struct Response {
    pub entity: u64,
    pub id: u64,
    pub result: Result<Option<Output>, String>,
}

#[derive(Default)]
struct Mailbox {
    queued: BTreeMap<(u64, u64), Request>,
    entities: BTreeMap<u64, u64>,
    ready: BTreeMap<u64, Response>,
    failure: Option<String>,
}

impl Mailbox {
    fn queue(&mut self, request: Request) {
        self.cancel(request.entity);
        self.entities.insert(request.entity, request.id);
        self.queued.insert((request.id, request.entity), request);
    }

    fn cancel(&mut self, entity: u64) {
        if let Some(id) = self.entities.remove(&entity) {
            self.queued.remove(&(id, entity));
        }
        self.ready.remove(&entity);
    }

    fn apply_updates(&mut self, outgoing: BTreeMap<u64, Option<Request>>) {
        for (entity, request) in outgoing {
            match request {
                Some(request) => self.queue(request),
                None => self.cancel(entity),
            }
        }
    }

    fn pop(&mut self) -> Option<Request> {
        let (_, request) = self.queued.pop_first()?;
        self.entities.remove(&request.entity);
        Some(request)
    }

    fn publish(&mut self, response: Response) {
        if self
            .entities
            .get(&response.entity)
            .is_some_and(|id| *id > response.id)
        {
            return;
        }
        self.ready.insert(response.entity, response);
    }
}

/// Main-thread handle. Outgoing updates and retained completions coalesce by entity.
/// The wake channel contains at most one notification, never a queue of jobs.
pub struct Worker {
    mailbox: Arc<Mutex<Mailbox>>,
    wake: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
    outgoing: BTreeMap<u64, Option<Request>>,
    current: BTreeMap<u64, u64>,
    pending: BTreeMap<u64, u64>,
    failure: Option<String>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    pub fn new(root: PathBuf) -> Self {
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let (wake, receiver) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let spawned = spawn_worker(root, mailbox.clone(), receiver, stop.clone());
        let (thread, failure) = match spawned {
            Ok(thread) => (Some(thread), None),
            Err(error) => {
                let message = format!("navigation worker could not start: {error}");
                eprintln!("{message}");
                (None, Some(message))
            }
        };
        Self {
            mailbox,
            wake,
            thread,
            outgoing: BTreeMap::new(),
            current: BTreeMap::new(),
            pending: BTreeMap::new(),
            failure,
            stop,
        }
    }

    pub fn submit(&mut self, request: Request) {
        if self
            .current
            .get(&request.entity)
            .is_some_and(|id| *id >= request.id)
        {
            return;
        }
        self.current.insert(request.entity, request.id);
        self.pending.insert(request.entity, request.id);
        self.outgoing.insert(request.entity, Some(request));
        self.dispatch();
    }

    pub fn cancel(&mut self, entity: u64) {
        self.current.remove(&entity);
        self.pending.remove(&entity);
        self.outgoing.insert(entity, None);
        self.dispatch();
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty()
    }

    /// Take one completion set without waiting for a worker or a held mailbox.
    pub fn poll(&mut self) -> Vec<Response> {
        if self.failure.is_none() {
            let responses = self.exchange();
            if self.failure.is_none() {
                return responses;
            }
        }
        self.fail_pending()
    }

    fn exchange(&mut self) -> Vec<Response> {
        let responses = match self.take_mailbox_results() {
            Ok(Some(responses)) => responses,
            Ok(None) => return Vec::new(),
            Err(error) => {
                self.failure = Some(error);
                return Vec::new();
            }
        };
        if self.wake_worker().is_err() {
            // The supervisor publishes the cause before dropping the receiver.
            // Keep pending IDs intact so the next poll surfaces that exact cause.
            return Vec::new();
        }
        self.accept_current(responses)
    }

    fn take_mailbox_results(&mut self) -> Result<Option<Vec<Response>>, String> {
        let mut mailbox = match self.mailbox.try_lock() {
            Ok(mailbox) => mailbox,
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Poisoned(_)) => {
                return Err("navigation worker mailbox poisoned".into());
            }
        };
        if let Some(error) = &mailbox.failure {
            return Err(error.clone());
        }
        if self
            .thread
            .as_ref()
            .is_none_or(|thread| thread.is_finished())
        {
            return Err("navigation worker exited unexpectedly".into());
        }
        mailbox.apply_updates(std::mem::take(&mut self.outgoing));
        Ok(Some(
            std::mem::take(&mut mailbox.ready).into_values().collect(),
        ))
    }

    fn dispatch(&mut self) {
        if self.failure.is_some() {
            return;
        }
        match self.mailbox.try_lock() {
            Ok(mut mailbox) => mailbox.apply_updates(std::mem::take(&mut self.outgoing)),
            Err(TryLockError::WouldBlock) => return, // Latest intent remains coalesced locally.
            Err(TryLockError::Poisoned(_)) => {
                self.failure = Some("navigation worker mailbox poisoned".into());
                return;
            }
        }
        if self.wake_worker().is_err() {
            // poll() handles the published supervisor cause or finished-thread error.
            return;
        }
    }

    fn wake_worker(&self) -> Result<(), String> {
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()), // A queued wake already covers current jobs.
            Err(TrySendError::Disconnected(())) => Err("navigation worker disconnected".into()),
        }
    }

    fn accept_current(&mut self, responses: Vec<Response>) -> Vec<Response> {
        let mut current: Vec<_> = responses
            .into_iter()
            .filter(|response| {
                self.current.get(&response.entity) == Some(&response.id)
                    && self.pending.get(&response.entity) == Some(&response.id)
            })
            .collect();
        for response in &current {
            self.pending.remove(&response.entity);
        }
        current.sort_by_key(|response| (response.entity, response.id));
        current
    }

    fn fail_pending(&mut self) -> Vec<Response> {
        let error = self
            .failure
            .as_ref()
            .expect("worker failure recorded")
            .clone();
        self.outgoing.clear();
        std::mem::take(&mut self.pending)
            .into_iter()
            .map(|(entity, id)| Response {
                entity,
                id,
                result: Err(error.clone()),
            })
            .collect()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Dropping the sole wake sender wakes an idle receiver; never join on a tick.
    }
}

fn spawn_worker(
    root: PathBuf,
    mailbox: Arc<Mutex<Mailbox>>,
    receiver: Receiver<()>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("navigation".into())
        .spawn(move || {
            supervise_worker(root, mailbox, receiver, stop);
        })
}

fn supervise_worker(
    root: PathBuf,
    mailbox: Arc<Mutex<Mailbox>>,
    receiver: Receiver<()>,
    stop: Arc<AtomicBool>,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Publish the panic reason before dropping the receiver; disconnection
        // must not race ahead of the explicit worker failure.
        run_worker(&root, &mailbox, &receiver, &stop);
    }));
    if let Err(panic) = result {
        let detail = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
            .unwrap_or_else(|| "non-string panic".into());
        let error = format!("navigation worker panicked: {detail}");
        match mailbox.lock() {
            Ok(mut mailbox) => mailbox.failure = Some(error.clone()),
            Err(poisoned) => poisoned.into_inner().failure = Some(error.clone()),
        }
        eprintln!("{error}");
    }
}

fn run_worker(root: &Path, mailbox: &Mutex<Mailbox>, receiver: &Receiver<()>, stop: &AtomicBool) {
    let mut maps = BTreeMap::new();
    while receiver.recv().is_ok() {
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let request = mailbox.lock().expect("navigation queue lock").pop();
            let Some(request) = request else { break };
            let result = query_owned_map(root, &mut maps, request.map_id, request.query);
            let response = Response {
                entity: request.entity,
                id: request.id,
                result,
            };
            mailbox
                .lock()
                .expect("navigation result lock")
                .publish(response);
        }
    }
}

fn query_owned_map(
    root: &Path,
    maps: &mut BTreeMap<u32, Option<NavigationMap>>,
    map_id: u32,
    query: Query,
) -> Result<Option<Output>, String> {
    #[cfg(test)]
    if matches!(query, Query::Panic) {
        panic!("controlled navigation worker death")
    }
    if !maps.contains_key(&map_id) {
        maps.insert(map_id, open_baked_map(&root.join(map_id.to_string()))?);
    }
    let Some(map) = maps.get_mut(&map_id).expect("owned map inserted") else {
        return Ok(None);
    };
    run_query(map, query).map(Some)
}

fn run_query(map: &mut NavigationMap, query: Query) -> Result<Output, String> {
    let output = match query {
        Query::Path { from, to } => Output::Path(map.path(from, to)?),
        Query::SmoothPath { from, to } => Output::Path(map.smooth_waypoint_path(from, to)?),
        Query::Collision { from, to } => {
            map.path(from, from)?;
            Output::Collision(map.first_collision(from, to)?)
        }
        Query::Candidates { from, targets } => {
            Output::Candidates(targets.into_iter().map(|to| map.path(from, to)).collect())
        }
        #[cfg(test)]
        Query::Panic => unreachable!("panic control handled before map loading"),
    };
    Ok(output)
}

fn open_baked_map(dir: &Path) -> Result<Option<NavigationMap>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", dir.display())),
    };
    let mut baked = false;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", dir.display()))?;
        baked |= entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "nav");
    }
    if baked {
        NavigationMap::open(dir).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
