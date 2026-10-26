//! Full native fights expose every incoming-damage scope on players, targets and observed allies.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_info::{SpellInfo, SpellStore};
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IncomingEffect {
    spell: u32,
    effect: u8,
    family: u32,
    mask: [u32; 4],
    amount_bits: u32,
    own_source: bool,
    recipient: &'static str,
}

#[derive(Default)]
struct Audit {
    effects: BTreeSet<IncomingEffect>,
    units: BTreeSet<u64>,
    events_read: usize,
}

impl Audit {
    fn observe(&mut self, run: &Run) {
        self.units.insert(run.actor.to_bits());
        self.units
            .extend(run.targets.iter().map(|unit| unit.to_bits()));
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        for event in &events[self.events_read..] {
            self.units
                .extend(event.source.into_iter().chain(event.target));
        }
        self.events_read = events.len();
        let units: Vec<_> = self.units.iter().copied().collect();
        for unit in units {
            self.capture_unit(run, unit);
        }
    }

    fn capture_unit(&mut self, run: &Run, bits: u64) {
        let Some(unit) = Entity::try_from_bits(bits) else {
            return;
        };
        let Some(active) = run.app.world().get::<ActiveAuras>(unit) else {
            return;
        };
        let recipient = if unit == run.actor {
            "player"
        } else if run.damage_targets.contains(&unit) {
            "enemy"
        } else {
            "ally_or_other"
        };
        for aura in active.iter() {
            for effect in aura.effects.iter().filter(|effect| effect.aura_type == 271) {
                self.effects.insert(IncomingEffect {
                    spell: aura.spell_id,
                    effect: effect.index,
                    family: aura.class_set,
                    mask: effect.class_mask,
                    amount_bits: effect.amount.to_bits(),
                    own_source: aura.caster == run.actor,
                    recipient,
                });
            }
        }
    }

    fn report(&self, run: &Run, count: usize) {
        let world = run.app.world();
        let events = &world.resource::<ObservedCombatEvents>().0;
        let applied = applied_incoming_spell_ids(run);
        let captured: BTreeSet<_> = self.effects.iter().map(|effect| effect.spell).collect();
        assert!(
            applied.is_subset(&captured),
            "uncaptured applied271 auras: {:?}",
            applied.difference(&captured).collect::<Vec<_>>()
        );
        assert!(
            captured.contains(&1299975),
            "real MID2 fights must exercise Burning Core"
        );
        for scope in &self.effects {
            print_scope(world, scope, count);
        }
        println!(
            "ENHANCEMENT_271_COMPLETE targets={count} seconds300 event_count={} observed_units={} captured_spell_ids={captured:?}",
            events.len(),
            self.units.len()
        );
    }
}

fn applied_incoming_spell_ids(run: &Run) -> BTreeSet<u32> {
    let world = run.app.world();
    let store = world.resource::<SpellStore>();
    world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::AuraApplied || event.kind == CombatLogKind::AuraRefreshed
        })
        .filter_map(|event| event.spell_id)
        .filter(|id| {
            store
                .get(*id)
                .is_some_and(|spell| spell.effects.iter().any(|effect| effect.aura == 271))
        })
        .collect()
}

fn print_scope(world: &World, scope: &IncomingEffect, count: usize) {
    let store = world.resource::<SpellStore>();
    let bolt = store.get(188196).unwrap();
    let nova = store.get(333977).unwrap();
    println!(
        "ENHANCEMENT_271 targets={count} spell={} name={:?} effect={} amount={} family={} mask={:x?} own_source={} recipient={} matches_bolt188196={} matches_nova333977={}",
        scope.spell,
        store.get(scope.spell).unwrap().name,
        scope.effect,
        f32::from_bits(scope.amount_bits),
        scope.family,
        scope.mask,
        scope.own_source,
        scope.recipient,
        matches_spell(scope, bolt),
        matches_spell(scope, nova)
    );
}

fn matches_spell(scope: &IncomingEffect, spell: &SpellInfo) -> bool {
    spell.class_options.as_ref().is_some_and(|class| {
        scope.family != 0
            && class.spell_class_set == scope.family
            && (0..4).any(|word| class.spell_class_mask[word] & scope.mask[word] != 0)
    })
}

#[test]
#[ignore = "requires native263 private inputs; complete300s ST and3T scope audit"]
fn enhancement_incoming_scope_audits_every_applied_271_effect_in_full_native_fights() {
    for count in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(263, count).unwrap();
        engage(&mut run);
        let mut audit = Audit::default();
        let (seconds, _, _) = advance_combat_observed(&mut run, |run| audit.observe(run)).unwrap();
        assert_eq!(seconds, 300.0);
        audit.report(&run, count);
    }
}
