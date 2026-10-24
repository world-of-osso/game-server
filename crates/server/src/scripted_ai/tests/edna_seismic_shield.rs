//! TC a352b1fa spell_edna_seismic_reverberation, native70448 effects unchanged.
use super::*;

const REVERBERATION: u32 = 424889;
const SHIELD: u32 = 424893;
// Actual native magic dispel: effect38/ally21, no power rows. This is the
// dispel mechanism boundary, not player-class or spellbook admission proof.
const DISPEL: u32 = 17201;
const SMASH: u32 = 424888;

fn earned_reverberation(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map, golems) = source_copy(difficulty);
    unlock(&mut h, boss, map, &golems);
    h.engage(boss);
    h.advance(15.5);
    assert_eq!(
        h.app.world().get::<CastState>(boss).unwrap().spell_id,
        SMASH
    );
    {
        let _rng = crate::random::seed_scope(0);
        h.advance(4.1);
    }
    assert!(
        h.has_aura(h.player(), REVERBERATION),
        "real first native Smash earns its debuff"
    );
    assert!(!h.has_aura(h.player(), SHIELD));
    (h, boss, map)
}

fn last_smash(h: &Harness, boss: Entity) -> &shared::protocol::CombatLogEvent {
    h.inbox(0)
        .log
        .iter()
        .rev()
        .find(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(h.player().to_bits())
                && event.spell_id == Some(SMASH)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("genuine native Seismic damage, never an injected amount")
}

#[test]
#[ignore = "EDNA source enemy-dispel shield RED must execute before implementation"]
fn encounter_coverage_edna_seismic_native_dispel_shield_mitigation_expiry_and_copy() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = earned_reverberation(difficulty);
        let baseline = {
            let event = last_smash(&h, boss);
            (event.amount, event.crit)
        };
        // The source second Smash starts39.8s after Spike occupancy. Dispel
        // during its real4s cast so the native6s shield covers the actual hit.
        h.advance(20.4);
        assert_eq!(
            h.app.world().get::<CastState>(boss).unwrap().spell_id,
            SMASH
        );
        let player = h.player();
        assert!(h.has_aura(player, REVERBERATION));
        crate::spell_cast::creature_cast(h.app.world_mut(), player, DISPEL, Some(player), false)
            .expect("ordinary native magic dispel; no resource grants or bypass flags");
        h.advance(0.05);
        assert!(
            !h.has_aura(player, REVERBERATION),
            "actual native effect38 removes the earned debuff"
        );
        assert!(
            h.has_aura(player, SHIELD),
            "TC enemy-spell removal casts Stone Shield on its carrier"
        );
        assert!(!h.has_aura(h.players[1].0, SHIELD));
        assert!(h.inbox(1).spell_gos.iter().all(|go| go.spell_id != SHIELD));
        {
            let _rng = crate::random::seed_scope(0);
            h.advance(4.0);
        }
        let shielded = last_smash(&h, boss);
        assert_eq!(
            shielded.crit, baseline.1,
            "native comparison uses the same hit/crit draws"
        );
        // Native logs quantize damage to integers: -50% allows at most one
        // point between twice the shielded receipt and the unshielded receipt.
        assert!(
            (i64::from(shielded.amount) * 2 - i64::from(baseline.0)).abs() <= 1,
            "native -50% Stone Shield: baseline={baseline:?}, shielded={shielded:?}"
        );
        assert!(h.has_aura(player, SHIELD));
        h.advance(2.1);
        assert!(
            !h.has_aura(player, SHIELD),
            "native6000ms duration, not a fabricated timer"
        );
    }
}

#[test]
#[ignore = "EDNA source enemy-only removal controls execute with shield RED"]
fn encounter_coverage_edna_seismic_non_enemy_removals_do_not_grant_shield() {
    for boundary in ["expiry", "cancel", "death"] {
        let (mut h, boss, _) = earned_reverberation(1);
        let player = h.player();
        // Stop new source attacks through the existing real evade boundary;
        // do not shorten the earned debuff or fabricate removal reasons.
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(0.1);
        assert!(
            h.has_aura(player, REVERBERATION),
            "evade must leave the earned native debuff for this control"
        );
        match boundary {
            "expiry" => h.advance(30.1),
            "cancel" => {
                crate::auras::remove_auras(
                    h.app.world_mut(),
                    player,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == REVERBERATION,
                );
                h.advance(0.1);
            }
            "death" => {
                crate::combat::kill_unit(h.app.world_mut(), boss, player);
                h.advance(0.1);
            }
            _ => unreachable!(),
        }
        assert!(
            !h.has_aura(player, REVERBERATION),
            "actual {boundary} removal"
        );
        assert!(
            !h.has_aura(player, SHIELD),
            "{boundary} is not AURA_REMOVE_BY_ENEMY_SPELL"
        );
        assert!(h.inbox(0).spell_gos.iter().all(|go| go.spell_id != SHIELD));
    }
}
