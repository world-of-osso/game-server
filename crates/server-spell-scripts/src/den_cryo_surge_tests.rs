use crate::combat::auras;
use crate::test_support::{ObservedCombatEvents, harness, spawn_player};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const AURA: u32 = 1239860;
const DAMAGE: u32 = 1239871;

#[test]
fn cryo_surge_ticks_at_the_aura_holder_and_only_hits_nearby_players() {
    let mut h = harness(Vec::new(), &[AURA, DAMAGE], 1).unwrap();
    let caster = h.npc;
    let holder = h.caster;
    let nearby = spawn_player(&mut h.app, 1, 2.0);
    let distant = spawn_player(&mut h.app, 1, 20.0);
    for target in [holder, nearby, distant] {
        let mut health = h.app.world_mut().get_mut::<Health>(target).unwrap();
        health.max = 1_000_000.0;
        health.current = health.max;
    }
    h.app.init_resource::<ObservedCombatEvents>();
    h.advance(2);
    auras::apply_aura(
        h.app.world_mut(),
        &auras::AuraApplication {
            caster,
            target: holder,
            spell_id: AURA,
            effect_mask: 1,
            source: auras::AuraSource::Cast,
        },
    );
    h.advance(1);
    let count_hits = |h: &crate::test_support::Harness, target| {
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target)
                    && event.spell_id == Some(DAMAGE)
                    && event.kind == CombatLogKind::Damage
            })
            .count()
    };
    let mut hits = count_hits(&h, nearby.to_bits());
    assert_eq!(
        hits,
        1,
        "initial tick: aura={}, nearby health={}, holder health={}, log={:?}",
        auras::has_aura(h.app.world(), holder, AURA),
        h.app.world().get::<Health>(nearby).unwrap().current,
        h.app.world().get::<Health>(holder).unwrap().current,
        h.app.world().resource::<ObservedCombatEvents>().0
    );
    for _ in 0..9 {
        h.advance(18);
        assert_eq!(count_hits(&h, nearby.to_bits()), hits, "no early tick");
        h.advance(2);
        hits += 1;
        assert_eq!(
            count_hits(&h, nearby.to_bits()),
            hits,
            "one tick each second"
        );
    }
    assert_eq!(count_hits(&h, holder.to_bits()), hits);
    assert_eq!(count_hits(&h, distant.to_bits()), 0);
}
