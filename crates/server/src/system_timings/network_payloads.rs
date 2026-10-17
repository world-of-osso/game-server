//! Read-only transport payload census before netcode encryption (diagnostic feature only).
use bevy::prelude::*;
use lightyear::prelude::{ChannelRegistry, ConnectionSystems, Link, LinkOf, Transport};
use lightyear_transport::plugin::TransportSystems;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(10);

#[derive(Default, Debug)]
struct PayloadTotals {
    packets: usize,
    bytes: usize,
    peak_packets: usize,
    channels: HashMap<&'static str, ChannelTotals>,
}

#[derive(Default, Debug)]
struct ChannelTotals {
    sent: usize,
    acked: usize,
    nacked: usize,
}

impl PayloadTotals {
    fn observe(&mut self, link: &Link) {
        let packets = link.send.len();
        self.packets += packets;
        self.bytes += link.send.iter().map(|payload| payload.len()).sum::<usize>();
        self.peak_packets = self.peak_packets.max(packets);
    }

    fn observe_transport(&mut self, transport: &Transport, registry: &ChannelRegistry) {
        for (kind, sender) in &transport.senders {
            let name = registry.get_name_from_kind(kind);
            let channel = self.channels.entry(name).or_default();
            channel.sent += sender.messages_sent.len();
            channel.acked += sender.message_acks.len();
            channel.nacked += sender.message_nacks.len();
        }
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
    links: Query<(Entity, &Link, Option<&Transport>), With<LinkOf>>,
    registry: Res<ChannelRegistry>,
    mut window: ResMut<NetworkWindow>,
) {
    window.frames += 1;
    for (entity, link, transport) in &links {
        let totals = window.peers.entry(entity).or_default();
        totals.observe(link);
        if let Some(transport) = transport {
            totals.observe_transport(transport, &registry);
        }
    }
    let elapsed = window.started.elapsed();
    if elapsed < WINDOW {
        return;
    }
    report_network_window(&window, elapsed);
    window.peers.clear();
    window.frames = 0;
    window.started = Instant::now();
}

fn report_network_window(window: &NetworkWindow, elapsed: Duration) {
    for (entity, totals) in &window.peers {
        for (name, channel) in &totals.channels {
            if channel.sent + channel.acked + channel.nacked == 0 {
                continue;
            }
            info!(
                peer = entity.to_bits(),
                channel = name,
                sent = channel.sent,
                acked = channel.acked,
                nacked = channel.nacked,
                "Network channel census (transport message/fragment records)"
            );
        }
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
