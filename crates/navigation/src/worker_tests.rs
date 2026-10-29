use super::*;
use std::time::{Duration, Instant};

fn request(entity: u64, id: u64, map_id: u32) -> Request {
    Request {
        entity,
        id,
        map_id,
        query: Query::Path {
            from: Vec3::new(100.0, 0.2, 100.0),
            to: Vec3::new(120.0, 0.2, 100.0),
        },
    }
}

fn drain(worker: &mut Worker) -> Vec<Response> {
    drain_with_timeout(worker, Duration::from_secs(5))
}

fn drain_with_timeout(worker: &mut Worker, timeout: Duration) -> Vec<Response> {
    let deadline = Instant::now() + timeout;
    let mut responses = Vec::new();
    while !worker.is_idle() {
        let batch = worker.poll();
        assert!(
            batch
                .windows(2)
                .all(|pair| { (pair[0].entity, pair[0].id) < (pair[1].entity, pair[1].id) }),
            "completion set must publish in stable entity/id order"
        );
        responses.extend(batch);
        assert!(
            Instant::now() < deadline,
            "navigation requests remained pending"
        );
        std::thread::yield_now();
    }
    responses
}

#[cfg(unix)]
#[test]
fn held_tile_on_one_map_does_not_block_another_map() {
    use std::io::Write;
    use std::sync::mpsc;
    let temp = tempfile::tempdir().unwrap();
    let map = temp.path().join("12");
    std::fs::create_dir(&map).unwrap();
    let tile = map.join("32_31.nav");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&tile)
            .status()
            .unwrap()
            .success()
    );
    let (opened_tx, opened_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let mut pipe = std::fs::OpenOptions::new().write(true).open(tile).unwrap();
        opened_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        pipe.write_all(b"invalid tile").unwrap();
    });
    let mut worker = Worker::new(temp.path().to_owned());
    worker.submit(request(1, 1, 12));
    opened_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    worker.submit(request(2, 1, 99));
    let deadline = Instant::now() + Duration::from_secs(5);
    let response = loop {
        if let Some(response) = worker
            .poll()
            .into_iter()
            .find(|response| response.entity == 2)
        {
            break response;
        }
        assert!(
            Instant::now() < deadline,
            "unrelated map was blocked by tile read"
        );
        std::thread::yield_now();
    };
    assert!(matches!(response.result, Ok(None)));
    release_tx.send(()).unwrap();
    let response = drain(&mut worker).pop().unwrap();
    assert!(response.result.is_err());
    writer.join().unwrap();
}

#[test]
#[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2519 assets"]
fn worker_loads_graph_before_the_first_long_request() {
    use std::os::unix::fs::symlink;
    let source =
        std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap()).join("2519");
    let temp = tempfile::tempdir().unwrap();
    let map = temp.path().join("2519");
    std::fs::create_dir(&map).unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "nav") {
            symlink(&path, map.join(path.file_name().unwrap())).unwrap();
        }
    }
    let graph = map.join("portal-graph.json");
    std::fs::copy(source.join("portal-graph.json"), &graph).unwrap();
    let mut worker = Worker::new(temp.path().to_owned());
    std::fs::remove_file(graph).unwrap();
    worker.submit(Request {
        entity: 1,
        id: 1,
        map_id: 2519,
        query: Query::Path {
            from: Vec3::new(-257.87, 123.38, 489.73),
            to: Vec3::new(-211.1, 99.34897, 932.7),
        },
    });
    let response = drain_with_timeout(&mut worker, Duration::from_secs(60))
        .pop()
        .unwrap();
    assert!(matches!(response.result, Ok(Some(Output::Path(_)))));
}

#[test]
#[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2516 assets"]
fn worker_keeps_long_route_plans_separate_across_movers_and_cancel() {
    let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let mut worker = Worker::new(root);
    let start = Vec3::new(-559.8952, 340.06406, -1631.8356);
    let end = Vec3::new(-1593.5763, 467.57016, -2037.4996);
    let mut positions = [start, start];
    let mut completed = [false, false];
    let mut seen = [
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
    ];
    for step in 1..=100 {
        for index in 0..2 {
            if !completed[index] {
                worker.submit(Request {
                    entity: index as u64 + 1,
                    id: step,
                    map_id: 2516,
                    query: Query::Path {
                        from: positions[index],
                        to: end,
                    },
                });
            }
        }
        for response in drain_with_timeout(&mut worker, Duration::from_secs(60)) {
            let index = response.entity as usize - 1;
            let Some(Output::Path(path)) = response.result.unwrap() else {
                panic!("expected path")
            };
            let endpoint = *path.points.last().expect("empty leg");
            assert!(
                seen[index].insert(endpoint.to_array().map(f32::to_bits)),
                "mover repeated an endpoint"
            );
            positions[index] = endpoint;
            completed[index] = path.complete;
        }
        if step == 1 {
            worker.cancel(2);
        }
        if completed.iter().all(|value| *value) {
            return;
        }
    }
    panic!("movers did not finish their routes");
}

#[test]
fn worker_unbaked_result_remains_distinct_from_pending() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = Worker::new(dir.path().to_owned());
    worker.submit(request(41, 1, 12));
    assert!(!worker.is_idle());
    let responses = drain(&mut worker);
    assert_eq!(responses.len(), 1);
    assert_eq!((responses[0].entity, responses[0].id), (41, 1));
    assert!(matches!(responses[0].result, Ok(None)));
}

#[test]
fn worker_coalesces_many_updates_to_one_latest_request_per_entity() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = Worker::new(dir.path().to_owned());
    for ordinal in 1..=1000 {
        for entity in 41..=45 {
            worker.submit(request(entity, ordinal * 10 + entity - 40, 12));
        }
    }
    let mut responses = drain(&mut worker);
    responses.sort_by_key(|response| response.entity);
    assert_eq!(responses.len(), 5);
    for response in responses {
        assert_eq!(response.id, 10000 + response.entity - 40);
        assert!(matches!(response.result, Ok(None)));
    }
}

#[test]
fn worker_cancelled_request_never_publishes_and_new_generation_can_run() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = Worker::new(dir.path().to_owned());
    worker.submit(request(41, 1, 12));
    worker.cancel(41);
    assert!(worker.poll().is_empty());
    assert!(worker.is_idle());
    worker.submit(request(41, 2, 12));
    let responses = drain(&mut worker);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].id, 2);
}

#[test]
fn worker_preserves_the_same_corrupt_tile_error_as_direct_query() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("12");
    std::fs::create_dir(&map_dir).unwrap();
    std::fs::write(map_dir.join("32_31.nav"), b"truncated navigation").unwrap();
    let mut map = NavigationMap::open(&map_dir).unwrap();
    let error = map
        .path(Vec3::new(100.0, 0.2, 100.0), Vec3::new(120.0, 0.2, 100.0))
        .unwrap_err();
    let mut worker = Worker::new(dir.path().to_owned());
    worker.submit(request(41, 1, 12));
    let response = drain(&mut worker).pop().unwrap();
    assert_eq!(response.result.unwrap_err(), error);
}

#[test]
fn worker_panic_fails_every_pending_and_subsequent_request_explicitly() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = Worker::new(dir.path().to_owned());
    worker.submit(Request {
        entity: 41,
        id: 1,
        map_id: 12,
        query: Query::Panic,
    });
    worker.submit(request(42, 2, 12));
    worker.submit(request(43, 3, 12));
    let responses = drain(&mut worker);
    assert_eq!(responses.len(), 3);
    for response in responses {
        assert!(
            response
                .result
                .unwrap_err()
                .contains("navigation worker panicked: controlled navigation worker death")
        );
    }
    worker.submit(request(44, 4, 12));
    let response = drain(&mut worker).pop().unwrap();
    assert_eq!((response.entity, response.id), (44, 4));
    assert!(
        response
            .result
            .unwrap_err()
            .contains("navigation worker panicked")
    );
}

#[cfg(feature = "bake")]
#[test]
fn worker_returns_real_path_collision_and_candidate_batch() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("12");
    std::fs::create_dir(&map_dir).unwrap();
    let a = Vec3::new(10.0, 0.0, 10.0);
    let b = Vec3::new(10.0, 0.0, 190.0);
    let c = Vec3::new(190.0, 0.0, 190.0);
    let d = Vec3::new(190.0, 0.0, 10.0);
    let tile = crate::bake::bake_tile(&[[a, b, c], [a, c, d]], (32, 31)).unwrap();
    std::fs::write(map_dir.join("32_31.nav"), tile).unwrap();
    let from = Vec3::new(100.0, 0.2, 100.0);
    let to = Vec3::new(120.0, 0.2, 100.0);
    let mut worker = Worker::new(dir.path().to_owned());
    worker.submit(request(41, 1, 12));
    let response = drain(&mut worker).pop().unwrap();
    let Some(Output::Path(path)) = response.result.unwrap() else {
        panic!("expected real baked path")
    };
    assert!(path.complete);
    assert!(path.points.last().unwrap().distance(to) < 0.5);
    worker.submit(Request {
        entity: 41,
        id: 2,
        map_id: 12,
        query: Query::SmoothPath { from, to },
    });
    let response = drain(&mut worker).pop().unwrap();
    let Some(Output::Path(path)) = response.result.unwrap() else {
        panic!("expected smooth baked path")
    };
    assert!(path.complete);
    assert!(path.points.last().unwrap().distance(to) < 0.5);
    worker.submit(Request {
        entity: 41,
        id: 3,
        map_id: 12,
        query: Query::Collision { from, to },
    });
    let response = drain(&mut worker).pop().unwrap();
    let Some(Output::Collision(end)) = response.result.unwrap() else {
        panic!("expected surface collision result")
    };
    assert!(end.distance(to) < 0.5);
    worker.submit(Request {
        entity: 41,
        id: 4,
        map_id: 12,
        query: Query::Candidates {
            from,
            targets: vec![to, from],
        },
    });
    let response = drain(&mut worker).pop().unwrap();
    let Some(Output::Candidates(paths)) = response.result.unwrap() else {
        panic!("expected candidate batch")
    };
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(Result::is_ok));
}
