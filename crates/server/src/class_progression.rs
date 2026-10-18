//! Class progression: active specialization, trait (talent) config, known
//! spells and action bars. Rules and persistence:
//! `docs/wiki/systems/class-progression.md`, `docs/wiki/systems/talents.md`.

use std::collections::{BTreeMap, BTreeSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::Message;
use lightyear::prelude::*;
use shared::components::{Level, Player, UnitLevel};
use shared::protocol::{
    ActionBarSnapshot, ActionRef, CommitTraitConfig, KnownSpellsSnapshot, STARTER_BUILD_LOADOUT_ID,
    SetActionButton, SetSpecialization, SpecializationChanged, SpellsLearned, SpellsUnlearned,
    TalentChannel, TraitCommitResult, TraitConfigSnapshot, TraitEntrySelection, TraitLoadoutInfo,
    TraitLoadoutOperation, TraitLoadoutRequest, TraitLoadoutsSnapshot,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};
use crate::persistence_progression::{
    ActionRefData, ProgressionData, TraitConfigsData, load_progression, load_trait_configs,
    save_progression, save_trait_config, save_trait_configs,
};
use crate::profession::PlayerProfessions;
use crate::race_class::race_mask_allows;
use crate::spell_info::{ClassInfo, ClassSpells, SpecInfo};
use crate::trait_config::{
    ConfigEntry, Purchase, TraitContext, build_config, granted_entries, starter_purchases,
    talent_spell_entries, unspent,
};

/// Retail spec choice unlocks at level 10; below it the class "Initial" spec is active.
pub const SPEC_UNLOCK_LEVEL: u8 = 10;
/// `ChrSpecialization.OrderIndex` of the per-class "Initial" spec.
const INITIAL_SPEC_ORDER_INDEX: u32 = 4;
/// Action bar slots 0..=179 (TrinityCore Player.h `MAX_ACTION_BUTTONS` 180): the six
/// pages, the bonus bars 1..=5 (slots 72..=131) and the override pages after them.
pub const ACTION_BAR_SLOTS: u8 = 180;
/// Main action bar slots 0..=11, where learned spells are pushed.
const MAIN_BAR_SLOTS: u8 = 12;
/// SkillLineAbilityAcquireMethod AutomaticSkillRank, AutomaticCharLevel and
/// LearnedOrAutomaticCharLevel: learned without a trainer (DBCEnums.h:2376-2382).
const AUTO_LEARN_METHODS: [u32; 3] = [1, 2, 4];
const ACQUIRE_AUTOMATIC_SKILL_RANK: u32 = 1;

/// Spells the player knows, in learn order. Cast validation reads this.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownSpells(Vec<u32>, Vec<(u32, u32)>, Vec<u32>);

impl KnownSpells {
    /// Membership-derived grants are separate from baseline/profession knowledge.
    pub(crate) fn guild_grants(&self) -> &[u32] {
        &self.2
    }
    pub(crate) fn set_guild_grants(&mut self, spells: &[u32]) {
        self.2 = spells.to_vec();
    }
    pub(crate) fn pvp_grants(&self) -> &[(u32, u32)] {
        &self.1
    }
    pub(crate) fn set_pvp_grants(&mut self, mut grants: Vec<(u32, u32)>) {
        grants.sort_unstable();
        grants.dedup();
        self.1 = grants;
    }

    #[cfg(test)]
    pub(crate) fn fixture(spells: Vec<u32>) -> Self {
        Self(spells, Vec::new(), Vec::new())
    }

    pub fn contains(&self, spell_id: u32) -> bool {
        self.iter().any(|id| id == spell_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        let baseline = self
            .0
            .iter()
            .copied()
            .filter(|id| !self.1.iter().any(|&(_, base)| base == *id));
        let grants = self
            .1
            .iter()
            .map(|&(spell, _)| spell)
            .filter(|id| !self.0.contains(id) || self.1.iter().any(|&(_, base)| base == *id));
        let guild =
            self.2.iter().copied().filter(|id| {
                !self.0.contains(id) && !self.1.iter().any(|&(spell, _)| spell == *id)
            });
        baseline.chain(grants).chain(guild)
    }
}

/// Active `ChrSpecialization` ID, replicated to every client.
pub use shared::components::ActiveSpec;

/// Occupied action bar slots.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionBar(BTreeMap<u8, ActionRef>);

impl ActionBar {
    pub(crate) fn update_pvp_overrides(
        &mut self,
        previous: &[(u32, u32)],
        current: &[(u32, u32)],
        known: &KnownSpells,
    ) -> Option<ActionBarSnapshot> {
        let map = |grants: &[(u32, u32)]| {
            grants
                .iter()
                .filter(|&&(_, base)| base != 0)
                .map(|&(spell, base)| (base, spell))
                .collect::<Vec<_>>()
        };
        let changed = apply_spell_overrides(&mut self.0, &map(previous), &map(current), known);
        changed.then(|| bar_snapshot(self))
    }
}

/// Validated trait config of the active spec (granted entries included),
/// its unspent points and the spells it applies.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitLoadout {
    tree_id: u32,
    entries: Vec<ConfigEntry>,
    unspent: Vec<(u32, i32)>,
    spells: Vec<KnownSpell>,
    /// `(TraitDefinition.OverridesSpellID, SpellID)` of the applied talents, then
    /// `(SpecializationSpells.OverridesSpellID, SpellID)` of the spec's learned spells:
    /// the baseline spell is not known and its bar buttons show the replacement.
    overrides: Vec<(u32, u32)>,
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
                init_player_progression.in_set(crate::achievement::InitializeSpellKnowledge),
                process_set_specialization,
                process_commit_trait_config,
                process_set_action_button,
                sync_level_changes,
            )
                .chain(),
        );
        // Learning can happen in Update (admin/trainer) or FixedUpdate (spell effects).
        app.add_systems(PostUpdate, publish_learned_spells);
    }
}

/// Merge explicitly learned spells without applying class/spec auto-learning filters.
/// Return only additions, preserving their persisted learn order.
fn merge_learned_spells(
    known: &mut KnownSpells,
    professions: Option<&PlayerProfessions>,
) -> Vec<u32> {
    let mut seen: BTreeSet<u32> = known.iter().collect();
    let learned: Vec<u32> = professions
        .into_iter()
        .flat_map(|profs| profs.spells.iter().copied())
        .filter(|spell| seen.insert(*spell))
        .collect();
    known.0.extend(&learned);
    learned
}

/// Publish admin, trainer and spell-effect learning to the owner's spellbook.
fn publish_learned_spells(
    connections: ConnectionLinks,
    mut players: Query<(&PlayerProfessions, &mut KnownSpells), Changed<PlayerProfessions>>,
    mut senders: Query<&mut MessageSender<SpellsLearned>>,
) {
    for (connection, link) in &connections {
        let Ok((professions, mut known)) = players.get_mut(link.0) else {
            continue;
        };
        let learned = merge_learned_spells(known.bypass_change_detection(), Some(professions));
        if !learned.is_empty() {
            known.set_changed();
            send(&mut senders, connection, SpellsLearned { spells: learned });
        }
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

/// Auto-learned class spells up to `level` for the race, the default skill line
/// spells (`default_skill_spells`), then the spec's spells
/// up to `level` (TrinityCore `Player::LearnSpecializationSpells` skips spells whose
/// `SpellLevel` is above the player level) and mastery, then talent spells, without
/// duplicates and without the spells the talents override.
fn compute_known_spells(
    class: &ClassInfo,
    race: u8,
    level: u8,
    spec: Option<&SpecInfo>,
    traits: &TraitLoadout,
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
    let defaults = default_skill_spells(class, race, level);
    let learnable = learnable.chain(defaults);
    let spec_spells = spec.into_iter().flat_map(|spec| {
        let spells = spec
            .spells
            .iter()
            .filter(move |spell| spell.required_level <= u32::from(level))
            .map(|spell| KnownSpell {
                spell_id: spell.spell_id,
                passive: spell.passive,
            });
        let masteries = spec.mastery_spells.iter().map(|&spell_id| KnownSpell {
            spell_id,
            passive: true,
        });
        spells.chain(masteries)
    });
    let overridden = |spell_id| traits.overrides.iter().any(|&(base, _)| base == spell_id);
    let mut known: Vec<KnownSpell> = Vec::new();
    for spell in learnable
        .chain(spec_spells)
        .chain(traits.active_talent_spells())
    {
        if !overridden(spell.spell_id)
            && !known.iter().any(|entry| entry.spell_id == spell.spell_id)
        {
            known.push(spell);
        }
    }
    known
}

/// Abilities of the default skill lines (Player::LearnDefaultSkills, Player.cpp:25275-25291):
/// per skill line the first row by ID for the race whose `MinLevel` is reached, and its
/// auto-learned abilities for the race up to `level` (LearnSkillRewardedSpells,
/// Player.cpp:25407-25457). An AutomaticSkillRank ability needs the line's starting
/// value to reach its `MinSkillLineRank`.
pub(crate) fn default_skill_spells(class: &ClassInfo, race: u8, level: u8) -> Vec<KnownSpell> {
    let mut learned_lines = Vec::new();
    let mut spells = Vec::new();
    for line in &class.default_skills {
        if learned_lines.contains(&line.skill_line)
            || !race_mask_allows(line.race_masks, race)
            || line.min_level > u32::from(level)
        {
            continue;
        }
        learned_lines.push(line.skill_line);
        let value = line.range.value(class.class_id, level);
        spells.extend(
            line.abilities
                .iter()
                .filter(|spell| {
                    AUTO_LEARN_METHODS.contains(&spell.acquire_method)
                        && (spell.acquire_method != ACQUIRE_AUTOMATIC_SKILL_RANK
                            || value >= spell.min_skill_rank)
                        && spell.required_level <= u32::from(level)
                        && race_mask_allows(spell.race_masks, race)
                })
                .map(|spell| KnownSpell {
                    spell_id: spell.spell_id,
                    passive: spell.passive,
                }),
        );
    }
    spells
}

/// Known non-passive, non-mount spells in learn order.
fn pushable_spells<'a>(
    class: Option<&'a ClassInfo>,
    known: &'a [KnownSpell],
) -> impl Iterator<Item = u32> + 'a {
    let is_mount = move |spell_id| {
        class.is_some_and(|class| {
            class
                .learnable
                .iter()
                .any(|spell| spell.spell_id == spell_id && spell.mount)
        })
    };
    known
        .iter()
        .filter(move |spell| !spell.passive && !is_mount(spell.spell_id))
        .map(|spell| spell.spell_id)
}

/// Retail pushes each newly learned active spell to the first empty main action bar
/// slot (`SPELL_PUSHED_TO_ACTIONBAR`, `IconIntroTracker`), in learn order; spells
/// already on the bar, passives and class mounts are not pushed. Returns true when
/// any slot was filled.
fn push_learned_spells(
    bar: &mut BTreeMap<u8, ActionRef>,
    class: Option<&ClassInfo>,
    known: &[KnownSpell],
    learned: &[u32],
) -> bool {
    let mut pushed = false;
    let pushable = pushable_spells(class, known)
        .filter(|spell_id| learned.contains(spell_id))
        .collect::<Vec<_>>();
    for spell_id in pushable {
        let action = ActionRef::Spell(spell_id);
        if bar.values().any(|existing| *existing == action) {
            continue;
        }
        let Some(slot) = (0..MAIN_BAR_SLOTS).find(|slot| !bar.contains_key(slot)) else {
            break;
        };
        bar.insert(slot, action);
        pushed = true;
    }
    pushed
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

/// Point buttons of overridden spells at their talent replacement, and buttons
/// of a replacement that is no longer known back at its baseline spell.
/// `previous` are the overrides the bar was last rewritten with. Returns true
/// when any button changed.
fn apply_spell_overrides(
    bar: &mut BTreeMap<u8, ActionRef>,
    previous: &[(u32, u32)],
    overrides: &[(u32, u32)],
    known: &KnownSpells,
) -> bool {
    let mut changed = false;
    for action in bar.values_mut() {
        let ActionRef::Spell(spell_id) = action else {
            continue;
        };
        let replacement = overrides.iter().find(|&&(base, _)| base == *spell_id);
        let restored = previous
            .iter()
            .find(|&&(_, talent)| talent == *spell_id && !known.contains(talent));
        if let Some(&(_, talent)) = replacement {
            *spell_id = talent;
            changed = true;
        } else if let Some(&(base, _)) = restored {
            *spell_id = base;
            changed = true;
        }
    }
    changed
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
    let talents: Vec<_> = talent_spell_entries(&class.traits, &entries).collect();
    let traits = TraitLoadout {
        tree_id: class.traits.tree_id,
        unspent: unspent(&class.traits, ctx, &entries),
        spells: talents
            .iter()
            .map(|entry| KnownSpell {
                spell_id: entry.spell_id,
                passive: entry.passive,
            })
            .collect(),
        overrides: talents
            .iter()
            .filter(|entry| entry.overrides_spell_id != 0)
            .map(|entry| (entry.overrides_spell_id, entry.spell_id))
            .chain(spec_overrides(spec, level))
            .collect(),
        entries,
    };
    let known = compute_known_spells(class, player.race, level, spec, &traits);
    Evaluated {
        spec: ActiveSpec(ctx.spec_id),
        known,
        traits,
        traits_reset,
    }
}

/// `(SpecializationSpells.OverridesSpellID, SpellID)` of the spec spells learned at
/// `level` (TrinityCore `Player::LearnSpecializationSpells` → `AddOverrideSpell`):
/// Havoc's Demon's Bite 162243 replaces the baseline 344859 like a talent override.
fn spec_overrides(spec: Option<&SpecInfo>, level: u8) -> impl Iterator<Item = (u32, u32)> + '_ {
    spec.into_iter()
        .flat_map(|spec| &spec.spells)
        .filter(move |spell| {
            spell.overrides_spell_id != 0 && spell.required_level <= u32::from(level)
        })
        .map(|spell| (spell.overrides_spell_id, spell.spell_id))
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
    loadouts: Query<'w, 's, &'static mut MessageSender<TraitLoadoutsSnapshot>>,
    /// The replicated `ActiveSpec` reaches clients only once its entity is marked.
    replication: Option<Res<'w, ReplicationDirtyQueue>>,
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
    players: Query<(&Player, &Level, &ClientId, Option<&PlayerProfessions>), Without<KnownSpells>>,
    mut senders: ProgressionSenders,
) {
    let (Some(db), Some(classes)) = (db, classes) else {
        return;
    };
    for (connection, link) in &connections {
        let Ok((player, level, client_id, professions)) = players.get(link.0) else {
            continue;
        };
        let stored = load_progression(&db.db, client_id.0);
        let requested = stored.as_ref().map_or(0, |data| data.spec_id);
        let configs = match refresh_starter_trait_configs(
            &classes,
            player,
            level.0,
            requested,
            &db,
            client_id.0,
        ) {
            Ok(configs) => configs,
            Err(reason) => {
                error!("Cannot restore Starter Build: {reason}");
                continue;
            }
        };
        let evaluated = evaluate(&classes, player, level.0, requested, |spec| {
            configs.spec(spec).to_vec()
        });
        let spec = evaluated.spec;
        if evaluated.traits_reset {
            save_trait_config(&db.db, client_id.0, spec.0, Vec::new());
        }
        let mut known = KnownSpells(
            evaluated.known.iter().map(|spell| spell.spell_id).collect(),
            Vec::new(),
            Vec::new(),
        );
        merge_learned_spells(&mut known, professions);
        let mut bar = ActionBar(match &stored {
            Some(data) => data
                .action_bar
                .iter()
                .map(|&(slot, action)| (slot, action_from_data(action)))
                .collect(),
            None => BTreeMap::new(),
        });
        let overrides = &evaluated.traits.overrides;
        let rewritten = apply_spell_overrides(&mut bar.0, &[], overrides, &known);
        let pruned = prune_unknown_spells(&mut bar.0, &known);
        if stored.is_none() || rewritten || pruned || requested != spec.0 {
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
            send(
                &mut senders.loadouts,
                connection,
                loadout_snapshot(
                    &load_trait_configs(&db.db, client_id.0),
                    spec.0,
                    &classes,
                    player,
                ),
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
    Entity,
    Option<&'static PlayerProfessions>,
);

/// Apply a re-evaluated spec, trait config and known spells after a level, spec
/// or trait change; send deltas, rewrite bar buttons of overridden spells, drop
/// bar buttons for unlearned spells and persist. `TraitConfigSnapshot` is sent when the spec or config changed, or
/// always with `always_send_traits`.
fn refresh_player(
    (player, _, client_id, mut spec, mut known, mut bar, mut traits, entity, professions): (
        &Player,
        &Level,
        &ClientId,
        Mut<ActiveSpec>,
        Mut<KnownSpells>,
        Mut<ActionBar>,
        Mut<TraitLoadout>,
        Entity,
        Option<&PlayerProfessions>,
    ),
    evaluated: Evaluated,
    always_send_traits: bool,
    classes: &ClassSpells,
    db: &GameDatabase,
    connection: Entity,
    senders: &mut ProgressionSenders,
) {
    let new_spec = evaluated.spec;
    let mut new_known = KnownSpells(
        evaluated.known.iter().map(|spell| spell.spell_id).collect(),
        Vec::new(),
        known.2.clone(),
    );
    merge_learned_spells(&mut new_known, professions);
    if *spec == new_spec {
        new_known.1 = known.1.clone();
    }
    let spec_changed = *spec != new_spec;
    if spec_changed {
        *spec = new_spec;
        if let Some(replication) = &senders.replication {
            replication.mark(entity);
        }
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
    let old_ids: Vec<_> = known.iter().collect();
    let new_ids: Vec<_> = new_known.iter().collect();
    let unlearned = spells_missing_from(&old_ids, &new_ids);
    let learned = spells_missing_from(&new_ids, &old_ids);
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
            SpellsLearned {
                spells: learned.clone(),
            },
        );
    }
    let rewritten = apply_spell_overrides(
        &mut bar.0,
        &traits.overrides,
        &evaluated.traits.overrides,
        &new_known,
    );
    let pruned = prune_unknown_spells(&mut bar.0, &new_known);
    let pushed = push_learned_spells(
        &mut bar.0,
        class_info(classes, player.class),
        &evaluated.known,
        &learned,
    );
    let bar_changed = rewritten || pruned || pushed;
    if bar_changed {
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
        send(
            &mut senders.loadouts,
            connection,
            loadout_snapshot(
                &load_trait_configs(&db.db, client_id.0),
                new_spec.0,
                classes,
                player,
            ),
        );
    }
    if spec_changed || bar_changed {
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
    pvp: Query<&crate::pvp::PlayerPvpState>,
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
            if pvp.get(link.0).is_ok_and(shuffle_spec_locked) {
                send(
                    &mut senders.spec,
                    connection,
                    SpecializationChanged { spec_id: entry.3.0 },
                );
                continue;
            }
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
            let configs = match refresh_starter_trait_configs(
                &classes,
                entry.0,
                entry.1.0,
                request.spec_id,
                &db,
                entry.2.0,
            ) {
                Ok(configs) => configs,
                Err(reason) => {
                    error!("Cannot activate Starter Build specialization: {reason}");
                    continue;
                }
            };
            let evaluated = evaluate(&classes, entry.0, entry.1.0, request.spec_id, |spec| {
                configs.spec(spec).to_vec()
            });
            refresh_player(
                entry,
                evaluated,
                false,
                &classes,
                &db,
                connection,
                &mut senders,
            );
        }
    }
}

/// Realm Shuffle admission fixes two healers/four damage until the entrant leaves.
fn shuffle_spec_locked(state: &crate::pvp::PlayerPvpState) -> bool {
    matches!(
        state.queue,
        Some(crate::pvp::PlayerPvpQueue::Arena {
            mode: shared::protocol::ArenaQueueMode::SoloShuffle
        })
    ) || state
        .battleground
        .as_ref()
        .and_then(|bg| bg.arena.as_ref())
        .is_some_and(|arena| arena.mode == shared::protocol::ArenaQueueMode::SoloShuffle)
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
    mut commits: Query<(Entity, &PlayerLink, &mut MessageReceiver<CommitTraitConfig>)>,
    mut loadouts: Query<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<TraitLoadoutRequest>,
    )>,
    mut players: Query<ProgressionPlayer>,
    mut senders: ProgressionSenders,
) {
    let (Some(db), Some(classes)) = (db, classes) else {
        return;
    };
    // Typed buffers must not reorder an Apply followed by a loadout switch.
    let mut requests = Vec::new();
    for (connection, link, mut receiver) in &mut commits {
        requests.extend(receiver.receive_with_tick().map(|message| {
            (
                connection,
                link.0,
                message.message_id,
                TraitMutation::Commit(message.data),
            )
        }));
    }
    for (connection, link, mut receiver) in &mut loadouts {
        requests.extend(receiver.receive_with_tick().map(|message| {
            (
                connection,
                link.0,
                message.message_id,
                TraitMutation::Loadout(message.data),
            )
        }));
    }
    requests.sort_by_key(|(connection, _, id, _)| (*connection, *id));
    for (connection, player, _, request) in requests {
        let Ok(entry) = players.get_mut(player) else {
            continue;
        };
        let spec_id = entry.3.0;
        let mut configs = load_trait_configs(&db.db, entry.2.0);
        let refresh_allocation = match update_trait_configs(
            &classes,
            entry.0,
            entry.1.0,
            spec_id,
            &mut configs,
            request,
        ) {
            Ok(refresh_allocation) => refresh_allocation,
            Err(reason) => {
                warn!("Rejected trait loadout mutation: {reason}");
                send(
                    &mut senders.commit,
                    connection,
                    TraitCommitResult {
                        ok: false,
                        reason: Some(reason),
                    },
                );
                continue;
            }
        };
        save_trait_configs(&db.db, entry.2.0, &configs);
        send(
            &mut senders.commit,
            connection,
            TraitCommitResult {
                ok: true,
                reason: None,
            },
        );
        if refresh_allocation {
            let evaluated = evaluate(&classes, entry.0, entry.1.0, spec_id, |spec| {
                configs.spec(spec).to_vec()
            });
            refresh_player(
                entry,
                evaluated,
                true,
                &classes,
                &db,
                connection,
                &mut senders,
            );
        } else {
            // Rename/delete are metadata, not a rollback of the client's staged ranks.
            send(
                &mut senders.loadouts,
                connection,
                loadout_snapshot(&configs, spec_id, &classes, entry.0),
            );
        }
    }
}

enum TraitMutation {
    Commit(CommitTraitConfig),
    Loadout(TraitLoadoutRequest),
}
fn update_trait_configs(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    active_spec: u32,
    configs: &mut TraitConfigsData,
    request: TraitMutation,
) -> Result<bool, String> {
    let request = match request {
        TraitMutation::Commit(request) => {
            let purchases = commit_purchases(classes, player, level, active_spec, &request)?;
            let spec = configs.spec_mut(active_spec);
            if spec.selected_id == STARTER_BUILD_LOADOUT_ID {
                spec.selected_id = 0;
            }
            spec.replace_active(purchases);
            return Ok(true);
        }
        TraitMutation::Loadout(request) => request,
    };
    if request.spec_id != active_spec {
        return Err(format!(
            "spec {} is not the active spec {active_spec}",
            request.spec_id
        ));
    }
    let refresh_allocation = match request.operation {
        TraitLoadoutOperation::Create { name, entries } => {
            let purchases = commit_purchases(
                classes,
                player,
                level,
                active_spec,
                &CommitTraitConfig {
                    spec_id: active_spec,
                    entries,
                },
            )?;
            configs.create(active_spec, name, purchases)?;
            true
        }
        TraitLoadoutOperation::Rename { id, name } => {
            configs.rename(active_spec, id, name)?;
            false
        }
        TraitLoadoutOperation::Delete { id } => {
            configs.delete(active_spec, id)?;
            false
        }
        TraitLoadoutOperation::Activate { id } => {
            let class = class_info(classes, player.class).ok_or("Class has no trait tree")?;
            if id == STARTER_BUILD_LOADOUT_ID {
                let purchases = starter_purchases(
                    &class.traits,
                    TraitContext {
                        spec_id: active_spec,
                        level,
                    },
                )?;
                let spec = configs.spec_mut(active_spec);
                spec.default = purchases;
                spec.selected_id = STARTER_BUILD_LOADOUT_ID;
                return Ok(true);
            }
            configs.activate(active_spec, id)?;
            build_config(
                &class.traits,
                TraitContext {
                    spec_id: active_spec,
                    level,
                },
                configs.spec(active_spec),
            )?;
            true
        }
    };
    Ok(refresh_allocation)
}

fn loadout_snapshot(
    configs: &TraitConfigsData,
    spec_id: u32,
    classes: &ClassSpells,
    player: &Player,
) -> TraitLoadoutsSnapshot {
    let spec = configs.specs.iter().find(|spec| spec.spec_id == spec_id);
    TraitLoadoutsSnapshot {
        spec_id,
        selected_id: spec.map_or(0, |spec| spec.selected_id),
        starter_available: class_info(classes, player.class).is_some_and(|class| {
            class
                .traits
                .starter_builds
                .iter()
                .any(|build| build.spec_id == spec_id)
        }),
        configs: spec.map_or_else(Vec::new, |spec| {
            spec.configs
                .iter()
                .map(|config| TraitLoadoutInfo {
                    id: config.id,
                    name: config.name.clone(),
                })
                .collect()
        }),
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

/// Rebuild the active built-in allocation from authored rows at the current level.
/// Named allocations and their persistent wire/storage format remain untouched.
fn refresh_starter_trait_configs(
    classes: &ClassSpells,
    player: &Player,
    level: u8,
    spec_id: u32,
    db: &GameDatabase,
    character_id: u64,
) -> Result<TraitConfigsData, String> {
    let mut configs = load_trait_configs(&db.db, character_id);
    if configs
        .specs
        .iter()
        .any(|spec| spec.spec_id == spec_id && spec.selected_id == STARTER_BUILD_LOADOUT_ID)
    {
        let class = class_info(classes, player.class).ok_or("Class has no trait tree")?;
        let purchases = starter_purchases(&class.traits, TraitContext { spec_id, level })?;
        if configs.spec(spec_id) != purchases {
            configs.spec_mut(spec_id).default = purchases;
            save_trait_configs(&db.db, character_id, &configs);
        }
    }
    Ok(configs)
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
        let configs = match refresh_starter_trait_configs(
            &classes,
            entry.0,
            entry.1.0,
            current_spec,
            &db,
            entry.2.0,
        ) {
            Ok(configs) => configs,
            Err(reason) => {
                error!("Cannot update Starter Build: {reason}");
                continue;
            }
        };
        let starter_active = configs.specs.iter().any(|spec| {
            spec.spec_id == current_spec && spec.selected_id == STARTER_BUILD_LOADOUT_ID
        });
        let evaluated = evaluate(&classes, entry.0, entry.1.0, current_spec, |spec| {
            if spec == current_spec && !starter_active {
                entry.6.purchases()
            } else {
                configs.spec(spec).to_vec()
            }
        });
        refresh_player(
            entry,
            evaluated,
            false,
            &classes,
            &db,
            connection,
            &mut senders,
        );
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

/// Put each action on its slot when the slot is empty and the action is not on the bar
/// yet, and send the owner the bar when any was placed. Not persisted: the login prune
/// keeps only class progression spells (`prune_unknown_spells`).
pub(crate) fn place_actions(world: &mut World, unit: Entity, actions: &[(u8, ActionRef)]) {
    let Some(mut bar) = world.get_mut::<ActionBar>(unit) else {
        return;
    };
    let mut placed = false;
    for &(slot, action) in actions {
        if bar.0.contains_key(&slot) || bar.0.values().any(|existing| *existing == action) {
            continue;
        }
        bar.0.insert(slot, action);
        placed = true;
    }
    if !placed {
        return;
    }
    let snapshot = bar_snapshot(&bar);
    let Some(connection) = crate::spell_cast::owner_connection(world, unit) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<ActionBarSnapshot>>(connection) {
        sender.send::<TalentChannel>(snapshot);
    }
}

/// Apply an admin level change to the online player, if any. `Level` changes
/// drive `sync_level_changes`; `UnitLevel` follows when present. A new level
/// refills health and mana like a level-up (Player::GiveLevel, Player.cpp:2202-2271,
/// which the online `.character level` calls: cs_character.cpp:770).
pub(crate) fn set_live_level(world: &mut World, character_id: u64, level: u8) {
    let mut players = world.query::<(Entity, &ClientId, &mut Level, Option<&mut UnitLevel>)>();
    let mut changed = Vec::new();
    for (entity, client_id, mut player_level, unit_level) in players.iter_mut(world) {
        if client_id.0 != character_id {
            continue;
        }
        if player_level.0 != level {
            changed.push(entity);
        }
        player_level.0 = level;
        if let Some(mut unit_level) = unit_level {
            unit_level.0 = level;
        }
    }
    for entity in changed {
        world
            .entity_mut(entity)
            .insert(crate::experience::RefillAfterLevelUp);
    }
}

#[cfg(test)]
#[path = "class_progression_tests.rs"]
mod tests;
