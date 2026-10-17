//! Temporary pure seed search; its result must be reproduced with the real fixture.
use super::*;

pub(super) fn find_counterexample_seed() -> u64 {
    for seed in 0..10_000 {
        let mut events = EventMap::seeded(seed);
        events.schedule_range(18670, 5000, 10000);
        events.schedule_range(5568, 10000, 15000);
        let mut casting_until = 0;
        let mut knock_ready = 0;
        let mut times = Vec::new();
        let mut blocked = Vec::new();
        for tick in 1..=1900 {
            let now = tick * 50;
            if tick == 401 {
                events.schedule(21808, 0);
            }
            blocked.push(((tick - 1) as f32 * 0.05, casting_until >= now));
            events.update(50);
            if casting_until >= now {
                continue;
            }
            while let Some(event) = events.execute_event() {
                let repeat = match event {
                    18670 => {
                        if now >= knock_ready {
                            times.push(tick as f32 * 0.05);
                            casting_until = now + 1000;
                            knock_ready = now + 12000;
                        }
                        (15000, 25000)
                    }
                    5568 => (10000, 20000),
                    21808 => {
                        casting_until = if tick == 401 { 24550 } else { now + 8000 };
                        (30000, 40000)
                    }
                    _ => unreachable!(),
                };
                events.repeat_from_due_range(repeat.0, repeat.1);
                if casting_until > now {
                    break;
                }
            }
        }
        if !super::celebras::follows_task_deadlines(&times, (5.0, 10.0), (15.0, 25.0), &blocked) {
            eprintln!("LANDSLIDE PURE CANDIDATE seed={seed} times={times:?}");
            return seed;
        }
    }
    panic!("No pure clock counterexample in bounded seed search");
}
