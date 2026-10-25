//! Cache publication and independent App resources across concurrent fixture users.
use super::prepared_inputs::{InputKey, PreparedInputs, cached_inputs};
use bevy::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, mpsc};
use std::time::Duration;

#[derive(Clone, Resource)]
struct InitializedDefinition {
    entry: u32,
    values: Vec<u32>,
}

fn create_inputs(entry: u32) -> PreparedInputs {
    let mut inputs = PreparedInputs::default();
    inputs.push(InitializedDefinition {
        entry,
        values: vec![entry],
    });
    inputs
}

struct GatedInitializer {
    calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl GatedInitializer {
    fn wait_then_initialize(&self, entry: u32) -> PreparedInputs {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv()
            .expect_err("dropping the publisher opens every blocked initializer");
        create_inputs(entry)
    }
}

#[test]
fn independent_keys_progress_while_one_key_initializes_once_for_concurrent_apps() {
    const READERS: usize = 8;
    const WAIT: Duration = Duration::from_secs(30);
    let first = InputKey {
        producer: "concurrent-publication-fixture",
        entry: 17306,
        map: 543,
        populate: false,
    };
    let other = InputKey {
        entry: 17308,
        ..first
    };
    std::thread::scope(|scope| {
        let (entered_tx, entered_rx) = mpsc::channel();
        // This sender is owned by the scope closure, so unwinding also opens the gate
        // before thread::scope waits for any spawned reader.
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let initializer = Arc::new(GatedInitializer {
            calls: AtomicUsize::new(0),
            entered: entered_tx,
            release: Mutex::new(release_rx),
        });
        let start = Arc::new(Barrier::new(READERS));
        let (ready_tx, ready_rx) = mpsc::channel();
        let readers: Vec<_> = (0..READERS)
            .map(|_| {
                let start = start.clone();
                let ready = ready_tx.clone();
                let initializer = initializer.clone();
                let load = move || initializer.wait_then_initialize(first.entry);
                scope.spawn(move || {
                    start.wait();
                    ready.send(()).unwrap();
                    cached_inputs(first, load)
                })
            })
            .collect();
        for _ in 0..READERS {
            ready_rx.recv_timeout(WAIT).unwrap();
        }
        entered_rx.recv_timeout(WAIT).unwrap();
        let other_calls = Arc::new(AtomicUsize::new(0));
        let other_initializations = other_calls.clone();
        let (finished_tx, finished_rx) = mpsc::channel();
        let other_reader = scope.spawn(move || {
            let inputs = cached_inputs(other, || {
                other_initializations.fetch_add(1, Ordering::SeqCst);
                create_inputs(other.entry)
            });
            finished_tx.send(()).unwrap();
            inputs
        });
        let independent_progress = finished_rx.recv_timeout(WAIT).is_ok();
        drop(release_tx);
        let inputs: Vec<_> = readers.into_iter().map(|r| r.join().unwrap()).collect();
        let other_inputs = other_reader.join().unwrap();
        assert_eq!(initializer.calls.load(Ordering::SeqCst), 1);
        assert_eq!(other_calls.load(Ordering::SeqCst), 1);
        let mut apps: Vec<_> = inputs
            .iter()
            .map(|inputs| {
                let mut app = App::new();
                inputs.install(&mut app);
                app
            })
            .collect();
        for app in &apps {
            let definition = app.world().resource::<InitializedDefinition>();
            assert_eq!(definition.entry, first.entry);
            assert_eq!(definition.values, [first.entry]);
        }
        apps[0]
            .world_mut()
            .resource_mut::<InitializedDefinition>()
            .values
            .clear();
        for app in &apps[1..] {
            assert_eq!(
                app.world().resource::<InitializedDefinition>().values,
                [first.entry]
            );
        }
        let mut other_app = App::new();
        other_inputs.install(&mut other_app);
        assert_eq!(
            other_app.world().resource::<InitializedDefinition>().values,
            [other.entry]
        );
        let warm = cached_inputs(first, || {
            panic!("a published key must not initialize again")
        });
        let mut warm_app = App::new();
        warm.install(&mut warm_app);
        assert_eq!(
            warm_app.world().resource::<InitializedDefinition>().values,
            [first.entry]
        );
        assert_eq!(initializer.calls.load(Ordering::SeqCst), 1);
        assert!(
            independent_progress,
            "an unrelated key must finish while the first initializer is still gated"
        );
    });
}
