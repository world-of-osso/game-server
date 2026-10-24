//! Real native event inputs; charge ownership is deliberately unchanged in C2.
use server::auras::procs::{ObservedProcEvents, ProcEventObservation};
use server::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use server::spell_cast::{
    creature_cast, powers,
    test_harness::{Harness, harness},
};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fixture(spells: &[u32], class: u8) -> Harness {
    let mut h = harness(Vec::new(), spells, class).unwrap();
    h.app.init_resource::<ObservedProcEvents>();
    h.advance(3);
    h
}

fn apply_native(h: &mut Harness, id: u32) -> u32 {
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == id)
        .unwrap()
        .instance_id
}

fn event(h: &Harness, id: u32, cast: bool) -> ProcEventObservation {
    h.app
        .world()
        .resource::<ObservedProcEvents>()
        .0
        .iter()
        .rev()
        .find(|observed| {
            let event = &observed.event;
            event.actor == h.caster
                && event.spell_id == Some(id)
                && !event.periodic
                && !event.reflected
                && (event.actor_mask[1] & 4 != 0) == cast
                && (cast || event.target.is_some())
        })
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "real native spell{id} emitted no {} event",
                if cast { "CAST" } else { "HIT" }
            )
        })
}

#[test]
#[ignore = "native ordinary CAST and actual aura-only HIT context"]
fn native_proc_capture_shield_phase_school_and_actual_empty_cost() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[17], 5);
    creature_cast(h.app.world_mut(), h.caster, 17, Some(h.caster), false).unwrap();
    assert!(auras::has_aura(h.app.world(), h.caster, 17));
    let cast = event(&h, 17, true);
    let hit = event(&h, 17, false);
    let cast = cast
        .event
        .context
        .as_ref()
        .expect("ordinary CAST native context");
    let hit = hit
        .event
        .context
        .as_ref()
        .expect("actual Shield aura HIT native context");
    assert_eq!(
        (
            cast.school_mask,
            cast.spell_type_mask,
            cast.spell_phase_mask
        ),
        (2, 7, 1)
    );
    assert_eq!(
        (hit.school_mask, hit.spell_type_mask, hit.spell_phase_mask),
        (2, 4, 2)
    );
    assert_eq!(cast.has_power_cost, Some(false));
    assert_eq!(cast.cast_from_item, Some(false));
    assert_eq!(cast.proc_chain_length, Some(0));
    assert_eq!(hit.hit_mask, 1);
}

#[test]
#[ignore = "native Fortitude actually spends its unconditional prepared mana cost"]
fn native_proc_capture_fortitude_records_real_prepared_positive_cost() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[21562], 5);
    let before = powers::power_value(h.app.world(), h.caster, PowerType::Mana)
        .unwrap()
        .0;
    creature_cast(h.app.world_mut(), h.caster, 21562, Some(h.caster), false).unwrap();
    assert!(auras::has_aura(h.app.world(), h.caster, 21562));
    assert!(
        powers::power_value(h.app.world(), h.caster, PowerType::Mana)
            .unwrap()
            .0
            < before
    );
    let observed = event(&h, 21562, true);
    assert_eq!(
        observed
            .event
            .context
            .as_ref()
            .expect("real prepared cost receipt")
            .has_power_cost,
        Some(true)
    );
}

#[test]
#[ignore = "real unused PointsIndex2 is absent from the applied-operation receipt"]
fn native_proc_capture_unused_gloomblade_operation_is_not_recorded() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[246332, 200758], 4);
    let instance = apply_native(&mut h, 246332);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.instance_id == instance)
        .unwrap();
    assert!(aura.effects.iter().any(|effect| effect.aura_type == 107
        && effect.misc_values[0] == 23
        && effect.amount == 2.0));
    let spell = h
        .app
        .world()
        .resource::<server::spell_info::SpellStore>()
        .get(200758)
        .unwrap();
    assert!(
        spell
            .effects
            .iter()
            .all(|effect| effect.index != 2 || effect.effect == 0)
    );
    let energy = powers::power_value(h.app.world(), h.caster, PowerType::Energy)
        .unwrap()
        .0;
    let health = h.health(h.npc);
    creature_cast(h.app.world_mut(), h.caster, 200758, Some(h.npc), false).unwrap();
    assert!(h.health(h.npc) < health);
    assert!(
        powers::power_value(h.app.world(), h.caster, PowerType::Energy)
            .unwrap()
            .0
            < energy
    );
    let observed = event(&h, 200758, false);
    assert!(
        observed.event.context.is_some(),
        "landed damage has actual context"
    );
    assert_eq!(
        observed.applied_modifier_ids,
        Some(Vec::new()),
        "matching an unused effect is not application"
    );
}

#[test]
#[ignore = "actual instant Regrowth preserves the exact applied Swiftness owner"]
fn native_proc_capture_swiftness_receipt_survives_into_regrowth_hit() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[132158, 8936], 11);
    creature_cast(h.app.world_mut(), h.caster, 132158, Some(h.caster), false).unwrap();
    h.advance(40);
    let instance = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 132158)
        .expect("real native Swiftness remains available")
        .instance_id;
    creature_cast(h.app.world_mut(), h.caster, 8936, Some(h.caster), false).unwrap();
    assert!(auras::has_aura(h.app.world(), h.caster, 8936));
    for cast in [true, false] {
        let observed = event(&h, 8936, cast);
        let context = observed
            .event
            .context
            .as_ref()
            .expect("actual native cast/hit context");
        if !cast {
            assert_eq!(
                context.spell_type_mask, 2,
                "resolved direct healing, including overheal"
            );
        }
        assert!(
            observed
                .applied_modifier_ids
                .as_ref()
                .expect("captured receipt, not unknown")
                .contains(&(h.caster, instance))
        );
    }
}

#[test]
#[ignore = "native instant critical operation; not a TC-positive charge oracle"]
fn native_proc_capture_actual_critical_source_and_next_cast_are_separate() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[28815, 1752], 4);
    let instance = apply_native(&mut h, 28815);
    creature_cast(h.app.world_mut(), h.caster, 1752, Some(h.npc), false).unwrap();
    let first_cast = event(&h, 1752, true);
    let first_hit = event(&h, 1752, false);
    h.advance(3);
    assert!(h.inbox.log.iter().any(|entry| entry.spell_id == Some(1752)
        && entry.kind == CombatLogKind::Damage
        && entry.amount > 0
        && entry.crit));
    let context = first_hit
        .event
        .context
        .as_ref()
        .expect("actual damage/crit context");
    assert_eq!(context.spell_type_mask, 1);
    assert_ne!(
        context.hit_mask & 2,
        0,
        "the result was critical, not fabricated NORMAL"
    );
    assert_eq!(
        first_cast.applied_modifier_ids,
        Some(Vec::new()),
        "crit still evaluates at hit, not CAST"
    );
    assert!(
        first_hit
            .applied_modifier_ids
            .as_ref()
            .unwrap()
            .contains(&(h.caster, instance))
    );
    // TC SpellAuras.cpp1834–1839 rejects28815's missing entry. This case
    // tests operation provenance, not consumption: explicitly clear the source
    // before the second cast so it stays valid after the atomic charge cutover.
    auras::remove_instance(
        h.app.world_mut(),
        h.caster,
        instance,
        auras::RemoveReason::Cancelled,
    );
    assert!(!auras::has_aura(h.app.world(), h.caster, 28815));
    h.advance(40);
    creature_cast(h.app.world_mut(), h.caster, 1752, Some(h.npc), false).unwrap();
    let next = event(&h, 1752, false);
    assert_eq!(
        next.applied_modifier_ids,
        Some(Vec::new()),
        "a later real cast must not reuse the earlier receipt"
    );
}

#[test]
#[ignore = "actual native Lava Burst child owns its applied receipt across missile delivery"]
fn native_proc_capture_delayed_lava_child_does_not_record_into_parent() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[378269, 51505, 285452], 7);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 20.0;
    creature_cast(h.app.world_mut(), h.caster, 51505, Some(h.npc), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    // This C2 case does not require C4's charge retention. Gain the actual
    // native source after ordinary launch, while its missile is still in flight.
    // TC Spell.cpp3679–3694 gives the child its own taking-spell scope.
    for _ in 0..600 {
        if h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
        {
            break;
        }
        h.advance(1);
    }
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|entry| entry.spell_id == Some(285452) && entry.kind == CombatLogKind::Damage),
        "actual missile has not impacted at launch"
    );
    let instance = apply_native(&mut h, 378269);
    for _ in 0..600 {
        h.advance(1);
        if h.inbox.log.iter().any(|entry| {
            entry.spell_id == Some(285452)
                && entry.kind == CombatLogKind::Damage
                && entry.amount > 0
        }) {
            break;
        }
    }
    let impact = h
        .inbox
        .log
        .iter()
        .find(|entry| {
            entry.spell_id == Some(285452)
                && entry.kind == CombatLogKind::Damage
                && entry.amount > 0
        })
        .expect("real native missile child285452 landed");
    let parent = event(&h, 51505, true);
    let child = event(&h, 285452, false);
    assert!(parent.event.context.is_some());
    let context = child
        .event
        .context
        .as_ref()
        .expect("actual delayed child HIT context");
    assert_eq!(context.spell_phase_mask, 2);
    assert_eq!(context.spell_type_mask, 1);
    assert_eq!(context.hit_mask & 2 != 0, impact.crit);
    assert_eq!(
        parent.applied_modifier_ids,
        Some(Vec::new()),
        "damage-only operation was not applied at parent CAST"
    );
    assert!(
        child
            .applied_modifier_ids
            .as_ref()
            .expect("child receipt, not unavailable")
            .contains(&(h.caster, instance))
    );
}

#[test]
#[ignore = "late Lava Surge did not modify the already-admitted hardcast"]
fn native_proc_capture_late_surge_is_not_an_applied_admission_modifier() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[51505, 285452, 77762], 7);
    creature_cast(h.app.world_mut(), h.caster, 51505, Some(h.npc), false).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .unwrap()
            .spell_id,
        51505
    );
    creature_cast(h.app.world_mut(), h.caster, 77762, Some(h.caster), true).unwrap();
    let instance = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 77762)
        .expect("actual proc gained while51505 still casts")
        .instance_id;
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .unwrap()
            .spell_id,
        51505
    );
    for _ in 0..600 {
        if h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
        {
            break;
        }
        h.advance(1);
    }
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
    let observed = event(&h, 51505, true);
    let applied = observed
        .applied_modifier_ids
        .expect("actual cast receipt, not unknown");
    assert!(
        !applied.contains(&(h.caster, instance)),
        "a finish-time facing/readiness query did not shorten the admitted cast"
    );
}

#[test]
#[ignore = "actual BodyAndSoul proc child captures the raised invocation chain length"]
fn native_proc_capture_real_aura_proc_child_retains_chain_provenance() {
    let _seed = server::random::seed_scope(70448);
    let mut h = fixture(&[64129, 65081, 17], 5);
    apply_native(&mut h, 64129);
    creature_cast(h.app.world_mut(), h.caster, 17, Some(h.caster), false).unwrap();
    for _ in 0..20 {
        h.advance(1);
        if auras::has_aura(h.app.world(), h.caster, 65081) {
            break;
        }
    }
    assert!(
        auras::has_aura(h.app.world(), h.caster, 65081),
        "actual ordinary Shield triggered native65081"
    );
    let observed = event(&h, 65081, false);
    let context = observed
        .event
        .context
        .as_ref()
        .expect("actual aura-triggered child context");
    assert_eq!(context.proc_chain_length, Some(1));
    assert_eq!(context.is_triggered, Some(true));
}
