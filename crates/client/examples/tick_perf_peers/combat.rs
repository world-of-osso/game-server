//! Explicit private dummy workload; combat evidence comes from received server events.
use bevy::prelude::*;
use bevy_replicon::shared::server_entity_map::ServerEntityMap;
use lightyear::prelude::*;
use shared::components::{MovementControl, Npc, Position};
use shared::protocol::*;
use std::time::{Duration, Instant};

const CAST_INTERVAL: Duration = Duration::from_secs(2);
const MOVE_INTERVAL: Duration = Duration::from_millis(100);
const APPROACH_SPEED: f32 = 1.0;
const MELEE_DISTANCE: f32 = 2.5;

#[derive(Resource, Clone)]
pub struct Combat {
    pub owner: Option<u64>,
    entry: u32,
    spells: Vec<u32>,
    known: Vec<u32>,
    target: Option<u64>,
    last_cast: Option<Instant>,
    last_move: Option<Instant>,
    casts_sent: usize,
    moves_sent: usize,
    observed_position: Option<Vec3>,
    observed_moves: usize,
    evidence: Evidence,
}

#[derive(Default, Clone, Debug)]
struct Evidence {
    damage: usize,
    casts: usize,
    auras: usize,
    rejected: usize,
}

impl Evidence {
    fn observe(&mut self, event: &CombatLogEvent, owner: u64, target: u64) {
        if event.source != Some(owner) {
            return;
        }
        match event.kind {
            CombatLogKind::Damage if event.target == Some(target) && event.amount > 0 => {
                self.damage += 1;
            }
            CombatLogKind::CastSuccess => self.casts += 1,
            CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed => self.auras += 1,
            _ => {}
        }
    }
}

impl Combat {
    pub fn read_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().skip(2).collect();
        if args.is_empty() {
            return None;
        }
        assert_eq!(
            args.len(),
            3,
            "usage: endpoint combat <npc-entry> <spell,spell>"
        );
        assert_eq!(args[0], "combat", "unknown workload");
        let spells: Vec<u32> = args[2]
            .split(',')
            .map(|value| value.parse().expect("combat spell ID"))
            .collect();
        assert!(!spells.is_empty(), "explicit combat spells required");
        Some(Self {
            owner: None,
            entry: args[1].parse().expect("combat NPC entry"),
            spells,
            known: Vec::new(),
            target: None,
            last_cast: None,
            last_move: None,
            casts_sent: 0,
            moves_sent: 0,
            observed_position: None,
            observed_moves: 0,
            evidence: Evidence::default(),
        })
    }

    pub fn assert_complete(&self, username: &str) {
        println!(
            "COMBAT_COMPLETE {username} owner={:?} target={:?} submitted={} moves={} observed_moves={} evidence={:?}",
            self.owner,
            self.target,
            self.casts_sent,
            self.moves_sent,
            self.observed_moves,
            self.evidence
        );
        assert!(
            self.evidence.damage >= 10,
            "{username}: fewer than ten own damage events"
        );
        assert!(
            self.evidence.casts >= 10,
            "{username}: fewer than ten own successful casts"
        );
        assert!(
            self.evidence.auras >= 1,
            "{username}: no own aura application/refresh"
        );
        assert!(
            self.observed_moves >= 10,
            "{username}: no sustained authoritative movement"
        );
    }
}

pub fn connect(event: On<Add, Connected>, mut commands: Commands) {
    commands.entity(event.entity).insert((
        MessageSender::<SetTarget>::default(),
        MessageSender::<AttackSwing>::default(),
        MessageSender::<SpellCastIntent>::default(),
        MessageSender::<PlayerInput>::default(),
        MessageReceiver::<KnownSpellsSnapshot>::default(),
        MessageReceiver::<CombatLogEvent>::default(),
        MessageReceiver::<CastFailed>::default(),
    ));
}

pub fn receive(
    combat: Option<ResMut<Combat>>,
    mut peers: Query<(
        &mut MessageReceiver<KnownSpellsSnapshot>,
        &mut MessageReceiver<CombatLogEvent>,
        &mut MessageReceiver<CastFailed>,
    )>,
) {
    let Some(mut combat) = combat else { return };
    for (mut known, mut events, mut failures) in &mut peers {
        for snapshot in known.receive() {
            combat.known = snapshot.spells;
        }
        for event in events.receive() {
            if let (Some(owner), Some(target)) = (combat.owner, combat.target) {
                combat.evidence.observe(&event, owner, target);
            }
        }
        for failure in failures.receive() {
            combat.evidence.rejected += 1;
            println!("COMBAT_REJECTED owner={:?} {failure:?}", combat.owner);
        }
    }
}

type Senders<'a> = (
    &'a mut MessageSender<SetTarget>,
    &'a mut MessageSender<AttackSwing>,
    &'a mut MessageSender<SpellCastIntent>,
    &'a mut MessageSender<PlayerInput>,
);

pub fn fight(
    run: Res<super::Run>,
    combat: Option<ResMut<Combat>>,
    map: Res<ServerEntityMap>,
    positions: Query<&Position>,
    controls: Query<&MovementControl>,
    npcs: Query<(&Npc, &Position)>,
    mut peers: Query<Senders>,
) {
    let Some(mut combat) = combat else { return };
    let Some(owner) = combat.owner else { return };
    let Some(local) = map.to_client().get(&Entity::from_bits(owner)) else {
        return;
    };
    let Ok(position) = positions.get(*local) else {
        return;
    };
    let Ok(control) = controls.get(*local) else {
        return;
    };
    let origin = Vec3::new(position.x, position.y, position.z);
    if combat
        .observed_position
        .is_some_and(|previous| previous.distance(origin) > 0.01)
    {
        combat.observed_moves += 1;
    }
    combat.observed_position = Some(origin);
    let Some((target, destination)) = nearest_dummy(&map, &npcs, combat.entry, origin) else {
        return;
    };
    for (mut targeting, mut attack, mut casting, mut movement) in &mut peers {
        if combat.target != Some(target) {
            targeting.send::<CombatChannel>(SetTarget {
                target_entity: Some(target),
            });
            attack.send::<CombatChannel>(AttackSwing { target });
            combat.target = Some(target);
            println!(
                "COMBAT_TARGET {} owner={owner} target={target}",
                run.username
            );
        }
        send_movement(
            &mut combat,
            &mut movement,
            origin,
            destination,
            control.epoch,
        );
        if run.all_entered.is_some() {
            send_cast(&mut combat, &mut casting, target);
        }
    }
}

fn nearest_dummy(
    map: &ServerEntityMap,
    npcs: &Query<(&Npc, &Position)>,
    entry: u32,
    origin: Vec3,
) -> Option<(u64, Vec3)> {
    map.to_client()
        .iter()
        .filter_map(|(server, client)| {
            let (npc, position) = npcs.get(*client).ok()?;
            let destination = Vec3::new(position.x, position.y, position.z);
            (npc.template_id == entry).then_some((server.to_bits(), destination))
        })
        .min_by(|a, b| {
            origin
                .distance_squared(a.1)
                .total_cmp(&origin.distance_squared(b.1))
        })
}

fn send_cast(combat: &mut Combat, sender: &mut MessageSender<SpellCastIntent>, target: u64) {
    if combat
        .last_cast
        .is_some_and(|time| time.elapsed() < CAST_INTERVAL)
    {
        return;
    }
    assert!(
        combat
            .spells
            .iter()
            .all(|spell| combat.known.contains(spell)),
        "requested workload spells {:?} are not all learned: {:?}",
        combat.spells,
        combat.known
    );
    let spell_id = combat.spells[combat.casts_sent % combat.spells.len()];
    sender.send::<CombatChannel>(SpellCastIntent {
        spell_id: Some(spell_id),
        spell: String::new(),
        target_entity: Some(target),
        target_item_guid: None,
        destination: None,
        witness: None,
    });
    combat.casts_sent += 1;
    combat.last_cast = Some(Instant::now());
}

fn send_movement(
    combat: &mut Combat,
    sender: &mut MessageSender<PlayerInput>,
    origin: Vec3,
    target: Vec3,
    epoch: u32,
) {
    if combat
        .last_move
        .is_some_and(|time| time.elapsed() < MOVE_INTERVAL)
    {
        return;
    }
    let delta = Vec3::new(target.x - origin.x, 0.0, target.z - origin.z);
    let distance = delta.length();
    let facing_yaw = delta.x.atan2(delta.z);
    let approaching = distance > MELEE_DISTANCE;
    // Small tangential steps keep melee peers moving without leaving their target's reach.
    let direction = if approaching {
        delta.normalize()
    } else {
        Vec3::new(delta.z, 0.0, -delta.x).normalize_or_zero()
    };
    let step = APPROACH_SPEED * MOVE_INTERVAL.as_secs_f32();
    let destination = origin + direction * step;
    sender.send::<MovementChannel>(PlayerInput {
        direction: direction.to_array(),
        facing_yaw,
        jumping: false,
        running: false,
        swimming: false,
        flying: false,
        position: destination.to_array(),
        epoch,
        transport: None,
    });
    combat.moves_sent += 1;
    combat.last_move = Some(Instant::now());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_excludes_other_actors_and_other_victims() {
        let mut evidence = Evidence::default();
        let mut event = CombatLogEvent {
            source: Some(10),
            target: Some(20),
            spell_id: Some(772),
            school_mask: 1,
            amount: 5,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: true,
            extra_spell_id: None,
            timestamp_unix_ms: 1,
            kind: CombatLogKind::Damage,
        };
        evidence.observe(&event, 10, 20);
        event.source = Some(11);
        evidence.observe(&event, 10, 20);
        event.source = Some(10);
        event.target = Some(21);
        evidence.observe(&event, 10, 20);
        event.target = Some(20);
        event.kind = CombatLogKind::AuraApplied;
        evidence.observe(&event, 10, 20);
        event.kind = CombatLogKind::CastSuccess;
        evidence.observe(&event, 10, 20);
        assert_eq!((evidence.damage, evidence.casts, evidence.auras), (1, 1, 1));
    }
}
