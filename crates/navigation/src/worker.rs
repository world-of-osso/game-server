//! Owned navigation queries off the simulation thread; no synchronous wait API.
use crate::{NavigationMap, NavigationPath, NavigationPlan};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};
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
    /// `Path` that walks PATHFIND_INCOMPLETE corridors (FollowMovementGenerator).
    FollowPath {
        from: Vec3,
        to: Vec3,
    },
    /// Resolve all walking segments, or report an incomplete route and its reason.
    CompleteRoute {
        from: Vec3,
        to: Vec3,
    },
    /// Follow a formation point projected onto the owner's connected surface.
    FollowFormation {
        from: Vec3,
        to: Vec3,
        owner: Vec3,
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

#[derive(Clone, Debug)]
pub enum Output {
    Path(NavigationPath),
    CompleteRoute(crate::CompleteRoute),
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
    cancelled: BTreeSet<u64>,
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
                None => {
                    self.cancel(entity);
                    self.cancelled.insert(entity);
                }
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

/// Main-thread handle. Each map has a worker so a cold tile read cannot block another map.
pub struct Worker {
    lanes: BTreeMap<u32, WorkerLane>,
    unbaked: WorkerLane,
    owners: BTreeMap<u64, u32>,
    current: BTreeMap<u64, u64>,
    pending: BTreeMap<u64, u64>,
    failure: Option<String>,
}

impl Worker {
    pub fn new(root: PathBuf) -> Self {
        let (lanes, failure) = match preload_maps(&root) {
            Ok(maps) => (
                maps.into_iter()
                    .map(|(id, map)| (id, WorkerLane::new(BTreeMap::from([(id, map)]))))
                    .collect(),
                None,
            ),
            Err(error) => (
                BTreeMap::new(),
                Some(format!("navigation worker could not start: {error}")),
            ),
        };
        Self {
            lanes,
            unbaked: WorkerLane::new(BTreeMap::new()),
            owners: BTreeMap::new(),
            current: BTreeMap::new(),
            pending: BTreeMap::new(),
            failure,
        }
    }

    fn lane(&mut self, map_id: u32) -> &mut WorkerLane {
        self.lanes.get_mut(&map_id).unwrap_or(&mut self.unbaked)
    }

    pub fn submit(&mut self, request: Request) {
        if self
            .current
            .get(&request.entity)
            .is_some_and(|id| *id >= request.id)
        {
            return;
        }
        if let Some(old_map) = self.owners.get(&request.entity).copied() {
            if old_map != request.map_id {
                self.lane(old_map).cancel(request.entity);
            }
        }
        self.owners.insert(request.entity, request.map_id);
        self.current.insert(request.entity, request.id);
        self.pending.insert(request.entity, request.id);
        self.lane(request.map_id).submit(request);
    }

    pub fn cancel(&mut self, entity: u64) {
        if let Some(map_id) = self.owners.remove(&entity) {
            self.lane(map_id).cancel(entity);
        }
        self.current.remove(&entity);
        self.pending.remove(&entity);
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn poll(&mut self) -> Vec<Response> {
        if let Some(error) = &self.failure {
            return std::mem::take(&mut self.pending)
                .into_iter()
                .map(|(entity, id)| Response {
                    entity,
                    id,
                    result: Err(error.clone()),
                })
                .collect();
        }
        let mut responses = Vec::new();
        for lane in self
            .lanes
            .values_mut()
            .chain(std::iter::once(&mut self.unbaked))
        {
            if !lane.is_idle() || !lane.outgoing.is_empty() {
                responses.extend(lane.poll());
            }
        }
        responses.retain(|response| {
            self.current.get(&response.entity) == Some(&response.id)
                && self.pending.get(&response.entity) == Some(&response.id)
        });
        for response in &responses {
            self.pending.remove(&response.entity);
        }
        responses.sort_by_key(|response| (response.entity, response.id));
        responses
    }
}

/// One map's bounded queue and exclusively owned navigation state.
struct WorkerLane {
    mailbox: Arc<Mutex<Mailbox>>,
    wake: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
    outgoing: BTreeMap<u64, Option<Request>>,
    current: BTreeMap<u64, u64>,
    pending: BTreeMap<u64, u64>,
    failure: Option<String>,
    stop: Arc<AtomicBool>,
}

impl WorkerLane {
    fn new(maps: MapCache) -> Self {
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let (wake, receiver) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let spawned = spawn_worker(maps, mailbox.clone(), receiver, stop.clone());
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

impl Drop for WorkerLane {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Dropping the sole wake sender wakes an idle receiver; never join on a tick.
    }
}

type MapCache = BTreeMap<u32, Result<Option<NavigationMap>, String>>;

fn preload_maps(root: &Path) -> Result<MapCache, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(format!("{}: {error}", root.display())),
    };
    let mut maps = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", root.display()))?;
        let Some(id) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let result = open_baked_map(&entry.path()).map(|map| {
            map.map(|mut map| {
                map.preload_hierarchy();
                map
            })
        });
        maps.insert(id, result);
    }
    Ok(maps)
}

fn spawn_worker(
    maps: MapCache,
    mailbox: Arc<Mutex<Mailbox>>,
    receiver: Receiver<()>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("navigation".into())
        .spawn(move || {
            supervise_worker(maps, mailbox, receiver, stop);
        })
}

fn supervise_worker(
    maps: MapCache,
    mailbox: Arc<Mutex<Mailbox>>,
    receiver: Receiver<()>,
    stop: Arc<AtomicBool>,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Publish the panic reason before dropping the receiver; disconnection
        // must not race ahead of the explicit worker failure.
        run_worker(maps, &mailbox, &receiver, &stop);
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

fn run_worker(
    mut maps: MapCache,
    mailbox: &Mutex<Mailbox>,
    receiver: &Receiver<()>,
    stop: &AtomicBool,
) {
    let mut plans: BTreeMap<u64, (u32, NavigationPlan)> = BTreeMap::new();
    while receiver.recv().is_ok() {
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let request = {
                let mut mailbox = mailbox.lock().expect("navigation queue lock");
                for entity in std::mem::take(&mut mailbox.cancelled) {
                    plans.remove(&entity);
                }
                mailbox.pop()
            };
            let Some(request) = request else { break };
            let entry = plans
                .entry(request.entity)
                .or_insert_with(|| (request.map_id, NavigationPlan::default()));
            if entry.0 != request.map_id {
                *entry = (request.map_id, NavigationPlan::default());
            }
            let result = query_owned_map(&mut maps, request.map_id, request.query, &mut entry.1);
            if result.is_err() || entry.1.is_empty() {
                plans.remove(&request.entity);
            }
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
    maps: &mut MapCache,
    map_id: u32,
    query: Query,
    plan: &mut NavigationPlan,
) -> Result<Option<Output>, String> {
    #[cfg(test)]
    if matches!(query, Query::Panic) {
        panic!("controlled navigation worker death")
    }
    let Some(map) = maps.get_mut(&map_id) else {
        return Ok(None);
    };
    let Some(map) = map.as_mut().map_err(|error| error.clone())? else {
        return Ok(None);
    };
    run_query(map, query, plan).map(Some)
}

fn run_query(
    map: &mut NavigationMap,
    query: Query,
    plan: &mut NavigationPlan,
) -> Result<Output, String> {
    let output = match query {
        Query::Path { from, to } => Output::Path(map.path_with_plan(from, to, plan)?),
        Query::FollowPath { from, to } => Output::Path(map.follow_path(from, to, plan)?),
        Query::CompleteRoute { from, to } => {
            Output::CompleteRoute(crate::complete_route::follow_complete_route(map, from, to))
        }
        Query::FollowFormation { from, to, owner } => {
            Output::Path(map.follow_formation_path(from, to, owner, plan)?)
        }
        Query::SmoothPath { from, to } => Output::Path(NavigationPath {
            points: map.smooth_waypoint_path(from, to)?,
            complete: true,
            collision_drop: None,
        }),
        Query::Collision { from, to } => {
            map.path(from, from)?;
            Output::Collision(map.first_collision(from, to)?)
        }
        Query::Candidates { from, targets } => Output::Candidates(
            targets
                .into_iter()
                .map(|to| map.complete_walk_path(from, to))
                .collect(),
        ),
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
