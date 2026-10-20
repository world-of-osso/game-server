//! Ignored native chain regressions, written after qualification and before consumer.
use super::*;
use shared::protocol::CombatLogEvent;

fn behavior_fixture(passive: bool) -> Harness {
    let mut h = fixture(passive);
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    h
}

fn heals(h: &Harness, owner: Entity) -> Vec<CombatLogEvent> {
    h.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(HEAL)
                && event.source == Some(owner.to_bits())
                && event.kind == CombatLogKind::Heal
        })
        .cloned()
        .collect()
}

fn assert_owned_children(h: &Harness, owner: Entity, count: usize) -> AuraInstance {
    let cooldown = aura(h, owner, COOLDOWN).expect("native30884 proc must cast real445698");
    assert_eq!(cooldown.caster, owner);
    assert_eq!(cooldown.duration_ms, 45000);
    let healing = heals(h, owner);
    assert_eq!(healing.len(), count, "{healing:?}");
    assert!(
        healing
            .iter()
            .all(|event| event.target == Some(owner.to_bits())
                && event.amount == 400
                && event.overflow == 0
                && !event.crit),
        "{healing:?}"
    );
    cooldown
}

fn take_swings(h: &mut Harness, actors: &[Entity], count: usize) {
    let starts = actors.to_vec();
    let target = h.caster;
    let first_event = h.inbox.log.len();
    h.app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            for actor in &starts {
                crate::combat::start_attack(&mut commands, *actor, target, None);
            }
        })
        .unwrap();
    for _ in 0..count * 60 {
        h.advance(1);
        let hits: Vec<_> = h.inbox.log[first_event..]
            .iter()
            .filter(|event| {
                event.spell_id.is_none()
                    && event.target == Some(target.to_bits())
                    && actors
                        .iter()
                        .any(|actor| event.source == Some(actor.to_bits()))
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .collect();
        if hits.len() >= count {
            assert_eq!(hits.len(), count);
            assert!(
                hits.iter().all(|event| event.kind == CombatLogKind::Damage
                    && event.amount == 10
                    && !event.crit),
                "qualified native10-damage path changed: {hits:?}"
            );
            for actor in actors {
                crate::combat::attack_stop(h.app.world_mut(), *actor);
            }
            h.advance(3);
            return;
        }
    }
    panic!(
        "native actors failed to deliver {count} qualified swings: {:?}",
        h.inbox.log
    );
}

#[test]
#[ignore = "behavioral RED before Nature's Guardian named proc consumer"]
fn native_natures_guardian_below_threshold_casts_cooldown_before_real_heal() {
    let mut h = behavior_fixture(true);
    let owner = h.caster;
    cast_native_wound(&mut h, owner);
    assert_eq!(h.health(owner), 600.0);
    assert_owned_children(&h, owner, 1);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let cooldown = events
        .iter()
        .position(|event| {
            event.spell_id == Some(COOLDOWN)
                && event.kind == CombatLogKind::AuraApplied
                && event.target == Some(owner.to_bits())
        })
        .unwrap();
    let healing = events
        .iter()
        .position(|event| {
            event.spell_id == Some(HEAL)
                && event.kind == CombatLogKind::Heal
                && event.target == Some(owner.to_bits())
        })
        .unwrap();
    assert!(
        cooldown < healing,
        "source cooldown must be applied before default heal"
    );
}

#[test]
#[ignore = "behavioral RED before Nature's Guardian strict current-health check"]
fn native_natures_guardian_above_and_equal_do_not_proc_but_below_does() {
    let mut h = behavior_fixture(true);
    let (owner, attacker) = (h.caster, h.npc);
    buff_native_npc(&mut h, attacker);
    take_swings(&mut h, &[attacker], 64);
    assert_eq!(h.health(owner), 360.0);
    assert!(heals(&h, owner).is_empty());
    assert!(aura(&h, owner, COOLDOWN).is_none());
    take_swings(&mut h, &[attacker], 1);
    assert_eq!(h.health(owner), 350.0);
    assert!(heals(&h, owner).is_empty());
    assert!(aura(&h, owner, COOLDOWN).is_none());
    take_swings(&mut h, &[attacker], 1);
    assert_eq!(h.health(owner), 740.0);
    assert_owned_children(&h, owner, 1);
}

#[test]
#[ignore = "behavioral RED before Nature's Guardian native cooldown lifetime"]
fn native_natures_guardian_cooldown_blocks_repeat_then_natural_expiry_rearms() {
    let mut h = behavior_fixture(true);
    let (owner, first) = (h.caster, h.npc);
    cast_native_wound(&mut h, owner);
    assert_eq!(h.health(owner), 600.0);
    let expiry = assert_owned_children(&h, owner, 1).expires_at.unwrap();
    let second = spawn_npc(&mut h.app, 3.0);
    buff_native_npc(&mut h, first);
    buff_native_npc(&mut h, second);
    take_swings(&mut h, &[first, second], 26);
    assert_eq!(h.health(owner), 340.0);
    assert!(h.app.world().resource::<Time>().elapsed_secs_f64() < expiry);
    assert_owned_children(&h, owner, 1);
    while h.app.world().resource::<Time>().elapsed_secs_f64() <= expiry {
        h.advance(1);
    }
    h.advance(3);
    assert!(aura(&h, owner, COOLDOWN).is_none());
    assert_eq!(h.health(owner), 340.0);
    take_swings(&mut h, &[first], 1);
    assert_eq!(h.health(owner), 730.0);
    assert_owned_children(&h, owner, 2);
}

#[test]
#[ignore = "behavioral RED before Nature's Guardian preserves its player's cast"]
fn native_natures_guardian_actual_proc_keeps_player_cast_and_owned_completion() {
    let mut h = behavior_fixture(true);
    let (owner, attacker) = (h.caster, h.npc);
    buff_native_npc(&mut h, attacker);
    take_swings(&mut h, &[attacker], 65);
    assert_eq!(h.health(owner), 350.0);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(attacker));
    let original = h.app.world().get::<CastState>(owner).unwrap().clone();
    assert_eq!(original.spell_id, LIGHTNING_BOLT);
    take_swings(&mut h, &[attacker], 1);
    assert_eq!(h.health(owner), 740.0);
    assert_owned_children(&h, owner, 1);
    let active = h
        .app
        .world()
        .get::<CastState>(owner)
        .expect("proc must not cancel original actual player cast");
    assert_eq!(
        (active.spell_id, active.target),
        (original.spell_id, original.target)
    );
    h.advance(200);
    assert!(h.app.world().get::<CastState>(owner).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(LIGHTNING_BOLT)
                && event.kind == CombatLogKind::Damage
                && event.source == Some(owner.to_bits())
                && event.target == Some(attacker.to_bits())
                && event.amount > 0)
    );
}

#[test]
#[ignore = "native Nature's Guardian missing-passive control"]
fn native_natures_guardian_without_passive_has_no_children() {
    let mut h = behavior_fixture(false);
    let owner = h.caster;
    cast_native_wound(&mut h, owner);
    assert_eq!(h.health(owner), 200.0);
    assert!(heals(&h, owner).is_empty());
    assert!(aura(&h, owner, COOLDOWN).is_none());
}

#[test]
#[ignore = "native Nature's Guardian removed-passive control"]
fn native_natures_guardian_unlearned_passive_has_no_children() {
    let mut h = behavior_fixture(true);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            COOLDOWN,
            HEAL,
            LIGHTNING_BOLT,
            WOUND,
            NPC_BUFFS,
        ]));
    super::super::remove_auras(
        h.app.world_mut(),
        owner,
        super::super::RemoveReason::Unlearned,
        |aura| aura.spell_id == PASSIVE,
    );
    h.advance(3);
    assert!(aura(&h, owner, PASSIVE).is_none());
    cast_native_wound(&mut h, owner);
    assert_eq!(h.health(owner), 200.0);
    assert!(heals(&h, owner).is_empty());
    assert!(aura(&h, owner, COOLDOWN).is_none());
}

#[test]
#[ignore = "behavioral RED before Nature's Guardian two-owner isolation"]
fn native_natures_guardian_two_shamans_keep_their_own_heals_and_cooldowns() {
    let mut h = behavior_fixture(true);
    let first = h.caster;
    let second = crate::spell_cast::test_harness::spawn_player(&mut h.app, 7, 1.0);
    h.app.world_mut().entity_mut(second).insert((
        crate::class_progression::ActiveSpec(262),
        crate::class_progression::KnownSpells::fixture(vec![PASSIVE, COOLDOWN, HEAL]),
    ));
    h.advance(3);
    assert_eq!(aura(&h, second, PASSIVE).unwrap().caster, second);
    cast_native_wound(&mut h, first);
    assert_eq!(h.health(first), 600.0);
    assert_eq!(h.health(second), 1000.0);
    let first_expiry = assert_owned_children(&h, first, 1).expires_at.unwrap();
    assert!(aura(&h, second, COOLDOWN).is_none());
    cast_native_wound(&mut h, second);
    assert_eq!(h.health(first), 600.0);
    assert_eq!(h.health(second), 600.0);
    let second_expiry = assert_owned_children(&h, second, 1).expires_at.unwrap();
    assert!(second_expiry > first_expiry);
}

#[test]
#[ignore = "native Nature's Guardian is not a lethal-hit rescue"]
fn native_natures_guardian_lethal_hit_is_not_rescued() {
    let mut h = harness(Vec::new(), &[PASSIVE, COOLDOWN, HEAL, 314_616], 7).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(262));
    h.advance(3);
    let (owner, attacker) = (h.caster, h.npc);
    crate::spell_cast::creature_cast(h.app.world_mut(), attacker, 314_616, Some(owner), false)
        .unwrap();
    h.advance(5);
    assert_eq!(h.health(owner), 0.0);
    assert!(heals(&h, owner).is_empty());
}
