//! TC Unit.cpp:6980–6984: effect mechanic precedes spell mechanic for aura276.
use super::*;

const ARCANE_POWER: u32 = 374736;
const STOMP: u32 = 374720;
const DAMAGE: u32 = 374731;

fn native_stomp_damage(h: &Harness) -> Vec<CombatLogEvent> {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| event.spell_id == Some(DAMAGE))
        .filter(|event| event.kind == CombatLogKind::Damage)
        .cloned()
        .collect()
}

fn uncrit_damage(event: &CombatLogEvent) -> f64 {
    // Native school_damage's default critical factor is2; fixture has no163/crit mods.
    f64::from(event.amount) / if event.crit { 2.0 } else { 1.0 }
}

fn cast_native_brand_damage(h: &mut Harness, leymor: Entity) -> CombatLogEvent {
    const BRAND_CHILD: u32 = 374582;
    h.players[0].2.log.clear();
    crate::spell_cast::creature_cast(h.app.world_mut(), leymor, BRAND_CHILD, None, true)
        .expect("native nonmatching Brand damage child");
    h.advance(0.1);
    let damage: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.spell_id == Some(BRAND_CHILD) && event.kind == CombatLogKind::Damage)
        .cloned()
        .collect();
    assert_eq!(damage.len(), 1);
    damage[0].clone()
}

#[test]
fn azure_leymor_sprout_mechanic_power_does_not_amplify_nonmatching_brand() {
    const BRAND_CHILD: u32 = 374582;
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(BRAND_CHILD)
        .unwrap();
    assert_eq!(spell.categories.as_ref().unwrap().mechanic, 0);
    assert!(
        spell
            .effects
            .iter()
            .all(|effect| effect.mechanic == 0 && effect.variance == 0.0)
    );
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14
    );
    let buffed = cast_native_brand_damage(&mut h, leymor);
    crate::auras::remove_auras(
        h.app.world_mut(),
        leymor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ARCANE_POWER,
    );
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        0
    );
    let baseline = cast_native_brand_damage(&mut h, leymor);
    assert!(
        (uncrit_damage(&buffed) - uncrit_damage(&baseline)).abs() <= 1.0,
        "nonmatching mechanic must not gain ArcanePower: {buffed:?} vs {baseline:?}"
    );
}

#[test]
fn azure_leymor_sprout_mechanic_power_amplifies_ordinary_stomp_from_native_stacks() {
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let stack_boundary = |h: &Harness| {
        (
            crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
            children
                .iter()
                .filter(|&&child| crate::spell_cast::targets::is_alive(h.app.world(), child))
                .count(),
            children
                .iter()
                .filter(|&&child| h.app.world().get::<crate::combat::Corpse>(child).is_some())
                .count(),
        )
    };
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14
    );
    let power = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(leymor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == ARCANE_POWER)
        .unwrap();
    let effect = power
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!((effect.aura_type, effect.misc_values[0]), (276, 22));
    let amount_per_stack = f64::from(effect.amount);
    let damage_spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DAMAGE)
        .unwrap();
    assert_eq!(damage_spell.categories.as_ref().unwrap().mechanic, 22);
    assert_eq!(damage_spell.effects[0].mechanic, 0);
    assert_eq!(damage_spell.effects[0].variance, 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(leymor)
            .unwrap()
            .critical_damage(damage_spell.school_mask()),
        1.0
    );
    assert_eq!(
        crate::auras::spell_mods::modify(
            h.app.world(),
            leymor,
            damage_spell,
            crate::auras::SpellModOp::CritDamageAndHealing,
            1.0
        ),
        1.0
    );
    // Six fixture seconds plus0.1 then39.1 reaches the ordinary45.2s channel.
    h.advance(39.1);
    // Native Fissure/Brand deaths change the original fourteen-stack snapshot.
    // Sample the actual Stomp channel, without forcing any Sprout to survive.
    let (stacks_at_stomp, living_sprouts, corpses) = stack_boundary(&h);
    assert!(
        stacks_at_stomp > 0,
        "ordinary Stomp has native stacks to amplify it"
    );
    assert_eq!(stacks_at_stomp as usize, living_sprouts);
    assert_eq!(living_sprouts + corpses, children.len());
    let expected_scale = 1.0 + amount_per_stack * f64::from(stacks_at_stomp) / 100.0;
    eprintln!(
        "native276 before_stomp stacks/living/corpses={:?}",
        (stacks_at_stomp, living_sprouts, corpses)
    );
    let channel = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(channel.spell_id, STOMP);
    assert_eq!(channel.cast_type, shared::casting::CastType::Channel);
    assert!(native_stomp_damage(&h).is_empty());
    let remaining = channel.duration - channel.elapsed;
    h.advance(remaining + 0.2);
    eprintln!(
        "native276 after_stomp stacks/living/corpses={:?}",
        stack_boundary(&h)
    );
    let amplified = native_stomp_damage(&h);
    assert_eq!(amplified.len(), 1, "one real ordinary channel-finish child");
    assert_eq!(amplified[0].source, Some(leymor.to_bits()));
    assert_eq!(amplified[0].target, Some(h.player().to_bits()));
    // Remove only the actual buff for the native baseline child; no stat/amount edits.
    crate::auras::remove_auras(
        h.app.world_mut(),
        leymor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ARCANE_POWER,
    );
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        0
    );
    h.players[0].2.log.clear();
    crate::spell_cast::creature_cast(h.app.world_mut(), leymor, DAMAGE, None, true)
        .expect("native baseline Stomp damage child");
    h.advance(0.1);
    let baseline = native_stomp_damage(&h);
    assert_eq!(baseline.len(), 1);
    let actual = uncrit_damage(&amplified[0]);
    let expected = uncrit_damage(&baseline[0]) * expected_scale;
    let rounding_bound = expected_scale + 1.0;
    assert!(
        (actual - expected).abs() <= rounding_bound,
        "native276 stack multiplier{expected_scale}: actual{actual}, expected{expected}; amplified={:?}, baseline={:?}",
        amplified[0],
        baseline[0]
    );
}
