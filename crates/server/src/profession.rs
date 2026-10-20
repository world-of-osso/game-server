//! Professions (docs/wiki/systems/professions.md): Retail skill lines learned from
//! profession trainers, and crafting through the spell pipeline.
//!
//! A profession is a parent `SkillLine` (197 Tailoring) plus per-expansion child
//! lines (2540 Classic Tailoring). Rules follow TrinityCore master
//! (a352b1fa): trainer spells are cast (`Trainer::TeachSpell`), `LEARN_SPELL`
//! (36) learns its trigger spell and the skills its `SKILL` (118) effects name
//! (`Player::AddSpell`), `SKILL_STEP` (44) raises a line to a tier
//! (`Spell::EffectSkill`), a line activates its parent at `ParentTierIndex`
//! (`Player::SetSkill`) and teaches its automatic abilities
//! (`Player::LearnSkillRewardedSpells`). A recipe cast checks and takes its
//! `SpellReagents` and `SpellTotems` (`Spell::CheckItems`/`TakeReagents`),
//! `CREATE_ITEM` (24) stores the item and rolls `Player::UpdateCraftSkill`.
//! `PlayerProfessions` changes are saved and sent as `ProfessionSnapshot`.

mod concentration;
#[cfg(test)]
mod crafting_provenance_fixture;
mod data;
pub(crate) mod favorites;
mod gathering;
#[cfg(test)]
mod modern_crafting_fixture;
mod quality_reagents;
pub(crate) mod specializations;
mod trainer;
pub(crate) use gathering::GatherProgress;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Player;
use shared::item_data::ItemDefinitionSource::Retail;
use shared::profession::{MAX_PRIMARY_PROFESSIONS, ProfessionSkillLine, recipe_difficulty};
use shared::protocol::{CraftRecipe, ProfessionChannel, ProfessionSnapshot};

pub use data::{ProfessionData, load_profession_data};
pub(crate) use trainer::TrainerPlugin;

use crate::auth::{Authenticated, InWorld};
use crate::gold::GoldUpdateQueue;
use crate::inventory::inventory_model::InventoryModel;
use crate::inventory::load_model;
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use crate::networking::PlayerLink;
use crate::persistence::inventory_store::save_inventory_record;
use crate::persistence::{self, ClientId, GameDatabase, InventoryItemData};
use crate::spell_cast::CastFailure;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use data::ACQUIRE_AUTOMATIC_SKILL_RANK;

pub const EFFECT_CREATE_ITEM: u32 = 24;
const EFFECT_QUALITY_CRAFT: u32 = 288;
const EFFECT_RECRAFT: u32 = 297;
pub const EFFECT_LEARN_SPELL: u32 = 36;
pub const EFFECT_SKILL_STEP: u32 = 44;
pub const EFFECT_SKILL: u32 = 118;

/// A character's learned skill lines and the spells learned through them.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct PlayerProfessions {
    pub lines: Vec<ProfessionSkillLine>,
    pub spells: Vec<u32>,
}

impl PlayerProfessions {
    pub fn knows(&self, spell: u32) -> bool {
        self.spells.contains(&spell)
    }

    pub fn line(&self, skill: u32) -> Option<&ProfessionSkillLine> {
        self.lines.iter().find(|line| line.skill_line == skill)
    }

    pub fn rank(&self, skill: u32) -> u16 {
        self.line(skill).map_or(0, |line| line.rank)
    }

    fn step(&self, skill: u32) -> u16 {
        self.line(skill).map_or(0, |line| line.step)
    }

    fn set_line(&mut self, line: ProfessionSkillLine) {
        match self
            .lines
            .iter_mut()
            .find(|known| known.skill_line == line.skill_line)
        {
            Some(known) => *known = line,
            None => self.lines.push(line),
        }
    }

    pub fn snapshot(&self, data: &ProfessionData) -> ProfessionSnapshot {
        ProfessionSnapshot {
            lines: self.lines.clone(),
            spells: self.spells.clone(),
            recipe_sources: data.trainer_sources(&self.lines),
            favorites: Vec::new(),
        }
    }
}

/// Learning rules for one character (race and class select tiers and abilities).
pub(crate) struct Learner<'a> {
    pub(crate) data: &'a ProfessionData,
    pub(crate) spells: &'a SpellStore,
    pub(crate) race: u8,
    pub(crate) class: u8,
}

impl Learner<'_> {
    /// `Player::LearnSpell` / `AddSpell`: the spell, then the skills its `SKILL`
    /// effects name. `from_skill` is the line teaching it, whose skill is not re-set.
    pub(crate) fn learn_spell(&self, profs: &mut PlayerProfessions, spell: u32, from_skill: u32) {
        if profs.knows(spell) {
            return;
        }
        profs.spells.push(spell);
        let Some(info) = self.spells.get(spell) else {
            warn!("Learned profession spell {spell} has no spell data");
            return;
        };
        for effect in info.effects.iter().filter(|e| e.effect == EFFECT_SKILL) {
            let skill = effect.misc_values[0] as u32;
            let step = effect.base_points as u16;
            if skill == from_skill || step == 0 {
                continue;
            }
            let Some(cap) = self.data.tier_cap(skill, step, self.race, self.class) else {
                warn!("Spell {spell}: skill {skill} has no tier {step} cap");
                continue;
            };
            let max = cap.max(profs.line(skill).map_or(0, |line| line.max_rank));
            // Player.cpp:3004-3005: an always-max skill (Riding) takes its new maximum.
            let rank = if self.data.always_max(skill, self.race, self.class) {
                max
            } else {
                profs.rank(skill).max(1)
            };
            self.set_skill(profs, skill, step, rank, max);
        }
    }

    /// `Spell::EffectSkill` (`SKILL_STEP`): raise `skill` to tier `step`.
    pub(crate) fn skill_step(&self, profs: &mut PlayerProfessions, skill: u32, step: u16) {
        if step == 0 || profs.step(skill) >= step {
            return;
        }
        let Some(max) = self.data.tier_cap(skill, step, self.race, self.class) else {
            warn!("Skill {skill} has no tier {step} cap");
            return;
        };
        self.set_skill(profs, skill, step, profs.rank(skill).max(1), max);
    }

    /// `Player::SetSkill` for an active line: enable the parent line at
    /// `ParentTierIndex`, store the line, teach its automatic abilities.
    fn set_skill(&self, profs: &mut PlayerProfessions, skill: u32, step: u16, rank: u16, max: u16) {
        if let Some(line) = self.data.skill_line(skill)
            && line.parent != 0
            && line.parent_tier_index > 0
            && profs.step(line.parent) < line.parent_tier_index
        {
            let parent_step = line.parent_tier_index;
            if let Some(parent_max) =
                self.data
                    .tier_cap(line.parent, parent_step, self.race, self.class)
            {
                let parent_rank = profs.rank(line.parent).max(1);
                self.set_skill(profs, line.parent, parent_step, parent_rank, parent_max);
            }
        }
        profs.set_line(ProfessionSkillLine {
            skill_line: skill,
            step,
            rank,
            max_rank: max,
        });
        self.learn_rewarded_spells(profs, skill, rank);
    }

    /// `Player::LearnSkillRewardedSpells`: automatic abilities of `skill`;
    /// `AutomaticSkillRank` ones from their `MinSkillLineRank`.
    fn learn_rewarded_spells(&self, profs: &mut PlayerProfessions, skill: u32, rank: u16) {
        let spells: Vec<(u32, u32)> = self
            .data
            .abilities_for_skill(skill, self.race, self.class)
            .filter(|ability| ability.auto_learned())
            .filter(|ability| {
                ability.acquire_method != ACQUIRE_AUTOMATIC_SKILL_RANK || rank >= ability.min_rank
            })
            .map(|ability| (ability.spell, ability.skill_line))
            .collect();
        for (spell, line) in spells {
            self.learn_spell(profs, spell, line);
        }
    }

    /// `Player::UpdateCraftSkill` + `UpdateSkillPro`: `roll` in 1..=1000 raises the
    /// recipe's skill-up line by `NumSkillUps` when it is within the difficulty's
    /// chance. Returns the new rank when the skill rose.
    pub(crate) fn craft_skill_up(
        &self,
        profs: &mut PlayerProfessions,
        spell: u32,
        roll: u32,
    ) -> Option<u16> {
        let ability = self
            .data
            .abilities_for_spell(spell)
            .iter()
            .find(|ability| ability.skillup_line != 0)?;
        let line = *profs.line(ability.skillup_line)?;
        if line.rank == 0 || line.rank >= line.max_rank {
            return None;
        }
        let chance = recipe_difficulty(line.rank, ability.trivial_low, ability.trivial_high)
            .skill_up_chance_permille();
        if roll > chance {
            return None;
        }
        let rank = (line.rank + ability.num_skill_ups.max(1)).min(line.max_rank);
        profs.set_line(ProfessionSkillLine { rank, ..line });
        self.learn_rewarded_spells(profs, line.skill_line, rank);
        Some(rank)
    }

    /// Gathering raises the authored expansion line, never a fabricated recipe rank.
    pub(crate) fn gather_skill_up(
        &self,
        profs: &mut PlayerProfessions,
        progress: GatherProgress,
        roll: u32,
    ) -> Option<u16> {
        let line = *profs.line(progress.skill)?;
        if line.rank == 0
            || line.rank >= line.max_rank
            || roll > progress.chance_permille(line.rank)
        {
            return None;
        }
        let parent = self.data.skill_line(progress.skill)?.parent;
        if !matches!(parent, 182 | 186 | 393) {
            return None;
        }
        let rank = (line.rank + 1).min(line.max_rank);
        profs.set_line(ProfessionSkillLine { rank, ..line });
        self.learn_rewarded_spells(profs, progress.skill, rank);
        Some(rank)
    }

    /// Primary profession lines known (`GetFreePrimaryProfessionPoints` counts
    /// against `MAX_PRIMARY_PROFESSIONS`).
    fn primary_professions(&self, profs: &PlayerProfessions) -> usize {
        profs
            .lines
            .iter()
            .filter(|line| self.data.is_primary_profession(line.skill_line))
            .count()
    }

    /// Whether casting or learning `spell` would add a primary profession the
    /// character lacks (`SpellInfo::IsPrimaryProfessionFirstRank` on the spell or
    /// its `LEARN_SPELL` triggers).
    pub(crate) fn adds_primary_profession(&self, profs: &PlayerProfessions, spell: u32) -> bool {
        let Some(info) = self.spells.get(spell) else {
            return false;
        };
        info.effects.iter().any(|effect| match effect.effect {
            EFFECT_SKILL => {
                let skill = effect.misc_values[0] as u32;
                self.data.is_primary_profession(skill) && profs.line(skill).is_none()
            }
            EFFECT_LEARN_SPELL => self.adds_primary_profession(profs, effect.trigger_spell),
            _ => false,
        })
    }

    /// No primary profession slot is left for `spell`.
    pub(crate) fn blocked_by_primary_limit(&self, profs: &PlayerProfessions, spell: u32) -> bool {
        self.adds_primary_profession(profs, spell)
            && self.primary_professions(profs) >= MAX_PRIMARY_PROFESSIONS
    }
}

/// Remaining casts of a `CraftRecipe` request; removed when a cast is cancelled
/// or the next one cannot start.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CraftQueue {
    pub(crate) spell_id: u32,
    pub(crate) remaining: u16,
}

pub struct ProfessionPlugin;

impl Plugin for ProfessionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((TrainerPlugin, specializations::SpecializationsPlugin))
            .add_systems(
                Update,
                (
                    receive_craft_requests,
                    quality_reagents::receive_allocations,
                ),
            )
            .add_systems(
                FixedUpdate,
                continue_craft_queues.after(crate::spell_cast::advance_casts),
            )
            .add_systems(Update, concentration::receive_queries)
            .add_systems(Update, favorites::receive_requests)
            .add_systems(PostUpdate, save_and_send_professions);
    }
}

/// Run `apply` on `player`'s professions with its race/class learning rules.
pub(crate) fn with_learner<R>(
    world: &mut World,
    player: Entity,
    apply: impl FnOnce(&Learner, &mut PlayerProfessions) -> R,
) -> Option<R> {
    let (race, class) = world
        .get::<Player>(player)
        .map(|player| (player.race, player.class))?;
    let mut profs = world.get::<PlayerProfessions>(player)?.clone();
    let result = {
        let data = world.get_resource::<ProfessionData>()?;
        let spells = world.get_resource::<SpellStore>()?;
        let learner = Learner {
            data,
            spells,
            race,
            class,
        };
        apply(&learner, &mut profs)
    };
    let mut current = world.get_mut::<PlayerProfessions>(player)?;
    if *current != profs {
        *current = profs;
    }
    Some(result)
}

/// `LEARN_SPELL` (36).
pub(crate) fn effect_learn_spell(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    // TC SpellEffects.cpp1964-1986: zero-trigger item casts learn ON_LEARN effects.
    let spells = if effect.trigger_spell != 0 {
        vec![effect.trigger_spell]
    } else if let Some(item) = ctx.spell.cast_retail_item_id {
        world
            .resource::<crate::item_templates::ItemTemplates>()
            .retail_learning_spells
            .get(&item)
            .cloned()
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for spell in spells {
        if with_learner(world, target, |learner, profs| {
            learner.learn_spell(profs, spell, 0)
        })
        .is_none()
        {
            warn!("LEARN_SPELL {spell}: target {target} has no professions");
        }
    }
}

/// `SKILL_STEP` (44).
pub(crate) fn effect_skill_step(
    world: &mut World,
    _ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    let skill = effect.misc_values[0] as u32;
    let step = effect.base_points as u16;
    if with_learner(world, target, |learner, profs| {
        learner.skill_step(profs, skill, step)
    })
    .is_none()
    {
        warn!("SKILL_STEP {skill}: target {target} has no professions");
    }
}

/// `CREATE_ITEM` (24): store `EffectBasePoints` of `EffectItemType` in the
/// target's bags, then roll the craft skill-up (`Spell::DoCreateItem`).
pub(crate) fn effect_create_item(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    let count = effect.base_points.round().max(1.0) as u32;
    if let Some(result) = crate::crafting_orders::complete_effect(
        world,
        ctx.caster,
        ctx.spell.id,
        effect.item_type,
        count,
    ) {
        if let Err(error) = result {
            warn!("Crafting order {} failed: {error}", ctx.spell.id);
            return;
        }
        let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=1000);
        with_learner(world, ctx.caster, |learner, profs| {
            learner.craft_skill_up(profs, ctx.spell.id, roll)
        });
        return;
    }
    let origin = world
        .get_resource::<ProfessionData>()
        .and_then(|data| data.crafting_origin(ctx.spell.id));
    if let Err(error) = store_created_item(world, target, effect.item_type, count, origin) {
        warn!("Spell {}: CREATE_ITEM failed: {error}", ctx.spell.id);
        return;
    }
    let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=1000);
    with_learner(world, target, |learner, profs| {
        learner.craft_skill_up(profs, ctx.spell.id, roll)
    });
}

/// The character's inventory, item templates and item names.
struct Bags<'w> {
    db: &'w GameDatabase,
    templates: &'w ItemTemplates,
    catalog: &'w VendorCatalog,
    character_id: u64,
}

impl Bags<'_> {
    fn load(&self) -> Result<InventoryModel, String> {
        load_model(&self.db.db, self.templates, self.character_id)
            .map_err(|error| format!("{error:?}"))
    }

    fn item_name(&self, item_id: u32) -> String {
        self.catalog
            .goods
            .get(&item_id)
            .map_or_else(|| format!("item {item_id}"), |goods| goods.name.clone())
    }

    fn new_item(&self, item_id: u32, count: u32) -> Result<InventoryItemData, String> {
        let goods = self
            .catalog
            .goods
            .get(&item_id)
            .ok_or_else(|| format!("item {item_id} is not in content_item"))?;
        let template = self
            .templates
            .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
            .ok_or_else(|| format!("item {item_id} has no template"))?;
        Ok(InventoryItemData {
            item_guid: persistence::next_id(&self.db.db, "item_instance"),
            item_id,
            name: goods.name.clone(),
            quality: goods.quality,
            required_level: template.required_level,
            stack_count: count,
            vendor_sell_price: goods.sell_price,
            unique_auction: false,
        })
    }

    fn store(
        &self,
        model: &mut InventoryModel,
        item: InventoryItemData,
    ) -> Result<(), shared::protocol::InventoryError> {
        let template = *self
            .templates
            .get(
                shared::item_data::ItemDefinitionSource::Retail,
                item.item_id,
            )
            .ok_or_else(|| {
                crate::inventory::inventory_model::error(
                    shared::protocol::InventoryErrorReason::ItemNotFound,
                    format!("item {} has no template", item.item_id),
                )
            })?;
        let db = &self.db.db;
        let mut mint = || persistence::next_id(db, "item_instance");
        model.store_item(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            &mut mint,
        )
    }
}

fn with_bags<R>(world: &World, player: Entity, apply: impl FnOnce(&Bags) -> R) -> Option<R> {
    let character_id = world.get::<ClientId>(player)?.0;
    let bags = Bags {
        db: world.get_resource::<GameDatabase>()?,
        templates: world.get_resource::<ItemTemplates>()?,
        catalog: world.get_resource::<VendorCatalog>()?,
        character_id,
    };
    Some(apply(&bags))
}

pub(crate) fn new_order_item(
    world: &World,
    player: Entity,
    item_id: u32,
    count: u32,
) -> Result<InventoryItemData, String> {
    with_bags(world, player, |bags| bags.new_item(item_id, count)).ok_or("crafter has no bags")?
}

pub(crate) fn store_created_item(
    world: &mut World,
    player: Entity,
    item_id: u32,
    count: u32,
    origin: Option<shared::protocol::CraftingOrigin>,
) -> Result<(), String> {
    let character_id = with_bags(world, player, |bags| {
        crate::inventory::grant_item_with_origin(
            &bags.db.db,
            bags.templates,
            bags.catalog,
            bags.character_id,
            (item_id, count),
            origin,
        )
        .map(|_| {
            let template = bags
                .templates
                .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
                .expect("committed craft template");
            crate::guild_news::record_item(
                &bags.db.db,
                bags.character_id,
                item_id,
                template,
                world.get_resource::<crate::item_stats::ItemScalingTables>(),
                crate::guild_news::ItemNewsSource::Crafted,
            );
            bags.character_id
        })
    })
    .ok_or("caster has no bags")??;
    let mut updates = world.resource_mut::<GoldUpdateQueue>();
    updates.record_items(character_id);
    updates.record_acquisition(character_id, item_id, count);
    Ok(())
}

/// `Spell::CheckItems` for player casters: reagents and tools in the bags, and
/// room for every created item. Spells without either pass.
pub(crate) fn check_cast_items(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let needs_quality_operation = spell
        .effects
        .iter()
        .any(|effect| matches!(effect.effect, EFFECT_QUALITY_CRAFT | EFFECT_RECRAFT));
    if needs_quality_operation {
        return shared::profession::evaluate_crafted_quality()
            .map(|_| ())
            .map_err(|gap| CastFailure::Unsupported(gap.to_string()));
    }
    let Some(data) = world.get_resource::<ProfessionData>() else {
        return Ok(());
    };
    let order_reagents = crate::crafting_orders::cast_requirements(world, caster, spell)
        .transpose()
        .map_err(CastFailure::Unsupported)?;
    let order_cast = order_reagents.is_some();
    let reagents = order_reagents
        .as_deref()
        .unwrap_or_else(|| data.reagents(spell.id));
    let tools = data.tools(spell.id);
    let created: Vec<(u32, u32)> = spell
        .effects
        .iter()
        .filter(|effect| effect.effect == EFFECT_CREATE_ITEM && effect.item_type != 0)
        .map(|effect| (effect.item_type, effect.base_points.round().max(1.0) as u32))
        .collect();
    if reagents.is_empty() && tools.is_none() && created.is_empty() {
        return Ok(());
    }
    if world.get::<Player>(caster).is_none() {
        return Ok(());
    }
    with_bags(world, caster, |bags| {
        let mut model = bags
            .load()
            .map_err(|detail| CastFailure::Unsupported(format!("bags unavailable: {detail}")))?;
        for &(item, count) in reagents {
            if model.bag_count(Retail, item) < count {
                return Err(CastFailure::Reagents(bags.item_name(item)));
            }
        }
        if let Some(tools) = tools {
            check_tools(data, &model, tools, bags)?;
        }
        // The model is only checked, never saved: store the created items in it.
        for (item, count) in created.into_iter().filter(|_| !order_cast) {
            let item = bags
                .new_item(item, count)
                .map_err(CastFailure::Unsupported)?;
            bags.store(&mut model, item)
                .map_err(|error| match error.reason {
                    shared::protocol::InventoryErrorReason::ItemMaxCount => {
                        CastFailure::Inventory(error)
                    }
                    _ => CastFailure::InventoryFull,
                })?;
        }
        Ok(())
    })
    .unwrap_or_else(|| Err(CastFailure::Unsupported("caster has no bags".into())))
}

fn check_tools(
    data: &ProfessionData,
    model: &InventoryModel,
    tools: &data::Tools,
    bags: &Bags,
) -> Result<(), CastFailure> {
    for &item in &tools.items {
        if model.bag_count(Retail, item) == 0 {
            return Err(CastFailure::Totems(bags.item_name(item)));
        }
    }
    for &category in &tools.categories {
        if !model
            .bag_item_ids(Retail)
            .any(|item| data.item_meets_totem_category(item, category))
        {
            let name = data
                .totem_category(category)
                .map_or_else(|| format!("tool category {category}"), |c| c.name.clone());
            return Err(CastFailure::Totems(name));
        }
    }
    Ok(())
}

/// `Spell::TakeReagents`: remove the spell's reagents from the caster's bags.
pub(crate) fn take_reagents(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Order reagents are consumed atomically with result/commission delivery at effect24.
    if crate::crafting_orders::is_order_cast(world, caster, spell.id) {
        return;
    }
    let Some(reagents) = world
        .get_resource::<ProfessionData>()
        .map(|data| data.reagents(spell.id).to_vec())
    else {
        return;
    };
    if reagents.is_empty() {
        return;
    }
    let taken = with_bags(world, caster, |bags| {
        let mut model = bags.load()?;
        for (item, count) in reagents {
            model.destroy_bag_items(Retail, item, count)?;
        }
        save_inventory_record(&bags.db.db, bags.character_id, &model.into_record());
        Ok::<_, String>(bags.character_id)
    });
    match taken {
        Some(Ok(character_id)) => world
            .resource_mut::<GoldUpdateQueue>()
            .record_items(character_id),
        Some(Err(error)) => warn!("Spell {}: reagents not taken: {error}", spell.id),
        None => {}
    }
}

fn receive_craft_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        &PlayerLink,
        Has<InWorld>,
        &mut MessageReceiver<CraftRecipe>,
    ), With<Authenticated>>();
    let requests: Vec<(Entity, CraftRecipe)> = receivers
        .iter_mut(world)
        .filter(|(_, in_world, _)| *in_world)
        .flat_map(|(link, _, mut receiver)| {
            let player = link.0;
            receiver
                .receive()
                .map(move |request| (player, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (player, request) in requests {
        world.entity_mut(player).remove::<CraftQueue>();
        if crate::spell_cast::cast_for_player(world, player, request.spell_id).is_ok()
            && request.casts > 1
        {
            world.entity_mut(player).insert(CraftQueue {
                spell_id: request.spell_id,
                remaining: request.casts - 1,
            });
        }
    }
}

/// Start the next queued craft once the previous cast has finished.
fn continue_craft_queues(world: &mut World) {
    let mut queues =
        world.query_filtered::<(Entity, &CraftQueue), Without<shared::casting::CastState>>();
    let ready: Vec<(Entity, CraftQueue)> = queues
        .iter(world)
        .map(|(player, queue)| (player, *queue))
        .collect();
    for (player, queue) in ready {
        if queue.remaining <= 1 {
            world.entity_mut(player).remove::<CraftQueue>();
        } else {
            world.entity_mut(player).insert(CraftQueue {
                remaining: queue.remaining - 1,
                ..queue
            });
        }
        if crate::spell_cast::cast_for_player(world, player, queue.spell_id).is_err() {
            world.entity_mut(player).remove::<CraftQueue>();
        }
    }
}

/// Persist changed professions and send the owner its `ProfessionSnapshot`.
pub(crate) fn save_and_send_professions(
    changed: Query<(Entity, &ClientId, &PlayerProfessions), Changed<PlayerProfessions>>,
    mut connections: Query<(&PlayerLink, &mut MessageSender<ProfessionSnapshot>)>,
    db: Res<GameDatabase>,
    data: Res<ProfessionData>,
) {
    for (player, character_id, profs) in &changed {
        persistence::save_professions(&db.db, character_id.0, profs);
        if let Some((_, mut sender)) = connections.iter_mut().find(|(link, _)| link.0 == player) {
            sender.send::<ProfessionChannel>(favorites::load_snapshot(
                &db.db,
                character_id.0,
                profs,
                &data,
            ));
        }
    }
}

#[cfg(test)]
#[path = "profession_tests.rs"]
mod tests;
