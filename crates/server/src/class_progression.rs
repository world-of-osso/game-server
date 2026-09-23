//! Class progression: active specialization, known spells and action bars.
//! Rules and persistence: `docs/wiki/systems/class-progression.md`.

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::Message;
use lightyear::prelude::*;
use shared::components::{Level, Player, UnitLevel};
use shared::protocol::{
    ActionBarSnapshot, ActionRef, KnownSpellsSnapshot, SetActionButton, SetSpecialization,
    SpecializationChanged, SpellsLearned, SpellsUnlearned, TalentChannel,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};
use crate::persistence_progression::{
    ActionRefData, ProgressionData, load_progression, save_progression,
};
use crate::race_class::race_mask_allows;
use crate::spell_info::{ClassInfo, ClassSpells, SpecInfo};

/// Retail spec choice unlocks at level 10; below it the class "Initial" spec is active.
pub const SPEC_UNLOCK_LEVEL: u8 = 10;
/// `ChrSpecialization.OrderIndex` of the per-class "Initial" spec.
const INITIAL_SPEC_ORDER_INDEX: u32 = 4;
/// Action bar slots 0..=119.
pub const ACTION_BAR_SLOTS: u8 = 120;
const DEFAULT_BAR_SLOTS: usize = 12;
/// SkillLineAbilityAcquireMethod AutomaticSkillRank, AutomaticCharLevel and
/// LearnedOrAutomaticCharLevel: learned without a trainer.
const AUTO_LEARN_METHODS: [u32; 3] = [1, 2, 4];

/// Spells the player knows, in learn order. Cast validation reads this.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownSpells(Vec<u32>);

impl KnownSpells {
    pub fn contains(&self, spell_id: u32) -> bool {
        self.0.contains(&spell_id)
    }
}

/// Active `ChrSpecialization` ID; 0 when the class has no spec data.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSpec(pub u32);

/// Occupied action bar slots.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionBar(BTreeMap<u8, ActionRef>);

/// Integration point for the cast pipeline: reject casts of unknown spells.
pub fn knows_spell(known: &Query<&KnownSpells>, entity: Entity, spell_id: u32) -> bool {
    known
        .get(entity)
        .is_ok_and(|spells| spells.contains(spell_id))
}

pub struct ClassProgressionPlugin;

impl Plugin for ClassProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init_player_progression,
                process_set_specialization,
                process_set_action_button,
                sync_level_changes,
            )
                .chain(),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KnownSpell {
    spell_id: u32,
    passive: bool,
}

fn class_info(classes: &ClassSpells, class_id: u8) -> Option<&ClassInfo> {
    classes
        .classes
        .iter()
        .find(|class| class.class_id == class_id)
}

fn is_initial(spec: &SpecInfo) -> bool {
    spec.order_index == INITIAL_SPEC_ORDER_INDEX
}

/// Below `SPEC_UNLOCK_LEVEL`: the Initial spec. Otherwise the requested spec when
/// it is a non-Initial spec of the class, else the first non-Initial spec by OrderIndex.
fn resolve_spec(class: &ClassInfo, level: u8, requested: u32) -> Option<&SpecInfo> {
    if level < SPEC_UNLOCK_LEVEL {
        return class.specs.iter().find(|spec| is_initial(spec));
    }
    let mut chosen = class.specs.iter().filter(|spec| !is_initial(spec));
    chosen
        .clone()
        .find(|spec| spec.spec_id == requested)
        .or_else(|| chosen.next())
}

/// Auto-learned class spells up to `level` for the race, then the spec's spells
/// and mastery, without duplicates.
fn compute_known_spells(
    class: &ClassInfo,
    race: u8,
    level: u8,
    spec: Option<&SpecInfo>,
) -> Vec<KnownSpell> {
    let learnable = class
        .learnable
        .iter()
        .filter(|spell| {
            AUTO_LEARN_METHODS.contains(&spell.acquire_method)
                && spell.required_level <= u32::from(level)
                && race_mask_allows(spell.race_masks, race)
        })
        .map(|spell| KnownSpell {
            spell_id: spell.spell_id,
            passive: spell.passive,
        });
    let spec_spells = spec.into_iter().flat_map(|spec| {
        let spells = spec.spells.iter().map(|spell| KnownSpell {
            spell_id: spell.spell_id,
            passive: spell.passive,
        });
        let masteries = spec.mastery_spells.iter().map(|&spell_id| KnownSpell {
            spell_id,
            passive: true,
        });
        spells.chain(masteries)
    });
    let mut known: Vec<KnownSpell> = Vec::new();
    for spell in learnable.chain(spec_spells) {
        if !known.iter().any(|entry| entry.spell_id == spell.spell_id) {
            known.push(spell);
        }
    }
    known
}

/// Known non-passive spells in learn order on slots 0..12.
fn default_action_bar(known: &[KnownSpell]) -> BTreeMap<u8, ActionRef> {
    known
        .iter()
        .filter(|spell| !spell.passive)
        .take(DEFAULT_BAR_SLOTS)
        .enumerate()
        .map(|(slot, spell)| (slot as u8, ActionRef::Spell(spell.spell_id)))
        .collect()
}

/// Remove spell buttons for spells that are not known. Returns true when any was removed.
fn prune_unknown_spells(bar: &mut BTreeMap<u8, ActionRef>, known: &KnownSpells) -> bool {
    let before = bar.len();
    bar.retain(|_, action| match action {
        ActionRef::Spell(spell_id) => known.contains(*spell_id),
        ActionRef::Item(_) | ActionRef::Macro(_) => true,
    });
    bar.len() != before
}

fn spells_missing_from(spells: &[u32], other: &[u32]) -> Vec<u32> {
    spells
        .iter()
        .copied()
        .filter(|spell_id| !other.contains(spell_id))
        .collect()
}

fn action_to_data(action: ActionRef) -> ActionRefData {
    match action {
        ActionRef::Spell(id) => ActionRefData::Spell(id),
        ActionRef::Item(id) => ActionRefData::Item(id),
        ActionRef::Macro(id) => ActionRefData::Macro(id),
    }
}

fn action_from_data(action: ActionRefData) -> ActionRef {
    match action {
        ActionRefData::Spell(id) => ActionRef::Spell(id),
        ActionRefData::Item(id) => ActionRef::Item(id),
        ActionRefData::Macro(id) => ActionRef::Macro(id),
    }
}

fn progression_data(spec: ActiveSpec, bar: &ActionBar) -> ProgressionData {
    ProgressionData {
        spec_id: spec.0,
        action_bar: bar
            .0
            .iter()
            .map(|(&slot, &action)| (slot, action_to_data(action)))
            .collect(),
    }
}

fn bar_snapshot(bar: &ActionBar) -> ActionBarSnapshot {
    ActionBarSnapshot {
        slots: bar
            .0
            .iter()
            .map(|(&slot, &action)| (slot, action))
            .collect(),
    }
}

/// Known spells and spec for a player's class, race and level.
fn evaluate(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    requested_spec: u32,
) -> (ActiveSpec, Vec<KnownSpell>) {
    let Some(class) = class_info(classes, player.class) else {
        return (ActiveSpec(0), Vec::new());
    };
    let spec = resolve_spec(class, level, requested_spec);
    let known = compute_known_spells(class, player.race, level, spec);
    (ActiveSpec(spec.map_or(0, |spec| spec.spec_id)), known)
}

#[derive(SystemParam)]
struct ProgressionSenders<'w, 's> {
    known: Query<'w, 's, &'static mut MessageSender<KnownSpellsSnapshot>>,
    learned: Query<'w, 's, &'static mut MessageSender<SpellsLearned>>,
    unlearned: Query<'w, 's, &'static mut MessageSender<SpellsUnlearned>>,
    spec: Query<'w, 's, &'static mut MessageSender<SpecializationChanged>>,
    bar: Query<'w, 's, &'static mut MessageSender<ActionBarSnapshot>>,
}

fn send<M: Message>(senders: &mut Query<&mut MessageSender<M>>, connection: Entity, message: M) {
    if let Ok(mut sender) = senders.get_mut(connection) {
        sender.send::<TalentChannel>(message);
    }
}

type ConnectionLinks<'w, 's> = Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>;

/// Give newly spawned players their spec, known spells and action bar, and send
/// the owner `SpecializationChanged`, `KnownSpellsSnapshot` and `ActionBarSnapshot`.
fn init_player_progression(
    mut commands: Commands,
    db: Option<Res<GameDatabase>>,
    classes: Option<Res<ClassSpells>>,
    connections: ConnectionLinks,
    players: Query<(&Player, &Level, &ClientId), Without<KnownSpells>>,
    mut senders: ProgressionSenders,
) {
    let (Some(db), Some(classes)) = (db, classes) else {
        return;
    };
    for (connection, link) in &connections {
        let Ok((player, level, client_id)) = players.get(link.0) else {
            continue;
        };
        let stored = load_progression(&db.db, client_id.0);
        let requested = stored.as_ref().map_or(0, |data| data.spec_id);
        let (spec, known_list) = evaluate(&classes, player, level.0, requested);
        let known = KnownSpells(known_list.iter().map(|spell| spell.spell_id).collect());
        let mut bar = ActionBar(match &stored {
            Some(data) => data
                .action_bar
                .iter()
                .map(|&(slot, action)| (slot, action_from_data(action)))
                .collect(),
            None => default_action_bar(&known_list),
        });
        let pruned = prune_unknown_spells(&mut bar.0, &known);
        if stored.is_none() || pruned || requested != spec.0 {
            save_progression(&db.db, client_id.0, &progression_data(spec, &bar));
        }
        if spec.0 != 0 {
            send(
                &mut senders.spec,
                connection,
                SpecializationChanged { spec_id: spec.0 },
            );
        }
        send(
            &mut senders.known,
            connection,
            KnownSpellsSnapshot {
                spells: known.0.clone(),
            },
        );
        send(&mut senders.bar, connection, bar_snapshot(&bar));
        commands.entity(link.0).insert((spec, known, bar));
    }
}

type ProgressionPlayer = (
    &'static Player,
    &'static Level,
    &'static ClientId,
    &'static mut ActiveSpec,
    &'static mut KnownSpells,
    &'static mut ActionBar,
);

/// Re-evaluate spec and known spells after a level or spec change; send deltas,
/// drop bar buttons for unlearned spells and persist.
fn refresh_player(
    (player, level, client_id, mut spec, mut known, mut bar): (
        &Player,
        &Level,
        &ClientId,
        Mut<ActiveSpec>,
        Mut<KnownSpells>,
        Mut<ActionBar>,
    ),
    requested_spec: u32,
    classes: &ClassSpells,
    db: &GameDatabase,
    connection: Entity,
    senders: &mut ProgressionSenders,
) {
    let (new_spec, known_list) = evaluate(classes, player, level.0, requested_spec);
    let new_known = KnownSpells(known_list.iter().map(|spell| spell.spell_id).collect());
    let spec_changed = *spec != new_spec;
    if spec_changed {
        *spec = new_spec;
        send(
            &mut senders.spec,
            connection,
            SpecializationChanged {
                spec_id: new_spec.0,
            },
        );
    }
    let unlearned = spells_missing_from(&known.0, &new_known.0);
    let learned = spells_missing_from(&new_known.0, &known.0);
    if !unlearned.is_empty() {
        send(
            &mut senders.unlearned,
            connection,
            SpellsUnlearned { spells: unlearned },
        );
    }
    if !learned.is_empty() {
        send(
            &mut senders.learned,
            connection,
            SpellsLearned { spells: learned },
        );
    }
    let pruned = prune_unknown_spells(&mut bar.0, &new_known);
    if pruned {
        send(&mut senders.bar, connection, bar_snapshot(&bar));
    }
    if *known != new_known {
        *known = new_known;
    }
    if spec_changed || pruned {
        save_progression(&db.db, client_id.0, &progression_data(*spec, &bar));
    }
}

/// `SetSpecialization`: the spec must be a non-Initial spec of the player's class
/// and the player at least `SPEC_UNLOCK_LEVEL`. A rejected request is answered
/// with `SpecializationChanged` carrying the unchanged spec.
fn process_set_specialization(
    db: Option<Res<GameDatabase>>,
    classes: Option<Res<ClassSpells>>,
    mut receivers: Query<(Entity, &PlayerLink, &mut MessageReceiver<SetSpecialization>)>,
    mut players: Query<ProgressionPlayer>,
    mut senders: ProgressionSenders,
) {
    let (Some(db), Some(classes)) = (db, classes) else {
        return;
    };
    for (connection, link, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok(entry) = players.get_mut(link.0) else {
                continue;
            };
            if let Err(error) = validate_spec_request(&classes, entry.0, entry.1.0, request.spec_id)
            {
                warn!("Rejected SetSpecialization {}: {error}", request.spec_id);
                let current = entry.3.0;
                send(
                    &mut senders.spec,
                    connection,
                    SpecializationChanged { spec_id: current },
                );
                continue;
            }
            refresh_player(
                entry,
                request.spec_id,
                &classes,
                &db,
                connection,
                &mut senders,
            );
        }
    }
}

fn validate_spec_request(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    spec_id: u32,
) -> Result<(), String> {
    let spec = class_info(classes, player.class)
        .and_then(|class| class.specs.iter().find(|spec| spec.spec_id == spec_id))
        .filter(|spec| !is_initial(spec))
        .ok_or_else(|| {
            format!(
                "spec {spec_id} is not selectable for class {}",
                player.class
            )
        })?;
    if level < SPEC_UNLOCK_LEVEL {
        return Err(format!(
            "spec {} requires level {SPEC_UNLOCK_LEVEL}",
            spec.spec_id
        ));
    }
    Ok(())
}

/// Level changes re-evaluate spec (Initial below the unlock level) and known spells.
fn sync_level_changes(
    db: Option<Res<GameDatabase>>,
    classes: Option<Res<ClassSpells>>,
    connections: ConnectionLinks,
    mut players: Query<ProgressionPlayer, Changed<Level>>,
    mut senders: ProgressionSenders,
) {
    let (Some(db), Some(classes)) = (db, classes) else {
        return;
    };
    for (connection, link) in &connections {
        let Ok(entry) = players.get_mut(link.0) else {
            continue;
        };
        let requested = entry.3.0;
        refresh_player(entry, requested, &classes, &db, connection, &mut senders);
    }
}

/// `SetActionButton`: slot must be below `ACTION_BAR_SLOTS` and spell buttons need
/// a known spell. A rejected request is answered with the unchanged `ActionBarSnapshot`.
fn process_set_action_button(
    db: Option<Res<GameDatabase>>,
    mut receivers: Query<(Entity, &PlayerLink, &mut MessageReceiver<SetActionButton>)>,
    known_spells: Query<&KnownSpells>,
    mut players: Query<(&ClientId, &ActiveSpec, &mut ActionBar)>,
    mut bar_senders: Query<&mut MessageSender<ActionBarSnapshot>>,
) {
    let Some(db) = db else { return };
    for (connection, link, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok((client_id, spec, mut bar)) = players.get_mut(link.0) else {
                continue;
            };
            let is_known = |spell_id| knows_spell(&known_spells, link.0, spell_id);
            match apply_action_button(&mut bar, is_known, &request) {
                Ok(()) => save_progression(&db.db, client_id.0, &progression_data(*spec, &bar)),
                Err(error) => {
                    warn!("Rejected SetActionButton slot {}: {error}", request.slot);
                    send(&mut bar_senders, connection, bar_snapshot(&bar));
                }
            }
        }
    }
}

fn apply_action_button(
    bar: &mut ActionBar,
    is_known: impl Fn(u32) -> bool,
    request: &SetActionButton,
) -> Result<(), String> {
    if request.slot >= ACTION_BAR_SLOTS {
        return Err(format!("slot must be below {ACTION_BAR_SLOTS}"));
    }
    match request.action {
        Some(ActionRef::Spell(spell_id)) if !is_known(spell_id) => {
            Err(format!("spell {spell_id} is not known"))
        }
        Some(action) => {
            bar.0.insert(request.slot, action);
            Ok(())
        }
        None => {
            bar.0.remove(&request.slot);
            Ok(())
        }
    }
}

/// Apply an admin level change to the online player, if any. `Level` changes
/// drive `sync_level_changes`; `UnitLevel` follows when present.
pub(crate) fn set_live_level(world: &mut World, character_id: u64, level: u8) {
    let mut players = world.query::<(&ClientId, &mut Level, Option<&mut UnitLevel>)>();
    for (client_id, mut player_level, unit_level) in players.iter_mut(world) {
        if client_id.0 != character_id {
            continue;
        }
        player_level.0 = level;
        if let Some(mut unit_level) = unit_level {
            unit_level.0 = level;
        }
    }
}

#[cfg(test)]
#[path = "class_progression_tests.rs"]
mod tests;
