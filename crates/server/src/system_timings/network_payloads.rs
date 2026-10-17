//! Read-only transport payload census before netcode encryption (diagnostic feature only).
use bevy::prelude::*;
use lightyear::prelude::{ConnectionSystems, Link, LinkOf, TransportSystems};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(10);

#[derive(Default, Debug)]
struct PayloadTotals {
    packets: usize,
    bytes: usize,
    peak_packets: usize,
}

impl PayloadTotals {
    fn observe(&mut self, link: &Link) {
        let packets = link.send.len();
        self.packets += packets;
        self.bytes += link.send.iter().map(|payload| payload.len()).sum::<usize>();
        self.peak_packets = self.peak_packets.max(packets);
    }
}

#[derive(Resource)]
struct NetworkWindow {
    started: Instant,
    frames: usize,
    peers: HashMap<Entity, PayloadTotals>,
}

pub(super) fn install(app: &mut App) {
    app.insert_resource(NetworkWindow {
        started: Instant::now(),
        frames: 0,
        peers: HashMap::new(),
    });
    app.add_systems(
        PostUpdate,
        count_network_payloads
            .after(TransportSystems::Send)
            .before(ConnectionSystems::Send),
    );
}

fn count_network_payloads(
    links: Query<(Entity, &Link), With<LinkOf>>,
    mut window: ResMut<NetworkWindow>,
) {
    window.frames += 1;
    for (entity, link) in &links {
        window.peers.entry(entity).or_default().observe(link);
    }
    let elapsed = window.started.elapsed();
    if elapsed < WINDOW {
        return;
    }
    for (entity, totals) in &window.peers {
        info!(
            peer = entity.to_bits(),
            frames = window.frames,
            packets = totals.packets,
            bytes = totals.bytes,
            peak_packets = totals.peak_packets,
            wall_seconds = elapsed.as_secs_f64(),
            "Network payload census before netcode encryption"
        );
    }
    window.peers.clear();
    window.frames = 0;
    window.started = Instant::now();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_census_counts_bytes_without_consuming_or_reordering_packets() {
        let mut first = Link::default();
        first.send.push(vec![1, 2, 3].into());
        first.send.push(vec![4, 5].into());
        let mut second = Link::default();
        second.send.push(vec![6].into());
        let mut totals = PayloadTotals::default();
        totals.observe(&first);
        totals.observe(&second);
        assert_eq!(
            (totals.packets, totals.bytes, totals.peak_packets),
            (3, 6, 2)
        );
        let received: Vec<Vec<u8>> = first.send.drain().map(|payload| payload.to_vec()).collect();
        assert_eq!(received, vec![vec![1, 2, 3], vec![4, 5]]);
        assert_eq!(second.send.pop().unwrap().as_ref(), &[6]);
    }
}
