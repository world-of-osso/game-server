//! TC a352b1fa boss_leymor.cpp481–492: REAL e0/aura23 removal self-death.
//! Positive recipients come only from the real ordinary45s Stomp, never injected auras.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};

const STOMP: u32 = 374720;
const DAMAGE: u32 = 374731;
const SPROUT: u32 = 190509;
const POWER: u32 = 374736;
const STEP: f32 = 0.05;

fn owned_stomp(h: &Harness, unit: Entity, leymor: Entity) -> Option<&AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == STOMP && aura.caster == leymor)
}

fn is_recipient(h: &Harness, unit: Entity, leymor: Entity) -> bool {
    owned_stomp(h, unit, leymor).is_some_and(|aura| {
        aura.effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == 23)
    })
}

fn living(h: &Harness, unit: Entity) -> bool {
    crate::spell_cast::targets::is_alive(h.app.world(), unit)
}

fn stacks(h: &Harness, leymor: Entity) -> usize {
    crate::auras::aura_count(h.app.world(), leymor, POWER) as usize
}

fn damage_count(h: &Harness, leymor: Entity) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(DAMAGE)
                && event.source == Some(leymor.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .count()
}

fn fixture() -> (Harness, Entity) {
    // Reuse the already-proven native intro/Tender-depletion setup. No Sprout is
    // killed or granted an aura here: ordinary374364/missile/summon owns its birth.
    let (h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let spell = h.app.world().resource::<SpellStore>().get(STOMP).unwrap();
    let first = &spell.effects[0];
    assert_eq!(
        (
            first.index,
            first.effect,
            first.aura,
            first.aura_period_ms,
            first.trigger_spell
        ),
        (0, 6, 23, 500, 374727)
    );
    assert_eq!((spell.effects[1].index, spell.effects[1].aura), (1, 4));
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .copy_difficulty(map)
            .unwrap()
            .difficulty_id,
        205
    );
    (h, leymor)
}

fn wait_for_stomp(h: &mut Harness, leymor: Entity) -> Vec<Entity> {
    for _ in 0..1000 {
        if h.app
            .world()
            .get::<CastState>(leymor)
            .is_some_and(|cast| cast.spell_id == STOMP)
        {
            let recipients: Vec<_> = h
                .creatures_of(SPROUT)
                .into_iter()
                .filter(|&unit| living(h, unit) && is_recipient(h, unit, leymor))
                .collect();
            assert!(
                !recipients.is_empty(),
                "prerequisite: real channel selected native e0/aura23 recipients"
            );
            assert_eq!(
                damage_count(h, leymor),
                0,
                "no completion child while actual channel stands"
            );
            let player = h.player();
            let aura =
                owned_stomp(h, player, leymor).expect("native e1 Player recipient prerequisite");
            assert!(
                aura.effects
                    .iter()
                    .any(|effect| effect.index == 1 && effect.aura_type == 4)
            );
            assert!(
                !is_recipient(h, player, leymor),
                "Player has e1 only, not the self-kill effect"
            );
            return recipients;
        }
        h.advance(STEP);
    }
    panic!("prerequisite: ordinary source45s Stomp channel did not occur");
}

fn engage_other_copy(h: &mut Harness) -> (Entity, InMap) {
    let (leymor, map) = super::azure_intro::spawn_second_copy(h);
    let player = h.players[1].0;
    let tenders: Vec<_> = h
        .creatures_of(191164)
        .into_iter()
        .filter(|&unit| h.app.world().get::<InMap>(unit) == Some(&map))
        .collect();
    assert_eq!(
        tenders.len(),
        3,
        "same sourced intro prerequisite as the first copy"
    );
    for tender in tenders {
        crate::combat::kill_unit(h.app.world_mut(), player, tender);
    }
    h.advance(1.2);
    assert!(!h.has_aura(leymor, 375729));
    let position = *h.app.world().get::<Position>(leymor).unwrap();
    h.app.world_mut().entity_mut(player).insert(position);
    crate::creature_aggro::add_threat(h.app.world_mut(), leymor, player, 1.0);
    h.advance(6.1);
    let children: Vec<_> = h
        .creatures_of(SPROUT)
        .into_iter()
        .filter(|&unit| h.app.world().get::<InMap>(unit) == Some(&map))
        .collect();
    assert_eq!(
        children.len(),
        14,
        "other copy also uses actual ordinary summons"
    );
    assert!(children.iter().all(|&unit| living(h, unit)));
    (leymor, map)
}

fn assert_removed_recipients(h: &Harness, leymor: Entity, recipients: &[Entity], before: usize) {
    for &unit in recipients {
        assert!(
            !is_recipient(h, unit, leymor),
            "actual e0 aura removed from {unit:?}"
        );
        assert!(
            !living(h, unit),
            "REAL removal must self-kill native recipient {unit:?}"
        );
        assert!(
            h.app.world().get::<crate::combat::Corpse>(unit).is_some(),
            "real death/corpse, not despawn"
        );
    }
    assert_eq!(
        stacks(h, leymor),
        before - recipients.len(),
        "existing death callback loses exactly one native owner stack per recipient"
    );
    assert!(
        h.health(h.player()).current > 0.0,
        "e1/aura4 removal must not self-kill the Player"
    );
}

#[test]
fn azure_stomp_removal_native_completion_self_kills_only_real_periodic_recipients() {
    let (mut h, leymor) = fixture();
    h.advance(28.0);
    let (other_leymor, other_map) = engage_other_copy(&mut h);
    let recipients = wait_for_stomp(&mut h, leymor);
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    let unselected: Vec<_> = h
        .creatures_of(SPROUT)
        .into_iter()
        .filter(|&unit| {
            h.app.world().get::<InMap>(unit) == Some(&map)
                && living(&h, unit)
                && !recipients.contains(&unit)
        })
        .collect();
    let other_children: Vec<_> = h
        .creatures_of(SPROUT)
        .into_iter()
        .filter(|&unit| h.app.world().get::<InMap>(unit) == Some(&other_map))
        .collect();
    assert_eq!(other_children.len(), 14);
    let other_before = stacks(&h, other_leymor);
    let before = stacks(&h, leymor);
    assert_eq!(before, recipients.len() + unselected.len());
    eprintln!(
        "Stomp actual recipient boundary: selected={} unselected={} owner_stacks={before}",
        recipients.len(),
        unselected.len()
    );
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(
        damage_count(&h, leymor),
        1,
        "existing actual channel-finish child remains once"
    );
    assert_removed_recipients(&h, leymor, &recipients, before);
    assert!(
        unselected.iter().all(|&unit| living(&h, unit)),
        "unselected same-copy Sprouts survive"
    );
    assert!(
        other_children.iter().all(|&unit| living(&h, unit)),
        "other-copy native Sprouts survive"
    );
    assert_eq!(stacks(&h, other_leymor), other_before);
    let after = stacks(&h, leymor);
    h.advance(0.2);
    assert_eq!(
        stacks(&h, leymor),
        after,
        "corpse/aura cleanup cannot decrement again"
    );
}

#[test]
fn azure_stomp_removal_native_interruption_also_self_kills_real_periodic_recipients() {
    let (mut h, leymor) = fixture();
    let recipients = wait_for_stomp(&mut h, leymor);
    let before = stacks(&h, leymor);
    assert!(h.app.world().get::<CastState>(leymor).is_some());
    // This is the real Spell::cancel path, not a manually removed/inserted aura.
    // It does not claim Stomp is interruptible by a particular player ability.
    crate::spell_cast::cancel_cast(h.app.world_mut(), leymor);
    h.advance(0.1);
    assert!(h.app.world().get::<CastState>(leymor).is_none());
    assert_eq!(
        damage_count(&h, leymor),
        0,
        "interruption does not fake successful completion"
    );
    assert_removed_recipients(&h, leymor, &recipients, before);
    let after = stacks(&h, leymor);
    h.advance(0.2);
    assert_eq!(stacks(&h, leymor), after);
}
