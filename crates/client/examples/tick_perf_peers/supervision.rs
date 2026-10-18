//! Peer lifetime supervision: live presence, ramp waits, and out-of-order measurement joins.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc::{Receiver, RecvTimeoutError, Sender},
};
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(test)]
#[path = "supervision_tests.rs"]
mod tests;

pub enum Event {
    Entered(usize),
    Exited(usize),
}

/// Counts one entered peer for as long as its App owns this value.
pub struct Live(Arc<AtomicUsize>);

impl Live {
    pub fn enter(count: &Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self(count.clone())
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Reports a worker's exit, including panic unwinds, to the supervisor.
struct ExitNotice(usize, Sender<Event>);

impl Drop for ExitNotice {
    fn drop(&mut self) {
        let _ = self.1.send(Event::Exited(self.0));
    }
}

pub fn spawn_peer(
    index: usize,
    events: Sender<Event>,
    body: impl FnOnce() -> bool + Send + 'static,
) -> JoinHandle<bool> {
    std::thread::spawn(move || {
        let _exit = ExitNotice(index, events);
        body()
    })
}

pub fn ramp(
    peers: usize,
    gap: Duration,
    entry_timeout: Duration,
    events: &Receiver<Event>,
    mut spawn: impl FnMut(usize) -> JoinHandle<bool>,
) -> Result<Vec<JoinHandle<bool>>, String> {
    let mut workers = Vec::new();
    for index in 1..=peers {
        workers.push(spawn(index));
        wait(events, entry_timeout, Some(index))?;
        if index < peers {
            wait(events, gap, None)?;
        }
    }
    Ok(workers)
}

/// Waits for `entry` (or the whole `limit` when None); any peer exit fails the ramp.
fn wait(events: &Receiver<Event>, limit: Duration, entry: Option<usize>) -> Result<(), String> {
    match events.recv_timeout(limit) {
        Ok(Event::Entered(index)) if Some(index) == entry => Ok(()),
        Ok(Event::Entered(index)) => Err(format!("unexpected entry from peer {index}")),
        Ok(Event::Exited(index)) => Err(format!("peer {index} exited during ramp")),
        Err(RecvTimeoutError::Timeout) => match entry {
            None => Ok(()),
            Some(index) => Err(format!("peer {index} entry timed out")),
        },
        Err(RecvTimeoutError::Disconnected) => Err("peer events disconnected".into()),
    }
}

pub fn confirm_all_entered(
    live: &AtomicUsize,
    workers: &[JoinHandle<bool>],
    peers: usize,
) -> Result<(), String> {
    let live = live.load(Ordering::SeqCst);
    let running = workers
        .iter()
        .filter(|worker| !worker.is_finished())
        .count();
    if live == peers && running == peers {
        Ok(())
    } else {
        Err(format!(
            "not all entered: live={live} running={running} expected={peers}"
        ))
    }
}

/// Joins workers in exit order so any peer's failure surfaces immediately.
pub fn measure(workers: Vec<JoinHandle<bool>>, events: &Receiver<Event>) -> Result<(), String> {
    let mut workers: Vec<_> = workers.into_iter().map(Some).collect();
    for _ in 0..workers.len() {
        let index = match events.recv() {
            Ok(Event::Exited(index)) => index,
            Ok(Event::Entered(index)) => return Err(format!("unexpected entry from peer {index}")),
            Err(_) => return Err("peer events disconnected".into()),
        };
        let worker = workers[index - 1]
            .take()
            .ok_or_else(|| format!("peer {index} exited twice"))?;
        match worker.join() {
            Ok(true) => {}
            Ok(false) => return Err(format!("peer {index} failed")),
            Err(_) => return Err(format!("peer {index} panicked")),
        }
    }
    Ok(())
}
