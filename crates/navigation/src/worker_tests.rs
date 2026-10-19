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
    let deadline = Instant::now() + Duration::from_secs(5);
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
    assert!(path.last().unwrap().distance(to) < 0.5);
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
    assert!(path.last().unwrap().distance(to) < 0.5);
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
