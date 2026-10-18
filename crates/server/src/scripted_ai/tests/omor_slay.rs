//! TC a352b1fa Omor KilledUnit: actual lethal damage, not last-damager inference.
use super::*;

fn slay_fixture() -> (Harness, Entity) {
    let (mut h, boss, _) = super::omor::orbital_fixture();
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 5.0,
        ..origin
    });
    install_player_death_transition(&mut h);
    (h, boss)
}

pub(super) fn install_player_death_transition(h: &mut Harness) {
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::persistence::ClientId(1001));
    h.app.init_resource::<crate::death::DeathServerState>();
    h.app
        .init_resource::<crate::death::DeathTransitionCounters>();
    h.app.add_systems(
        FixedUpdate,
        crate::death::process_player_death_transitions.after(crate::combat::auto_attack_system),
    );
}

fn ordinary_hound_fixture() -> (Harness, Entity, Entity, InMap) {
    let (mut h, boss, map) = super::omor::summon_speech_fixture();
    install_player_death_transition(&mut h);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30707, Some(boss), false)
        .expect("actual ordinary30707 hound admission");
    h.advance(2.1);
    let hounds = h.creatures_of(17540);
    assert_eq!(hounds.len(), 1, "actual source owned Fiendish Hound");
    let hound = hounds[0];
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(hound)
            .unwrap()
            .0,
        boss.to_bits()
    );
    (h, boss, hound, map)
}

fn slay_count(h: &Harness, boss: Entity) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17308][&3];
    let name = &h
        .app
        .world()
        .get::<shared::components::Npc>(boss)
        .unwrap()
        .name;
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            let authored = texts
                .iter()
                .any(|text| line.content == text.male_text || line.content == text.female_text);
            let from_boss = line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && &line.sender == name;
            authored && from_boss
        })
        .count()
}

fn cast_ordinary_shadow_bolt(h: &mut Harness, boss: Entity, target: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30686, Some(target), false)
        .expect("actual native Omor Shadow Bolt admission");
    h.advance(4.2); // Native cast/flight/recovery; do not force a hit or reset cooldowns.
}

#[test]
fn encounter_coverage_omor_slay_ordinary_shadow_bolt_speaks_once_on_player_death() {
    let (mut h, boss) = slay_fixture();
    let player = h.player();
    let before = h.health(player).current;
    cast_ordinary_shadow_bolt(&mut h, boss, player);
    assert!(h.health(player).current > 0.0, "initial hit is nonlethal");
    assert!(
        h.health(player).current < before,
        "actual source30686 damage before slay test"
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "nonlethal damage cannot speak slay"
    );
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1.0;
    // The initial ordinary bolt engages the real source scheduler. Let its next
    // native cast finish instead of competing with it or bypassing InProgress.
    h.advance(14.1);
    assert_eq!(h.health(player).current, 0.0, "actual lethal30686 impact");
    let death = h
        .app
        .world()
        .get::<crate::death::PlayerDeathState>(player)
        .expect("production death transition creates corpse state");
    assert_eq!(death.state, shared::death::DeathState::Dead);
    assert!(
        death.corpse.is_some(),
        "source death boundary precedes slay hook"
    );
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(30686)
                && event.amount > 0
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "owned native Shadow Bolt damage receipt"
    );
    assert_eq!(
        slay_count(&h, boss),
        1,
        "source KilledUnit emits exactly one authored Talk3"
    );
    h.app
        .world_mut()
        .write_message(crate::death::PlayerDeathTransition { player });
    h.advance(1.0);
    assert_eq!(
        slay_count(&h, boss),
        1,
        "repeated death/idle ticks cannot replay slay"
    );
    assert!(h.inbox(1).chat.is_empty(), "other copy cannot hear slay");
}

#[test]
fn encounter_coverage_omor_slay_actual_creature_kill_speaks_once() {
    let (mut h, boss, hound, _) = ordinary_hound_fixture();
    assert!(h.health(hound).current > 0.0);
    crate::combat::kill_unit(h.app.world_mut(), boss, hound);
    h.advance(0.05);
    assert_eq!(h.health(hound).current, 0.0);
    assert!(h.app.world().get::<crate::combat::Corpse>(hound).is_some());
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(hound)
            .is_some()
    );
    assert_eq!(
        slay_count(&h, boss),
        1,
        "KilledUnit has no player-victim restriction"
    );
    crate::combat::kill_unit(h.app.world_mut(), boss, hound);
    h.advance(0.1);
    assert_eq!(
        slay_count(&h, boss),
        1,
        "already-dead target cannot repeat slay"
    );
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
fn encounter_coverage_omor_slay_fall_death_ignores_stale_last_damager() {
    let (mut h, boss, _, _) = ordinary_hound_fixture();
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::combat::LastDamager(boss));
    let world = h.app.world_mut();
    let mut health = world.get_mut::<Health>(player).unwrap();
    health.current = 1.0;
    let fall = crate::death::deal_environmental_damage(
        player,
        shared::protocol::EnvironmentalKind::Falling,
        &mut health,
        1.0,
    )
    .expect("actual environmental fall damage boundary");
    assert_eq!(fall.source, None, "environmental death has no killer");
    assert_eq!(health.current, 0.0);
    crate::spell_cast::log_event(world, fall);
    world.write_message(crate::death::PlayerDeathTransition { player });
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(player)
            .unwrap()
            .state,
        shared::death::DeathState::Dead
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "old nonlethal attacker is not the fall killer"
    );
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
fn encounter_coverage_omor_slay_owned_hound_kill_does_not_credit_owner() {
    let (mut h, boss, hound, map) = ordinary_hound_fixture();
    let player = h.player();
    let observer = *h.app.world().get::<Position>(player).unwrap();
    let point = Vec3::new(observer.x + 3.0, observer.y, observer.z);
    let support = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, point);
    let crate::ground::GroundProbe::Supported(height) = support else {
        panic!("authentic observer-side hound floor: {support:?}");
    };
    h.app.world_mut().entity_mut(hound).insert(Position {
        x: point.x,
        y: height,
        z: point.z,
    });
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1.0;
    crate::creature_aggro::add_threat(h.app.world_mut(), hound, player, 1.0);
    h.advance(20.0);
    assert_eq!(
        h.health(player).current,
        0.0,
        "actual hound melee kills victim"
    );
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(hound.to_bits())
                && event.target == Some(player.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Death
        }),
        "real killing blow belongs to hound, not Omor"
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "owner rewards do not redirect killer AI speech"
    );
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
fn encounter_coverage_omor_slay_revived_transition_does_not_poison_later_fall() {
    let (mut h, boss, _, _) = ordinary_hound_fixture();
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), boss, player);
    // Cancel this queued death before its corpse transition, then die to a fall.
    let maximum = h.health(player).max;
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = maximum;
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(player)
            .is_none()
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "invalid queued death cannot notify killer"
    );
    let world = h.app.world_mut();
    let fall = crate::death::deal_environmental_damage(
        player,
        shared::protocol::EnvironmentalKind::Falling,
        &mut world.get_mut::<Health>(player).unwrap(),
        maximum,
    )
    .expect("later actual environmental damage");
    crate::spell_cast::log_event(world, fall);
    world.write_message(crate::death::PlayerDeathTransition { player });
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(player)
            .unwrap()
            .state,
        shared::death::DeathState::Dead
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "consumed invalid cause cannot leak into later fall"
    );
}

#[derive(Resource, Default)]
struct DeathOrder(Vec<(&'static str, Entity, bool, f32)>);

struct DeathOrderProbe;

fn record_death_order(ctx: &mut Ctx, victim: Entity, phase: &'static str) {
    let corpse = ctx.world.get::<crate::combat::Corpse>(victim).is_some();
    let health = ctx.world.get::<Health>(victim).unwrap().current;
    ctx.world
        .resource_mut::<DeathOrder>()
        .0
        .push((phase, victim, corpse, health));
}

impl CreatureAi for DeathOrderProbe {
    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        record_death_order(ctx, victim, "killer");
    }
    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        let victim = ctx.me;
        record_death_order(ctx, victim, "victim");
    }
}

#[test]
fn encounter_coverage_omor_slay_corpse_precedes_killer_before_victim_death_script() {
    let (mut h, boss, hound, _) = ordinary_hound_fixture();
    h.app.init_resource::<DeathOrder>();
    for actor in [boss, hound] {
        h.app.world_mut().entity_mut(actor).insert(CreatureAiState {
            ai: Some(Box::new(DeathOrderProbe)),
            pending: Default::default(),
        });
    }
    crate::combat::kill_unit(h.app.world_mut(), boss, hound);
    h.advance(0.05);
    assert_eq!(
        h.app.world().resource::<DeathOrder>().0,
        vec![("killer", hound, true, 0.0), ("victim", hound, true, 0.0)],
        "script-observed corpse/health and notification order follow approved boundary"
    );
}
