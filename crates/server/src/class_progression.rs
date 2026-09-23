//! Class progression: active specialization, trait (talent) config, known
//! spells and action bars. Rules and persistence:
//! `docs/wiki/systems/class-progression.md`, `docs/wiki/systems/talents.md`.

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::Message;
use lightyear::prelude::*;
use shared::components::{Level, Player, UnitLevel};
use shared::protocol::{
    ActionBarSnapshot, ActionRef, CommitTraitConfig, KnownSpellsSnapshot, SetActionButton,
    SetSpecialization, SpecializationChanged, SpellsLearned, SpellsUnlearned, TalentChannel,
    TraitCommitResult, TraitConfigSnapshot, TraitEntrySelection,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};
use crate::persistence_progression::{
    ActionRefData, ProgressionData, load_progression, load_trait_configs, save_progression,
    save_trait_config,
};
use crate::race_class::race_mask_allows;
use crate::spell_info::{ClassInfo, ClassSpells, SpecInfo};
use crate::trait_config::{
    ConfigEntry, Purchase, TraitContext, build_config, granted_entries, talent_spell_entries,
    unspent,
};

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
    #[cfg(test)]
    pub(crate) fn fixture(spells: Vec<u32>) -> Self {
        Self(spells)
    }

    pub fn contains(&self, spell_id: u32) -> bool {
        self.0.contains(&spell_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
}

/// Active `ChrSpecialization` ID; 0 when the class has no spec data.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSpec(pub u32);

/// Occupied action bar slots.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionBar(BTreeMap<u8, ActionRef>);

/// Validated trait config of the active spec (granted entries included),
/// its unspent points and the spells it applies.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitLoadout {
    tree_id: u32,
    entries: Vec<ConfigEntry>,
    unspent: Vec<(u32, i32)>,
    spells: Vec<KnownSpell>,
}

impl TraitLoadout {
    /// Integration point for the aura runtime: active and passive spells of
    /// the applied talents, in config order. Known spells include them.
    pub fn active_talent_spells(&self) -> impl Iterator<Item = KnownSpell> + '_ {
        self.spells.iter().copied()
    }

    fn purchases(&self) -> Vec<Purchase> {
        self.entries
            .iter()
            .filter(|entry| entry.ranks > 0)
            .map(|entry| Purchase {
                node_id: entry.node_id,
                entry_id: entry.entry_id,
                ranks: entry.ranks,
            })
            .collect()
    }
}

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
                process_commit_trait_config,
                process_set_action_button,
                sync_level_changes,
            )
                .chain(),
        );
    }
}

/// A known spell and its SPELL_ATTR0_PASSIVE flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownSpell {
    pub spell_id: u32,
    pub passive: bool,
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
/// and mastery, then talent spells, without duplicates.
fn compute_known_spells(
    class: &ClassInfo,
    race: u8,
    level: u8,
    spec: Option<&SpecInfo>,
    talents: impl Iterator<Item = KnownSpell>,
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
    for spell in learnable.chain(spec_spells).chain(talents) {
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

/// Progression state for a player's class, race, level and requested spec.
struct Evaluated {
    spec: ActiveSpec,
    known: Vec<KnownSpell>,
    traits: TraitLoadout,
    /// The saved purchases were invalid and were replaced by the granted-only config.
    traits_reset: bool,
}

/// Spec, trait config and known spells. `purchases_for` supplies the saved
/// trait purchases of the resolved spec.
fn evaluate(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    requested_spec: u32,
    purchases_for: impl FnOnce(u32) -> Vec<Purchase>,
) -> Evaluated {
    let Some(class) = class_info(classes, player.class) else {
        return Evaluated {
            spec: ActiveSpec(0),
            known: Vec::new(),
            traits: TraitLoadout::default(),
            traits_reset: false,
        };
    };
    let spec = resolve_spec(class, level, requested_spec);
    let ctx = TraitContext {
        spec_id: spec.map_or(0, |spec| spec.spec_id),
        level,
    };
    let (entries, traits_reset) =
        match build_config(&class.traits, ctx, &purchases_for(ctx.spec_id)) {
            Ok(entries) => (entries, false),
            Err(reason) => {
                warn!("Resetting trait config of spec {}: {reason}", ctx.spec_id);
                (granted_entries(&class.traits, ctx), true)
            }
        };
    let traits = TraitLoadout {
        tree_id: class.traits.tree_id,
        unspent: unspent(&class.traits, ctx, &entries),
        spells: talent_spell_entries(&class.traits, &entries)
            .map(|entry| KnownSpell {
                spell_id: entry.spell_id,
                passive: entry.passive,
            })
            .collect(),
        entries,
    };
    let known = compute_known_spells(
        class,
        player.race,
        level,
        spec,
        traits.active_talent_spells(),
    );
    Evaluated {
        spec: ActiveSpec(ctx.spec_id),
        known,
        traits,
        traits_reset,
    }
}

fn trait_snapshot(spec: ActiveSpec, traits: &TraitLoadout) -> TraitConfigSnapshot {
    TraitConfigSnapshot {
        spec_id: spec.0,
        tree_id: traits.tree_id,
        entries: traits
            .entries
            .iter()
            .map(|entry| TraitEntrySelection {
                node_id: entry.node_id,
                entry_id: entry.entry_id,
                rank: entry.total() as u8,
            })
            .collect(),
        unspent: traits.unspent.clone(),
    }
}

#[derive(SystemParam)]
struct ProgressionSenders<'w, 's> {
    known: Query<'w, 's, &'static mut MessageSender<KnownSpellsSnapshot>>,
    learned: Query<'w, 's, &'static mut MessageSender<SpellsLearned>>,
    unlearned: Query<'w, 's, &'static mut MessageSender<SpellsUnlearned>>,
    spec: Query<'w, 's, &'static mut MessageSender<SpecializationChanged>>,
    bar: Query<'w, 's, &'static mut MessageSender<ActionBarSnapshot>>,
    traits: Query<'w, 's, &'static mut MessageSender<TraitConfigSnapshot>>,
    commit: Query<'w, 's, &'static mut MessageSender<TraitCommitResult>>,
}

fn send<M: Message>(senders: &mut Query<&mut MessageSender<M>>, connection: Entity, message: M) {
    if let Ok(mut sender) = senders.get_mut(connection) {
        sender.send::<TalentChannel>(message);
    }
}

type ConnectionLinks<'w, 's> = Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>;

/// Give newly spawned players their spec, trait config, known spells and action
/// bar, and send the owner `SpecializationChanged`, `TraitConfigSnapshot`,
/// `KnownSpellsSnapshot` and `ActionBarSnapshot`.
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
        let configs = load_trait_configs(&db.db, client_id.0);
        let evaluated = evaluate(&classes, player, level.0, requested, |spec| {
            configs.spec(spec).to_vec()
        });
        let spec = evaluated.spec;
        if evaluated.traits_reset {
            save_trait_config(&db.db, client_id.0, spec.0, Vec::new());
        }
        let known = KnownSpells(evaluated.known.iter().map(|spell| spell.spell_id).collect());
        let mut bar = ActionBar(match &stored {
            Some(data) => data
                .action_bar
                .iter()
                .map(|&(slot, action)| (slot, action_from_data(action)))
                .collect(),
            None => default_action_bar(&evaluated.known),
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
            send(
                &mut senders.traits,
                connection,
                trait_snapshot(spec, &evaluated.traits),
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
        commands
            .entity(link.0)
            .insert((spec, known, bar, evaluated.traits));
    }
}

type ProgressionPlayer = (
    &'static Player,
    &'static Level,
    &'static ClientId,
    &'static mut ActiveSpec,
    &'static mut KnownSpells,
    &'static mut ActionBar,
    &'static mut TraitLoadout,
);

/// Apply a re-evaluated spec, trait config and known spells after a level, spec
/// or trait change; send deltas, drop bar buttons for unlearned spells and
/// persist. `TraitConfigSnapshot` is sent when the spec or config changed, or
/// always with `always_send_traits`.
fn refresh_player(
    (_, _, client_id, mut spec, mut known, mut bar, mut traits): (
        &Player,
        &Level,
        &ClientId,
        Mut<ActiveSpec>,
        Mut<KnownSpells>,
        Mut<ActionBar>,
        Mut<TraitLoadout>,
    ),
    evaluated: Evaluated,
    always_send_traits: bool,
    db: &GameDatabase,
    connection: Entity,
    senders: &mut ProgressionSenders,
) {
    let new_spec = evaluated.spec;
    let new_known = KnownSpells(evaluated.known.iter().map(|spell| spell.spell_id).collect());
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
    if evaluated.traits_reset {
        save_trait_config(&db.db, client_id.0, new_spec.0, Vec::new());
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
    let traits_changed = *traits != evaluated.traits;
    if traits_changed {
        *traits = evaluated.traits;
    }
    if new_spec.0 != 0 && (spec_changed || traits_changed || always_send_traits) {
        send(
            &mut senders.traits,
            connection,
            trait_snapshot(new_spec, &traits),
        );
    }
    if spec_changed || pruned {
        save_progression(&db.db, client_id.0, &progression_data(*spec, &bar));
    }
}

/// `SetSpecialization`: the spec must be a non-Initial spec of the player's class
/// and the player at least `SPEC_UNLOCK_LEVEL`. A rejected request is answered
/// with `SpecializationChanged` carrying the unchanged spec. An accepted one
/// loads the new spec's saved trait config.
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
            let configs = load_trait_configs(&db.db, entry.2.0);
            let evaluated = evaluate(&classes, entry.0, entry.1.0, request.spec_id, |spec| {
                configs.spec(spec).to_vec()
            });
            refresh_player(entry, evaluated, false, &db, connection, &mut senders);
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

/// `CommitTraitConfig`: the config must be for the active spec and valid at the
/// player's level (`trait_config::build_config`). An accepted config is
/// persisted and answered with `TraitCommitResult { ok: true }`, the spell
/// deltas and `TraitConfigSnapshot`. A rejected one is answered with
/// `ok: false` and the reason, and changes nothing.
fn process_commit_trait_config(
    db: Option<Res<GameDatabase>>,
    classes: Option<Res<ClassSpells>>,
    mut receivers: Query<(Entity, &PlayerLink, &mut MessageReceiver<CommitTraitConfig>)>,
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
            let spec_id = entry.3.0;
            let purchases = match commit_purchases(&classes, entry.0, entry.1.0, spec_id, &request)
            {
                Ok(purchases) => purchases,
                Err(reason) => {
                    warn!(
                        "Rejected CommitTraitConfig for spec {}: {reason}",
                        request.spec_id
                    );
                    let result = TraitCommitResult {
                        ok: false,
                        reason: Some(reason),
                    };
                    send(&mut senders.commit, connection, result);
                    continue;
                }
            };
            save_trait_config(&db.db, entry.2.0, spec_id, purchases.clone());
            let result = TraitCommitResult {
                ok: true,
                reason: None,
            };
            send(&mut senders.commit, connection, result);
            let evaluated = evaluate(&classes, entry.0, entry.1.0, spec_id, |_| purchases);
            refresh_player(entry, evaluated, true, &db, connection, &mut senders);
        }
    }
}

/// Client ranks include granted ranks; the purchases are the ranks above them.
fn commit_purchases(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    active_spec: u32,
    request: &CommitTraitConfig,
) -> Result<Vec<Purchase>, String> {
    if request.spec_id != active_spec {
        return Err(format!(
            "spec {} is not the active spec {active_spec}",
            request.spec_id
        ));
    }
    let class = class_info(classes, player.class)
        .ok_or_else(|| format!("class {} has no trait tree", player.class))?;
    let ctx = TraitContext {
        spec_id: active_spec,
        level,
    };
    let granted = granted_entries(&class.traits, ctx);
    let purchases: Vec<Purchase> = request
        .entries
        .iter()
        .map(|selection| {
            let free = granted
                .iter()
                .find(|g| g.node_id == selection.node_id && g.entry_id == selection.entry_id)
                .map_or(0, |g| g.granted);
            Purchase {
                node_id: selection.node_id,
                entry_id: selection.entry_id,
                ranks: u32::from(selection.rank).saturating_sub(free),
            }
        })
        .filter(|purchase| purchase.ranks > 0)
        .collect();
    build_config(&class.traits, ctx, &purchases)?;
    Ok(purchases)
}

/// Level changes re-evaluate spec (Initial below the unlock level), the trait
/// config against the new point budget, and known spells.
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
        let current_spec = entry.3.0;
        let evaluated = evaluate(&classes, entry.0, entry.1.0, current_spec, |spec| {
            if spec == current_spec {
                entry.6.purchases()
            } else {
                load_trait_configs(&db.db, entry.2.0).spec(spec).to_vec()
            }
        });
        refresh_player(entry, evaluated, false, &db, connection, &mut senders);
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
