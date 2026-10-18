use super::*;
use bevy::prelude::{App, Resource};
use std::sync::mpsc;

/// Hang guard only: a correct supervisor reports long before this.
const VERDICT: Duration = Duration::from_secs(20);
const LONG: Duration = Duration::from_secs(60);

#[derive(Resource)]
struct Presence {
    _live: Live,
}

fn entered_app(count: &Arc<AtomicUsize>) -> App {
    let mut app = App::new();
    app.insert_resource(Presence {
        _live: Live::enter(count),
    });
    app
}

fn verdict<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> mpsc::Receiver<T> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
}

#[test]
fn live_presence_ends_when_app_exits_normally() {
    let count = Arc::new(AtomicUsize::new(0));
    let peer = count.clone();
    let worker = std::thread::spawn(move || entered_app(&peer).run().is_success());
    assert!(worker.join().expect("peer thread"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

fn ramp_with(
    peers: usize,
    body: fn(usize, &Arc<AtomicUsize>, &Sender<Event>) -> bool,
) -> (Result<usize, String>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let spawned = Arc::new(AtomicUsize::new(0));
    let (events, receiver) = mpsc::channel();
    let (peer_count, peer_spawned) = (count.clone(), spawned.clone());
    let result = verdict(move || {
        ramp(peers, LONG, LONG, &receiver, |index| {
            peer_spawned.fetch_add(1, Ordering::SeqCst);
            let (count, sender) = (peer_count.clone(), events.clone());
            spawn_peer(index, events.clone(), move || body(index, &count, &sender))
        })
        .map(|workers| workers.len())
    })
    .recv_timeout(VERDICT)
    .expect("ramp kept waiting after peer death");
    (result, spawned)
}

#[test]
fn ramp_fails_when_entered_peer_dies_during_gap() {
    let (result, spawned) = ramp_with(3, |index, count, events| {
        let mut app = entered_app(count);
        events.send(Event::Entered(index)).expect("supervisor");
        app.update();
        false
    });
    assert!(result.unwrap_err().contains("peer 1"));
    assert_eq!(spawned.load(Ordering::SeqCst), 1);
}

#[test]
fn ramp_fails_when_peer_dies_before_entry() {
    let (result, spawned) = ramp_with(3, |_, _, _| false);
    assert!(result.unwrap_err().contains("peer 1"));
    assert_eq!(spawned.load(Ordering::SeqCst), 1);
}

#[test]
fn all_entered_requires_every_peer_live() {
    let count = Arc::new(AtomicUsize::new(0));
    let (events, _receiver) = mpsc::channel();
    let (exited, exits) = mpsc::channel();
    let mut releases = Vec::new();
    let mut workers = Vec::new();
    for index in 1..=11 {
        let (release, hold) = mpsc::channel::<()>();
        releases.push(release);
        let (count, exited) = (count.clone(), exited.clone());
        workers.push(spawn_peer(index, events.clone(), move || {
            let mut app = entered_app(&count);
            app.update();
            if index <= 5 {
                let _ = hold.recv();
            }
            drop(app);
            exited.send(index).expect("test");
            true
        }));
    }
    for _ in 6..=11 {
        exits.recv_timeout(VERDICT).expect("normal peer exit");
    }
    let verdict = confirm_all_entered(&count, &workers, 11);
    drop(releases);
    for worker in workers {
        assert!(worker.join().expect("peer thread"));
    }
    assert!(verdict.is_err(), "ALL_ENTERED with 5 live peers");
}

#[test]
fn measurement_reports_later_peer_failure_while_earlier_peer_runs() {
    let count = Arc::new(AtomicUsize::new(0));
    let (events, receiver) = mpsc::channel();
    let (release, hold) = mpsc::channel::<()>();
    let first_count = count.clone();
    let first = spawn_peer(1, events.clone(), move || {
        let mut app = entered_app(&first_count);
        app.update();
        let _ = hold.recv();
        true
    });
    let second = spawn_peer(2, events.clone(), move || {
        entered_app(&count).update();
        false
    });
    let result = verdict(move || measure(vec![first, second], &receiver)).recv_timeout(VERDICT);
    drop(release);
    assert_eq!(
        result.expect("measurement waited on running peer 1"),
        Err("peer 2 failed".to_string())
    );
}
