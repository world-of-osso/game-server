//! Locally sourced PvpTalent/Category/SlotUnlock. TC Player.cpp:27690–27814.
use crate::auth::{Authenticated, InWorld};
use crate::class_progression::{ActionBar, KnownSpells};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};
use crate::pvp::{PlayerPvpProgress, PlayerPvpState, pvp_support};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::components::{ActiveSpec, CombatStatus, Health, Level, Player};
use shared::protocol::{
    ActionBarSnapshot, PvpChannel, PvpStateUpdate, PvpTalentSlotSnapshot, PvpTalentSnapshot,
    PvpTalentsSnapshot, SelectPvpTalent, SpellsLearned, SpellsUnlearned, TalentChannel,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Talent {
    spec: u32,
    spell: u32,
    override_spell: u32,
    category: u32,
    level: u8,
    condition: u32,
}
#[derive(Resource, Default)]
pub(crate) struct Catalog {
    talents: BTreeMap<u32, Talent>,
    categories: BTreeMap<u32, u8>,
    slots: BTreeMap<u8, [u8; 3]>,
}
#[derive(Clone, Copy)]
struct SelectionContext {
    spec: u32,
    class: u8,
    level: u8,
    alive: bool,
    in_combat: bool,
    can_swap: bool,
    old_on_cooldown: bool,
}
impl Catalog {
    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut talents = conn.prepare("SELECT id,spec_id,spell_id,override_spell,category,level_required,player_condition FROM pvp_talent")?;
        let talents = talents
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    Talent {
                        spec: r.get(1)?,
                        spell: r.get(2)?,
                        override_spell: r.get(3)?,
                        category: r.get(4)?,
                        level: r.get(5)?,
                        condition: r.get(6)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut categories = conn.prepare("SELECT id,slot_mask FROM pvp_talent_category")?;
        let categories = categories
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut slots = conn.prepare("SELECT slot_mask,level_required,death_knight_level,demon_hunter_level FROM pvp_talent_slot_unlock")?;
        let slots = slots
            .query_map([], |r| Ok((r.get(0)?, [r.get(1)?, r.get(2)?, r.get(3)?])))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self {
            talents,
            categories,
            slots,
        })
    }
    fn required_level(&self, slot: u8, class: u8) -> Option<u8> {
        let levels = self.slots.get(&(1 << slot))?;
        Some(
            levels[match class {
                6 => 1,
                12 => 2,
                _ => 0,
            }],
        )
    }
    fn validate(&self, context: SelectionContext, slot: u8, id: u32) -> Result<&Talent, String> {
        if !context.alive || context.in_combat {
            return Err("PvP talents cannot change while dead or in combat".into());
        }
        self.check_eligibility(context, slot, id)
    }
    fn check_eligibility(
        &self,
        context: SelectionContext,
        slot: u8,
        id: u32,
    ) -> Result<&Talent, String> {
        if !(1..=3).contains(&slot) {
            return Err("Unknown Mainline PvP talent slot".into());
        }
        let talent = self.talents.get(&id).ok_or("Unknown PvP talent")?;
        if talent.spec != context.spec {
            return Err("PvP talent belongs to another specialization".into());
        }
        let required = self
            .required_level(slot, context.class)
            .ok_or("PvP slot unlock source unavailable")?;
        if context.level < required.max(talent.level) {
            return Err("PvP talent slot or choice is level locked".into());
        }
        if !self
            .categories
            .get(&talent.category)
            .is_some_and(|mask| mask & (1 << slot) != 0)
        {
            return Err("PvP talent is not allowed in this slot".into());
        }
        if talent.condition != 0 {
            return Err(format!(
                "PlayerCondition{} cannot yet be evaluated",
                talent.condition
            ));
        }
        Ok(talent)
    }
    fn select(
        &self,
        progress: &mut PlayerPvpProgress,
        context: SelectionContext,
        slot: u8,
        id: u32,
    ) -> Result<(), String> {
        self.validate(context, slot, id)?;
        let selected = progress.pvp_talents.entry(context.spec).or_default();
        if selected.values().any(|&chosen| chosen == id) {
            return Err("PvP talent is already selected".into());
        }
        if selected.contains_key(&slot) && !context.can_swap {
            return Err("Replacing a PvP talent requires a rest area or talent-swap aura".into());
        }
        if context.old_on_cooldown {
            return Err("Selected PvP talent is on cooldown".into());
        }
        selected.insert(slot, id);
        Ok(())
    }
}
fn read_context(world: &World, owner: Entity, slot: u8) -> Result<SelectionContext, String> {
    let spec = world
        .get::<ActiveSpec>(owner)
        .ok_or("Player specialization unavailable")?
        .0;
    let player = world
        .get::<Player>(owner)
        .ok_or("Player identity unavailable")?;
    let level = world
        .get::<Level>(owner)
        .ok_or("Player level unavailable")?
        .0;
    let alive = world
        .get::<Health>(owner)
        .ok_or("Player health unavailable")?
        .current
        > 0.0;
    let in_combat = world
        .get::<CombatStatus>(owner)
        .ok_or("Player combat state unavailable")?
        .0;
    let resting = world
        .get::<crate::rest::PlayerRestState>(owner)
        .is_some_and(|rest| rest.is_resting());
    let can_swap = resting
        || world
            .get::<crate::auras::ActiveAuras>(owner)
            .is_some_and(|auras| auras.has_type(crate::auras::handlers::ALLOW_TALENT_SWAPPING));
    let old = world
        .get::<PlayerPvpState>(owner)
        .and_then(|s| s.progress.pvp_talents.get(&spec))
        .and_then(|s| s.get(&slot));
    let old_spell = old
        .and_then(|id| world.resource::<Catalog>().talents.get(id))
        .and_then(|t| world.resource::<SpellStore>().get(t.spell));
    let now = crate::spell_cast::history_now(world);
    let old_on_cooldown = old_spell.is_some_and(|spell| {
        world
            .get::<crate::spell_cast::SpellHistory>(owner)
            .is_some_and(|history| history.on_cooldown(spell, now))
    });
    Ok(SelectionContext {
        spec,
        class: player.class,
        level,
        alive,
        in_combat,
        can_swap,
        old_on_cooldown,
    })
}
pub(super) fn receive_requests(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<SelectPvpTalent>,
    ), With<Authenticated>>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, in_world, mut r)| {
            r.receive()
                .map(|request| (connection, link.0, in_world.character_id, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, character, request) in requests {
        let result = select_and_persist(world, connection, owner, character, request);
        send_state(world, connection, owner, result.err());
    }
}
fn select_and_persist(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    character: u64,
    request: SelectPvpTalent,
) -> Result<(), String> {
    let catalog = world
        .get_resource::<Catalog>()
        .ok_or("PvP catalog unavailable; run import-pvp-progression.py")?;
    let context = read_context(world, owner, request.slot)?;
    let talent = catalog.validate(context, request.slot, request.talent_id)?;
    if world.resource::<SpellStore>().get(talent.spell).is_none() {
        return Err("PvP talent spell source is unavailable".into());
    }
    let mut progress = world
        .get::<PlayerPvpState>(owner)
        .ok_or("PvP state unavailable")?
        .progress
        .clone();
    catalog.select(&mut progress, context, request.slot, request.talent_id)?;
    persistence::save_pvp(&world.resource::<GameDatabase>().db, character, &progress);
    world
        .get_mut::<PlayerPvpState>(owner)
        .expect("validated PvP state")
        .progress = progress;
    sync_player(world, connection, owner);
    Ok(())
}
fn send_state(world: &mut World, connection: Entity, owner: Entity, error: Option<String>) {
    let state = world.get::<PlayerPvpState>(owner).cloned();
    if let Some(mut sender) = world.get_mut::<MessageSender<PvpStateUpdate>>(connection) {
        sender.send::<PvpChannel>(PvpStateUpdate {
            snapshot: state.as_ref().map(pvp_support::build_pvp_snapshot),
            message: None,
            error,
        });
    }
}
fn project(world: &World, owner: Entity) -> Option<(PvpTalentsSnapshot, Vec<(u32, u32)>)> {
    let catalog = world.get_resource::<Catalog>()?;
    let context = read_context(world, owner, 1).ok()?;
    let state = world.get::<PlayerPvpState>(owner)?;
    let selected = state.progress.pvp_talents.get(&context.spec);
    let slots = (1..=3)
        .filter_map(|index| {
            let required_level = catalog.required_level(index, context.class)?;
            let selected = selected.and_then(|s| s.get(&index)).copied();
            let can_change = context.alive
                && !context.in_combat
                && context.level >= required_level
                && (selected.is_none() || context.can_swap);
            Some(PvpTalentSlotSnapshot {
                index,
                required_level,
                selected,
                can_change,
                unlocked: context.level >= required_level,
            })
        })
        .collect();
    let store = world.resource::<SpellStore>();
    let choices = catalog
        .talents
        .iter()
        .filter(|(_, t)| t.spec == context.spec)
        .map(|(&id, t)| {
            let spell = store.get(t.spell);
            let unavailable_reason = if t.condition != 0 {
                Some(format!(
                    "PlayerCondition{} is not yet evaluated",
                    t.condition
                ))
            } else if context.level < t.level {
                Some(format!("Requires level{}", t.level))
            } else if spell.is_none() {
                Some("Spell source unavailable".into())
            } else {
                None
            };
            PvpTalentSnapshot {
                id,
                spell_id: t.spell,
                name: spell.map_or_else(String::new, |s| s.name.clone()),
                icon: spell
                    .and_then(|s| s.misc.as_ref())
                    .map_or(0, |m| m.icon_file_data_id),
                required_level: t.level,
                slot_mask: catalog.categories[&t.category],
                unavailable_reason,
            }
        })
        .collect();
    let map = world.get::<crate::maps::InMap>(owner)?;
    // TC Player::ArePvpTalentsEnabled: actual SPELL_AURA_PVP_TALENTS (119).
    let active = world
        .get::<crate::auras::ActiveAuras>(owner)
        .is_some_and(|auras| auras.has_type(119))
        || state
            .battleground
            .as_ref()
            .is_some_and(|bg| bg.accepted && bg.map_id == map.map_id && map.instance_id != 0);
    let grants = if active {
        selected
            .into_iter()
            .flat_map(|s| s.iter())
            .filter_map(|(&slot, &id)| catalog.check_eligibility(context, slot, id).ok())
            .map(|t| (t.spell, t.override_spell))
            .collect()
    } else {
        Vec::new()
    };
    Some((
        PvpTalentsSnapshot {
            spec_id: context.spec,
            slots,
            choices,
        },
        grants,
    ))
}
pub(super) fn sync_players(world: &mut World) {
    if !world.contains_resource::<Catalog>() {
        return;
    }
    let mut query = world.query_filtered::<(Entity, &PlayerLink), With<Authenticated>>();
    let players: Vec<_> = query
        .iter(world)
        .map(|(connection, link)| (connection, link.0))
        .collect();
    for (connection, owner) in players {
        sync_player(world, connection, owner);
    }
}
fn sync_player(world: &mut World, connection: Entity, owner: Entity) {
    let Some((snapshot, mut grants)) = project(world, owner) else {
        return;
    };
    grants.sort_unstable();
    grants.dedup();
    let Some(known) = world.get::<KnownSpells>(owner) else {
        return;
    };
    let previous = known.pvp_grants().to_vec();
    let snapshot_changed = world
        .get::<PlayerPvpState>(owner)
        .is_some_and(|s| s.talents.as_ref() != Some(&snapshot));
    if previous != grants {
        update_grants(world, connection, owner, &previous, grants);
    }
    if snapshot_changed {
        world
            .get_mut::<PlayerPvpState>(owner)
            .expect("projected PvP owner")
            .talents = Some(snapshot);
        send_state(world, connection, owner, None);
    }
}
fn update_grants(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    previous: &[(u32, u32)],
    grants: Vec<(u32, u32)>,
) {
    let before: BTreeSet<_> = world.get::<KnownSpells>(owner).unwrap().iter().collect();
    world
        .get_mut::<KnownSpells>(owner)
        .unwrap()
        .set_pvp_grants(grants.clone());
    let known = world.get::<KnownSpells>(owner).unwrap().clone();
    let after: BTreeSet<_> = known.iter().collect();
    if let Some(mut sender) = world.get_mut::<MessageSender<SpellsLearned>>(connection) {
        sender.send::<TalentChannel>(SpellsLearned {
            spells: after.difference(&before).copied().collect(),
        });
    }
    if let Some(mut sender) = world.get_mut::<MessageSender<SpellsUnlearned>>(connection) {
        sender.send::<TalentChannel>(SpellsUnlearned {
            spells: before.difference(&after).copied().collect(),
        });
    }
    let bar = world
        .get_mut::<ActionBar>(owner)
        .and_then(|mut bar| bar.update_pvp_overrides(previous, &grants, &known));
    if let Some(bar) = bar
        && let Some(mut sender) = world.get_mut::<MessageSender<ActionBarSnapshot>>(connection)
    {
        sender.send::<TalentChannel>(bar);
    }
}
#[cfg(any(test, feature = "test-support"))]
impl Catalog {
    fn fixture() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE pvp_talent(id INTEGER,spec_id INTEGER,spell_id INTEGER,override_spell INTEGER,category INTEGER,level_required INTEGER,player_condition INTEGER);
            INSERT INTO pvp_talent VALUES (752,70,210256,0,2,0,0),(5573,70,204018,0,2,25,0),(5572,70,199456,0,2,25,103471);
            CREATE TABLE pvp_talent_category(id INTEGER,slot_mask INTEGER);INSERT INTO pvp_talent_category VALUES(2,15);
            CREATE TABLE pvp_talent_slot_unlock(slot_mask INTEGER,level_required INTEGER,death_knight_level INTEGER,demon_hunter_level INTEGER);
            INSERT INTO pvp_talent_slot_unlock VALUES(1,10,10,10),(2,20,20,20),(4,30,30,30),(8,40,40,40);").unwrap();
        Self::load(&conn).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_ret_talent_slots_validate_and_survive_reopen() {
        let catalog = Catalog::fixture();
        let mut progress = crate::pvp::PlayerPvpProgress::default();
        let context = SelectionContext {
            spec: 70,
            class: 2,
            level: 90,
            alive: true,
            in_combat: false,
            can_swap: true,
            old_on_cooldown: false,
        };
        catalog.select(&mut progress, context, 1, 752).unwrap();
        assert!(catalog.select(&mut progress, context, 2, 752).is_err());
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        spec: 73,
                        ..context
                    },
                    2,
                    752
                )
                .is_err()
        );
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        level: 19,
                        ..context
                    },
                    1,
                    752
                )
                .is_err()
        );
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        in_combat: true,
                        ..context
                    },
                    1,
                    5573
                )
                .is_err()
        );
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        alive: false,
                        ..context
                    },
                    1,
                    5573
                )
                .is_err()
        );
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        can_swap: false,
                        ..context
                    },
                    1,
                    5573
                )
                .is_err()
        );
        assert!(
            catalog
                .select(
                    &mut progress,
                    SelectionContext {
                        old_on_cooldown: true,
                        ..context
                    },
                    1,
                    5573
                )
                .is_err()
        );
        assert!(
            catalog.select(&mut progress, context, 1, 5572).is_err(),
            "underived PlayerCondition103471 is not guessed true"
        );
        catalog.select(&mut progress, context, 1, 5573).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pvp.redb");
        {
            let db = redb::Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            crate::persistence::save_pvp(&db, 41, &progress);
        }
        let db = redb::Database::open(&path).unwrap();
        let loaded = crate::persistence::load_pvp(&db, 41);
        assert_eq!(loaded.pvp_talents[&70][&1], 5573);
    }
    #[test]
    fn arena_preparation_allows_actual_pvp_talent_replacement_only_while_aura_remains() {
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[32727, 210256], 2).unwrap();
        h.app.insert_resource(Catalog::fixture());
        let mut state = PlayerPvpState::default();
        state
            .progress
            .pvp_talents
            .insert(70, [(1, 752)].into_iter().collect());
        h.app.world_mut().entity_mut(h.caster).insert((
            state,
            ActiveSpec(70),
            Level(90),
            CombatStatus(false),
        ));
        let replace = |h: &mut crate::spell_cast::test_harness::Harness| {
            let context = read_context(h.app.world(), h.caster, 1).unwrap();
            let mut progress = h
                .app
                .world()
                .get::<PlayerPvpState>(h.caster)
                .unwrap()
                .progress
                .clone();
            let result =
                h.app
                    .world()
                    .resource::<Catalog>()
                    .select(&mut progress, context, 1, 5573);
            (result, progress)
        };
        assert!(replace(&mut h).0.is_err(), "outside a rest area");
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 32727,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let (result, progress) = replace(&mut h);
        result.unwrap();
        assert_eq!(progress.pvp_talents[&70][&1], 5573);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(CombatStatus(true));
        assert!(
            replace(&mut h).0.is_err(),
            "preparation never permits combat talent swaps"
        );
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(CombatStatus(false));
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Expired,
            |aura| aura.spell_id == 32727,
        );
        assert!(
            replace(&mut h).0.is_err(),
            "gate opening removes permission"
        );
    }
    #[test]
    fn temporary_pvp_grants_do_not_remove_class_owned_spells() {
        let mut known = crate::class_progression::KnownSpells::fixture(vec![204018, 853]);
        known.set_pvp_grants(vec![(204018, 0), (210256, 0)]);
        assert!(known.contains(210256));
        known.set_pvp_grants(Vec::new());
        assert!(!known.contains(210256));
        assert!(known.contains(204018));
        assert!(known.contains(853));
        known.set_pvp_grants(vec![(210256, 853)]);
        assert!(
            !known.contains(853),
            "active override suppresses only its baseline"
        );
        known.set_pvp_grants(Vec::new());
        assert!(
            known.contains(853),
            "baseline ownership returns when PvP activation ends"
        );
    }
    #[test]
    fn pvp_grants_follow_actual_aura_not_cached_war_mode_state() {
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[210256, 282559], 2).unwrap();
        h.app.insert_resource(Catalog::fixture());
        let mut state = PlayerPvpState::default();
        state
            .progress
            .pvp_talents
            .insert(70, [(1, 752)].into_iter().collect());
        state.war_mode = Some(shared::protocol::WarModeSnapshot {
            desired: true,
            active: false,
            can_enable: true,
            can_disable: true,
            enlisted_spell: Some(282559),
        });
        h.app.world_mut().entity_mut(h.caster).insert((
            state,
            ActiveSpec(70),
            Level(90),
            crate::maps::InMap::continent(0),
            CombatStatus(false),
        ));
        assert!(
            project(h.app.world(), h.caster).unwrap().1.is_empty(),
            "cached Enlisted identity is not an actual aura"
        );
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 282559,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert_eq!(
            project(h.app.world(), h.caster).unwrap().1,
            vec![(210256, 0)]
        );
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Unlearned,
            |a| a.spell_id == 282559,
        );
        assert!(project(h.app.world(), h.caster).unwrap().1.is_empty());
    }
    #[test]
    fn active_pvp_talent_grants_survive_entering_combat() {
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[853, 210256, 282559], 2)
            .unwrap();
        h.app.insert_resource(Catalog::fixture());
        let mut state = PlayerPvpState::default();
        state
            .progress
            .pvp_talents
            .insert(70, [(1, 752)].into_iter().collect());
        state.war_mode = Some(shared::protocol::WarModeSnapshot {
            desired: true,
            active: false,
            can_enable: true,
            can_disable: true,
            enlisted_spell: Some(282559),
        });
        h.app.world_mut().entity_mut(h.caster).insert((
            state,
            ActiveSpec(70),
            Level(90),
            crate::maps::InMap::continent(0),
            CombatStatus(false),
        ));
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 282559,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let (_, before) = project(h.app.world(), h.caster).unwrap();
        assert_eq!(before, vec![(210256, 0)]);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(CombatStatus(true));
        let (_, during) = project(h.app.world(), h.caster).unwrap();
        assert_eq!(
            during, before,
            "Learning restrictions must not deactivate selected PvP talents during combat"
        );
    }
}
