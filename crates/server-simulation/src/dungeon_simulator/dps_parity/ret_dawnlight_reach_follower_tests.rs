//! A Ret follower at 5 Holy Power spends beside an idle pack its spenders cannot
//! hit, and refuses a spender whose Divine Storm area or Sun's Avatar beam can.
use super::follower_dps_diagnostic::prepare_class_follower;
use super::*;
use shared::components::{Position, PowerType};
use shared::protocol::CombatLogKind;

const LEVEL: u8 = 90;
const TARASEK: u32 = 243983;
const DAWNLIGHT_TALENT: u32 = 431377;
const AVATAR_TALENT: u32 = 431425;
const DAWNLIGHT_CHARGE: u32 = 431522;
const DAWNLIGHT: u32 = 431380;
const AVATAR: u32 = 431911;
const VERDICT: u32 = 383328;
const STORM: u32 = 53385;
const SPEND_TICKS: usize = 60;

struct Fight {
    app: App,
    actor: Entity,
    boss: Entity,
}

/// A Herald Ret follower and a level-90 boss stand-in, as in the other Ret fixtures.
fn ret_beside_boss() -> Fight {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (app, actor, boss, _) = prepare_class_follower(app, copy, 70, LEVEL);
    let traits = app
        .world()
        .get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap();
    for talent in [DAWNLIGHT_TALENT, AVATAR_TALENT] {
        assert!(
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent),
            "the level-{LEVEL} Ret profile must select {talent}"
        );
    }
    Fight { app, actor, boss }
}

impl Fight {
    /// A point `distance` yards along the follower-to-boss line, `side` yards across it.
    fn along(&self, distance: f32, side: f32) -> Position {
        let world = self.app.world();
        let from = *world.get::<Position>(self.actor).unwrap();
        let to = *world.get::<Position>(self.boss).unwrap();
        let forward = Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalize();
        Position {
            x: from.x + forward.x * distance - forward.z * side,
            z: from.z + forward.z * distance + forward.x * side,
            ..from
        }
    }

    fn boss_distance(&self) -> f32 {
        let world = self.app.world();
        let from = *world.get::<Position>(self.actor).unwrap();
        let to = *world.get::<Position>(self.boss).unwrap();
        Vec3::new(to.x - from.x, 0.0, to.z - from.z).length()
    }

    /// An idle Sycophantic Tarasek, the never-engaged Voidscar trash beside Taz'Rah.
    fn idle_enemy(&mut self, at: Position) -> Entity {
        let unit = crate::zones::summon_creature(
            self.app.world_mut(),
            self.actor,
            TARASEK,
            [at.x, at.y, at.z, 0.0],
        )
        .expect("idle Tarasek spawns");
        assert!(crate::spell_cast::targets::relation_matches(
            self.app.world(),
            self.actor,
            unit,
            crate::spell_cast::targets::Relation::Enemy,
        ));
        unit
    }

    /// The owner pulls the boss from behind the follower, away from any pack
    /// beyond it; the follower targets the boss through its own brain.
    fn engage(&mut self) {
        let owner = self
            .app
            .world()
            .get::<crate::followers::Follower>(self.actor)
            .unwrap()
            .owner;
        let boss = self.boss;
        let behind = self.along(-8.0, 0.0);
        self.app.world_mut().entity_mut(owner).insert((
            behind,
            crate::networking::CombatTarget(Some(boss)),
            crate::combat::AutoAttack::new(boss),
        ));
        crate::combat_refs::set_in_combat_with(self.app.world_mut(), owner, boss);
    }

    fn set_holy_power(&mut self, amount: f32) {
        self.app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(self.actor)
            .unwrap()
            .get_mut(PowerType::HolyPower)
            .unwrap()
            .current = amount;
    }

    fn grant_dawnlight_charge(&mut self) {
        let actor = self.actor;
        crate::spell_cast::cast_triggered(self.app.world_mut(), actor, DAWNLIGHT_CHARGE, Some(actor), 1);
        assert!(crate::auras::has_aura(self.app.world(), actor, DAWNLIGHT_CHARGE));
    }

    fn safe(&self, spell: u32) -> bool {
        crate::followers::action_safe(self.app.world(), self.actor, self.boss, Some(spell))
    }

    /// The follower's successful casts over the next `ticks` frames.
    fn casts(&mut self, ticks: usize) -> Vec<u32> {
        let seen = self
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        for _ in 0..ticks {
            self.app.update();
            assert!(
                self.app
                    .world()
                    .get::<server_follower_apl::FollowerAplFailed>(self.actor)
                    .is_none()
            );
        }
        let actor = self.actor.to_bits();
        self.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[seen..]
            .iter()
            .filter(|event| event.source == Some(actor) && event.kind == CombatLogKind::CastSuccess)
            .filter_map(|event| event.spell_id)
            .collect()
    }

    /// Whether `unit` fights the follower or its owner, or took the follower's damage.
    /// Ret's Consecration stalker (43499) is a separate actor: it is not the party.
    fn party_engaged(&self, unit: Entity) -> bool {
        let world = self.app.world();
        let owner = world.get::<crate::followers::Follower>(self.actor).unwrap().owner;
        let damaged = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(self.actor.to_bits())
                    && event.target == Some(unit.to_bits())
                    && event.kind == CombatLogKind::Damage
            });
        damaged
            || crate::combat_refs::active_opponents(world, unit)
                .any(|opponent| opponent == self.actor || opponent == owner)
    }

    fn idle(&self, unit: Entity) -> bool {
        crate::combat_refs::active_opponents(self.app.world(), unit)
            .next()
            .is_none()
    }
}

/// Voidscar trace key: Taz'Rah's never-engaged trash stayed 26–30 yards from the boss
/// while Ret held 5 Holy Power and cast no spender.
fn spends_beside_idle_pack(charge: bool) {
    let mut fight = ret_beside_boss();
    let behind = fight.boss_distance();
    let pack: Vec<_> = [27.0, 31.0, 35.0]
        .into_iter()
        .map(|beyond| {
            let at = fight.along(behind + beyond, 0.0);
            fight.idle_enemy(at)
        })
        .collect();
    fight.engage();
    fight.set_holy_power(5.0);
    if charge {
        fight.grant_dawnlight_charge();
    }
    let casts = fight.casts(SPEND_TICKS);
    assert!(
        casts.iter().any(|spell| matches!(*spell, VERDICT | STORM)),
        "charge={charge}: Ret at 5 Holy Power must spend beside a pack 27–35 yards beyond the boss: {casts:?}"
    );
    let world = fight.app.world();
    if charge {
        assert!(
            world
                .get::<crate::auras::ActiveAuras>(fight.boss)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == DAWNLIGHT && aura.caster == fight.actor)),
            "the spent charge applies Dawnlight to the boss"
        );
        assert!(!crate::auras::has_aura(world, fight.actor, DAWNLIGHT_CHARGE));
    }
    for unit in pack {
        assert!(
            !fight.party_engaged(unit),
            "charge={charge}: the spender pulled {unit:?}: {casts:?}"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_ret_spends_beside_idle_pack_without_dawnlight_charge() {
    spends_beside_idle_pack(false);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_ret_spends_beside_idle_pack_with_dawnlight_charge() {
    spends_beside_idle_pack(true);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_ret_refuses_spenders_whose_area_or_beam_reaches_an_idle_enemy() {
    let mut fight = ret_beside_boss();
    let target = fight.boss;
    fight
        .app
        .world_mut()
        .entity_mut(fight.actor)
        .insert(crate::networking::CombatTarget(Some(target)));
    // Divine Storm's 8-yard caster area reaches an enemy 5 yards behind Ret.
    let behind = fight.along(-5.0, 0.0);
    let storm_victim = fight.idle_enemy(behind);
    assert!(!fight.safe(STORM), "Divine Storm would hit the idle enemy behind Ret");
    assert!(fight.safe(VERDICT), "single-target Final Verdict cannot reach it");
    fight.app.world_mut().despawn(storm_victim);
    // An enemy inside the Ret-to-boss line, beside the boss.
    let inside = fight.along(fight.boss_distance() - 0.5, 1.0);
    let beam_victim = fight.idle_enemy(inside);
    assert!(fight.safe(VERDICT), "without a charge Final Verdict applies no Dawnlight");
    fight.grant_dawnlight_charge();
    assert!(
        !fight.safe(VERDICT),
        "a charged Final Verdict's Dawnlight beams through the idle enemy"
    );
    // The real Sun's Avatar beam hits that enemy, but not one beyond the boss.
    let beyond = fight.along(fight.boss_distance() + 6.0, 0.0);
    let beyond = fight.idle_enemy(beyond);
    let actor = fight.actor;
    crate::spell_cast::cast_triggered(fight.app.world_mut(), actor, AVATAR, Some(target), 1);
    assert!(!fight.idle(beam_victim), "the beam must hit the enemy inside it");
    assert!(fight.idle(beyond), "the beam ends at its Dawnlight carrier");
}
