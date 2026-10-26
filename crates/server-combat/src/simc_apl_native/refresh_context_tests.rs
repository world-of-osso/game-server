//! Fresh native duration/policy, never merged aura amount or raw empower child duration.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, world_db};
use shared::components::Health;

const REND: u32 = 388539;
const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;

fn fixture(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
fn native_target_context_refresh_policy_is_explicit_for_actual_owned_rend() {
    let mut h = harness(Vec::new(), &[REND], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: REND,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == REND && aura.caster == h.caster)
    );
    let targets = [h.npc];
    for (policy, expected) in [
        (DotRefreshPolicy::Pandemic, 0.0),
        (DotRefreshPolicy::FinalTick, 0.0),
        (DotRefreshPolicy::Always, 1.0),
        (DotRefreshPolicy::Never, 0.0),
    ] {
        let b = Bindings {
            buffs: [("rend_dot".into(), REND)].into(),
            dot_refresh: [(
                "rend_dot".into(),
                DotRefresh {
                    resolve: native_dot_spell,
                    policy,
                },
            )]
            .into(),
            ..Default::default()
        };
        let f = NativeFacts::new(h.app.world(), h.caster, &b, fixture(h.app.world()), 1)
            .with_targets(Some(h.npc), &targets);
        assert_eq!(f.value("dot.rend_dot.refreshable").unwrap(), expected);
    }
}

fn resolve_fire_stage_one(
    world: &World,
    caster: Entity,
    spell: u32,
) -> Result<crate::spell_info::SpellInfo, String> {
    let store = world.resource::<crate::spell_info::SpellStore>();
    if spell != FIRE_DAMAGE
        || store
            .get(FIRE_BREATH)
            .is_none_or(|root| root.empower_stages.is_empty())
    {
        return Err("native Fire Breath stage1 metadata unavailable".into());
    }
    crate::spell_cast::script_registry::empower_release_preview(world, caster, FIRE_BREATH, 1)
        .ok_or_else(|| "native Fire Breath release script unavailable".into())
}

fn release_native_fire_breath() -> Harness {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::protocol::{CombatChannel, ReleaseEmpowerSpell};
    let mut h = harness(Vec::new(), &[FIRE_BREATH], 13).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ReleaseEmpowerSpell>::default(),
        MessageReceiver::<ReleaseEmpowerSpell>::default(),
    ));
    h.cast_and_deliver(FIRE_BREATH, Some(h.caster));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<ReleaseEmpowerSpell>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(ReleaseEmpowerSpell {
            spell_id: FIRE_BREATH,
        });
    h.advance(25);
    h
}

#[test]
fn native_target_context_refresh_fire_breath_uses_resolved_stage_one_not_raw_two_seconds() {
    let mut h = release_native_fire_breath();
    let targets = [h.npc];
    let b = Bindings {
        buffs: [("fire_breath_damage".into(), FIRE_DAMAGE)].into(),
        dot_refresh: [(
            "fire_breath_damage".into(),
            DotRefresh {
                resolve: resolve_fire_stage_one,
                policy: DotRefreshPolicy::Pandemic,
            },
        )]
        .into(),
        ..Default::default()
    };
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == h.caster)
        .unwrap();
    assert_eq!(
        aura.duration_ms, 20_000,
        "actual native rank1 release, not raw357209 two-second metadata"
    );
    let expiry = aura.expires_at.unwrap();
    let duration = f64::from(aura.duration_ms) / 1000.0;
    let before = NativeFacts::new(h.app.world(), h.caster, &b, fixture(h.app.world()), 1)
        .with_targets(Some(h.npc), &targets);
    assert_eq!(
        before.value("dot.fire_breath_damage.refreshable").unwrap(),
        0.0
    );
    let step = h.app.world().resource::<Time>().delta_secs_f64();
    let remaining = expiry - crate::spell_cast::now(h.app.world());
    h.advance(((remaining - duration * 0.25) / step).ceil() as usize);
    assert!(crate::spell_cast::targets::is_alive(h.app.world(), h.npc));
    let after = NativeFacts::new(h.app.world(), h.caster, &b, fixture(h.app.world()), 1)
        .with_targets(Some(h.npc), &targets);
    assert!(after.value("dot.fire_breath_damage.remains").unwrap() > 0.0);
    assert_eq!(
        after.value("dot.fire_breath_damage.refreshable").unwrap(),
        1.0,
        "five seconds remaining is in native rank1 six-second Pandemic window, outside raw357209 0.6-second window"
    );
}
