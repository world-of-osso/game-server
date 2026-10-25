//! Full TC rule using installed spell/aura metadata; no changed costs, flags or masks.
use super::interruptibility::can_be_interrupted;
use super::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;
use bevy::prelude::Entity;

const IMMUNITY: u32 = 104773;
const PREVENT: u32 = 210303;
const ALLOW: u32 = 181827;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            133, 386781, 77382, 6770, 458, 698, 74867, ALLOW, 179202, PREVENT, 19750, IMMUNITY,
        ],
        8,
    )
    .unwrap()
}

fn apply_native_aura(h: &mut Harness, spell: u32, target: Entity) {
    let native = h.app.world().resource::<SpellStore>().get(spell).unwrap();
    let effect_mask = crate::auras::aura_effect_mask(native);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: spell,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), target, spell));
}

fn eligible(
    h: &Harness,
    spell: u32,
    interrupter: Option<Entity>,
    target: Entity,
    ignore: bool,
) -> bool {
    let spell = h.app.world().resource::<SpellStore>().get(spell).unwrap();
    can_be_interrupted(h.app.world(), spell, interrupter, target, ignore)
}

#[test]
fn native_interruptibility_rule_early_overrides_bypass_live_mechanic_immunity() {
    let mut h = fixture();
    let npc = h.npc;
    let player = h.caster;
    apply_native_aura(&mut h, IMMUNITY, npc);
    apply_native_aura(&mut h, IMMUNITY, player);
    assert!(!eligible(&h, 133, None, npc, false));
    assert!(
        !eligible(&h, 386781, None, npc, true),
        "no SILENCE remains ineligible even ignoring immunity"
    );
    // Native77382 attr7=0x2000;6770 DamageCancels=0x400;698 channelDamage;
    // 74867 has only channelEnteringCombat=0x10000000. Each bypasses immunity.
    for spell in [77382, 6770, 698, 74867] {
        assert!(
            eligible(&h, spell, None, npc, false),
            "native override {spell}"
        );
    }
    assert!(!eligible(&h, 458, None, npc, false));
    assert!(
        eligible(&h, 458, None, player, false),
        "native mount31flags includes player-only0x10"
    );
}

#[test]
fn native_interruptibility_rule_prevent_affect_mask_and_ignore_immunity_are_independent() {
    let mut h = fixture();
    let target = h.npc;
    assert!(eligible(&h, 19750, None, target, false));
    apply_native_aura(&mut h, PREVENT, target);
    assert!(
        !eligible(&h, 19750, None, target, false),
        "Divine Favor203 affects native Flash of Light"
    );
    assert!(
        !eligible(&h, 19750, None, target, true),
        "ignore immunity never bypasses203"
    );
    assert!(
        eligible(&h, 133, None, target, false),
        "foreign class mask does not protect Fireball"
    );
    apply_native_aura(&mut h, IMMUNITY, target);
    assert!(!eligible(&h, 133, None, target, false));
    assert!(eligible(&h, 133, None, target, true));
    // Intrinsic overrides are evaluated before the target protection checks.
    for spell in [77382, 6770, 698, 74867] {
        assert!(eligible(&h, spell, None, target, false));
    }
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PREVENT,
    );
    assert!(!eligible(&h, 19750, None, target, false));
    assert!(eligible(&h, 19750, None, target, true));
}

#[test]
fn native_interruptibility_rule_allow_aura_is_exact_spell_and_interrupter_owned() {
    let mut h = fixture();
    let target = h.npc;
    let kicker = h.caster;
    apply_native_aura(&mut h, IMMUNITY, target);
    assert!(!eligible(&h, 179202, Some(kicker), target, false));
    apply_native_aura(&mut h, ALLOW, kicker);
    assert!(
        eligible(&h, 179202, Some(kicker), target, false),
        "native484/Misc179202 overrides missing category and immunity"
    );
    assert!(!eligible(&h, 179202, None, target, false));
    assert!(!eligible(&h, 386781, Some(kicker), target, false));
    crate::auras::remove_auras(
        h.app.world_mut(),
        kicker,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ALLOW,
    );
    apply_native_aura(&mut h, ALLOW, target);
    assert!(
        !eligible(&h, 179202, Some(kicker), target, false),
        "target-owned484 does not authorize the kicker"
    );
}
