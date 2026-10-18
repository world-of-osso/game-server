//! Shared initial-spawn / same-actor template projection (TC Creature::UpdateEntry).
use super::*;
use shared::components::{CombatStatus, Mana, ThreatTable};

/// Source addon path data, separate from the active movement generator's cursor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AddonPathId(pub u32);

/// UnitDefines.h:186; the template cannot clear the actor's real combat state.
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;
/// Existing native mana initialization: paladin and mage class-level rows.
const MANA_CLASSES: [u8; 2] = [2, 8];
const CREATURE_TYPE_CRITTER: u8 = 8;

struct EntryStats {
    level: u8,
    health: f32,
    armor: f32,
    scaling: Option<(LevelScaling, Arc<LevelStats>)>,
}

enum EquipmentUpdate {
    Keep,
    Clear,
    Replace(EquipmentAppearance),
}

pub(crate) struct EntryProjection {
    template: CreatureTemplate,
    origin: CreatureOrigin,
    temporary: bool,
    stats: EntryStats,
    display: (u32, f32),
    unit_flags: u32,
    addon: Option<CreatureAddon>,
    equipment: EquipmentUpdate,
    mana: Option<f32>,
    template_string_id: Option<String>,
}

/// Read context and roll level/model/equipment in the existing spawn order.
/// A caller-selected template is honored; a failed lookup is an explicit refusal.
pub(crate) fn read_entry_projection(
    data: &WorldData,
    template: Option<&CreatureTemplate>,
    context: &mut CreatureSpawnContext<'_>,
    placement: &CreatureSpawn,
    spawn_data: Option<&CreatureSpawn>,
    rng: &mut impl rand::Rng,
) -> Option<EntryProjection> {
    let Some(template) = template else {
        error!(
            "Creature::UpdateEntry: target template {} does not exist",
            placement.template_entry
        );
        return None;
    };
    let tuning = match read_spawn_tuning(data, template, placement, context) {
        Ok(tuning) => tuning,
        Err(MissingSpawnLevelContext) => {
            log_missing_context(template.entry, placement.guid, context);
            return None;
        }
    };
    let stats = roll_entry_stats(data, template, tuning, rng);
    Some(EntryProjection::roll_fields(
        data,
        template,
        context.temporary,
        placement,
        spawn_data,
        stats,
        rng,
    ))
}

fn log_missing_context(entry: u32, guid: u32, context: &CreatureSpawnContext<'_>) {
    error!(
        "CREATURE_LEVEL_CONTEXT_GAP: refusing creature{} spawn{} map{} copy{} difficulty{}",
        entry, guid, context.map.map_id, context.map.instance_id, context.difficulty_id,
    );
}

fn roll_entry_stats(
    data: &WorldData,
    template: &CreatureTemplate,
    tuning: Option<(GuardianLevelRange, Arc<LevelStats>)>,
    rng: &mut impl rand::Rng,
) -> EntryStats {
    let scaling = tuning.map(|(range, stats)| (roll_spawn_scaling(range, rng), stats));
    let (level, health, armor) = match &scaling {
        Some((scaling, stats)) => {
            let level = scaling.native_level();
            (level, stats.max_health(level), stats.armor(level))
        }
        None => (
            template.min_level,
            resolve_health(data, template),
            template.armor,
        ),
    };
    EntryStats {
        level,
        health,
        armor,
        scaling,
    }
}

fn roll_equipment_update(
    data: &WorldData,
    entry: u32,
    placement: &CreatureSpawn,
    spawn_data: Option<&CreatureSpawn>,
    rng: &mut impl rand::Rng,
) -> EquipmentUpdate {
    let equipment_id = spawn_data.map_or(1, |spawn| spawn.equipment_id);
    if equipment_id == 0 {
        return EquipmentUpdate::Clear;
    }
    let selector = CreatureSpawn {
        template_entry: entry,
        equipment_id,
        ..placement.clone()
    };
    creature_equipment(data, &selector, rng).map_or(EquipmentUpdate::Keep, EquipmentUpdate::Replace)
}

fn entry_mana(data: &WorldData, template: &CreatureTemplate, level: u8) -> Option<f32> {
    if !MANA_CLASSES.contains(&template.unit_class) {
        return None;
    }
    let base = data
        .class_level_base_mana
        .get(&(level, template.unit_class))
        .copied()
        .unwrap_or(0);
    Some((base as f32 * template.mana_modifier).round())
}

impl EntryProjection {
    fn roll_fields(
        data: &WorldData,
        template: &CreatureTemplate,
        temporary: bool,
        placement: &CreatureSpawn,
        spawn_data: Option<&CreatureSpawn>,
        stats: EntryStats,
        rng: &mut impl rand::Rng,
    ) -> Self {
        let display = match spawn_data {
            Some(spawn) => choose_display(template, spawn, rng),
            None => template_display(template, rng),
        };
        Self {
            template: template.clone(),
            origin: CreatureOrigin {
                guid: placement.guid,
                original_entry: placement.template_entry,
            },
            temporary,
            display,
            unit_flags: spawn_data
                .and_then(|spawn| spawn.unit_flags)
                .unwrap_or(template.unit_flags),
            addon: data
                .addons
                .get(&placement.guid)
                .or_else(|| data.template_addons.get(&template.entry))
                .cloned(),
            equipment: roll_equipment_update(data, template.entry, placement, spawn_data, rng),
            mana: entry_mana(data, template, stats.level),
            template_string_id: data.template_string_ids.get(&template.entry).cloned(),
            stats,
        }
    }

    /// Spawn records source identity. Updates keep it and never replay AI/spawn setup.
    pub(crate) fn apply(self, mut actor: EntityWorldMut<'_>, initial_spawn: bool) -> bool {
        if !initial_spawn && !self.matches_origin(&actor) {
            return false;
        }
        self.apply_core(&mut actor, initial_spawn);
        self.apply_scaling(&mut actor);
        self.apply_equipment(&mut actor);
        self.apply_pose(&mut actor);
        self.apply_mount_and_path(&mut actor);
        self.apply_mana(&mut actor);
        self.apply_regeneration(&mut actor);
        self.update_template_string_id(&mut actor);
        self.update_movement_rates(&mut actor);
        if initial_spawn && !self.temporary {
            actor.insert(self.origin);
        }
        if !initial_spawn {
            self.update_react_state(&mut actor);
        }
        self.apply_addon_auras(actor, initial_spawn);
        true
    }

    fn matches_origin(&self, actor: &EntityWorldMut<'_>) -> bool {
        let Some(origin) = actor.get::<CreatureOrigin>() else {
            error!(
                "CREATURE_ENTRY_ORIGIN_GAP: actor{} target{} has no ordinary spawn origin",
                actor.id(),
                self.template.entry
            );
            return false;
        };
        if origin.guid != self.origin.guid {
            error!(
                "CREATURE_ENTRY_ORIGIN_GAP: actor{} original{} spawn{} differs from requested spawn{}",
                actor.id(),
                origin.original_entry,
                origin.guid,
                self.origin.guid
            );
            return false;
        }
        true
    }

    fn apply_core(&self, actor: &mut EntityWorldMut<'_>, initial_spawn: bool) {
        let entry = if initial_spawn {
            self.origin.original_entry
        } else {
            self.template.entry
        };
        let flags = self.unit_flags_for(actor, initial_spawn);
        actor.insert((
            Npc {
                template_id: entry,
                name: self.template.name.clone(),
            },
            Health {
                current: self.stats.health,
                max: self.stats.health,
            },
            ModelDisplay {
                display_id: self.display.0,
            },
            NativeObjectScale(self.display.1),
            shared::components::UnitScale(self.display.1),
            UnitLevel(self.stats.level),
            CreatureArmor(self.stats.armor),
            CreatureClassification::from_rank(self.template.rank).expect("validated template rank"),
            UnitFactionTemplate(u32::from(self.template.faction)),
            UnitFlags(flags),
            NpcFlags(self.template.npc_flags),
            UnitPowers::default(),
        ));
    }

    fn unit_flags_for(&self, actor: &EntityWorldMut<'_>, initial_spawn: bool) -> u32 {
        if initial_spawn {
            return self.unit_flags;
        }
        let combat = actor.get::<CombatStatus>().is_some_and(|status| status.0);
        let combat_flag = if combat { UNIT_FLAG_IN_COMBAT } else { 0 };
        (self.unit_flags & !UNIT_FLAG_IN_COMBAT) | combat_flag
    }

    fn apply_scaling(&self, actor: &mut EntityWorldMut<'_>) {
        if let Some((scaling, stats)) = &self.stats.scaling {
            actor.insert((*scaling, ScaledStats(stats.clone())));
        } else {
            actor.remove::<(LevelScaling, ScaledStats)>();
        }
    }

    fn apply_equipment(&self, actor: &mut EntityWorldMut<'_>) {
        match &self.equipment {
            EquipmentUpdate::Keep => {}
            EquipmentUpdate::Clear => {
                actor.remove::<EquipmentAppearance>();
            }
            EquipmentUpdate::Replace(equipment) => {
                actor.insert(equipment.clone());
            }
        }
    }

    fn apply_pose(&self, actor: &mut EntityWorldMut<'_>) {
        let mut pose = actor.get::<UnitPose>().copied().unwrap_or_default();
        pose.sheath_state = SheathState::Melee;
        if let Some(addon) = &self.addon {
            pose.stand_state = addon.stand_state;
            pose.sheath_state = addon.sheath_state;
            if addon.emote_state != 0 {
                pose.emote_state = addon.emote_state;
            }
        }
        actor.insert((pose, SpawnPose(pose)));
    }

    fn apply_mount_and_path(&self, actor: &mut EntityWorldMut<'_>) {
        let Some(addon) = &self.addon else {
            return;
        };
        if addon.mount != 0 {
            let mut mount = actor.get::<Mounted>().cloned().unwrap_or_default();
            mount.mount_display_id = addon.mount;
            actor.insert(mount);
        }
        if addon.path_id != 0
            && actor.get::<AddonPathId>().map(|path| path.0) != Some(addon.path_id)
        {
            actor.insert(AddonPathId(addon.path_id));
        }
    }

    fn apply_mana(&self, actor: &mut EntityWorldMut<'_>) {
        if let Some(max) = self.mana {
            actor.insert((
                Mana { current: max, max },
                crate::spell_cast::powers::PowerPools {
                    order: vec![shared::components::PowerType::Mana],
                    ..Default::default()
                },
            ));
        } else {
            actor.remove::<(Mana, crate::spell_cast::powers::PowerPools)>();
        }
    }

    fn apply_regeneration(&self, actor: &mut EntityWorldMut<'_>) {
        if self.mana.is_some() && self.template.regenerates_power {
            actor
                .entry::<crate::spell_cast::powers::CreatureRegenTimer>()
                .or_default();
        } else {
            actor.remove::<crate::spell_cast::powers::CreatureRegenTimer>();
        }
        if self.template.regenerates_health {
            actor
                .entry::<crate::spell_cast::powers::CreatureHealthRegenTimer>()
                .or_default();
        } else {
            actor.remove::<crate::spell_cast::powers::CreatureHealthRegenTimer>();
        }
    }

    fn update_template_string_id(&self, actor: &mut EntityWorldMut<'_>) {
        let mut ids = actor
            .get::<CreatureStringIds>()
            .cloned()
            .unwrap_or_default();
        ids.template = self.template_string_id.clone();
        if ids == CreatureStringIds::default() {
            actor.remove::<CreatureStringIds>();
        } else {
            actor.insert(ids);
        }
    }

    fn update_movement_rates(&self, actor: &mut EntityWorldMut<'_>) {
        if let Some(mut home) = actor.get_mut::<SpawnPoint>() {
            home.speed_walk = self.template.speed_walk;
            home.speed_run = self.template.speed_run;
        }
        if let Some(mut path) = actor.get_mut::<WaypointState>() {
            path.speed_walk = self.template.speed_walk;
            path.speed_run = self.template.speed_run;
        }
    }

    fn update_react_state(&self, actor: &mut EntityWorldMut<'_>) {
        let state = if self.template.trigger || self.template.creature_type == CREATURE_TYPE_CRITTER
        {
            crate::scripted_ai::ReactState::Passive
        } else {
            crate::scripted_ai::ReactState::Aggressive
        };
        actor.insert(state);
        if self.template.aggro_mode == crate::creature_aggro::AggroMode::Aggressive {
            actor.insert(crate::creature_aggro::AggroRange);
        } else {
            actor.remove::<crate::creature_aggro::AggroRange>();
        }
        if self.template.aggro_mode != crate::creature_aggro::AggroMode::Passive {
            actor.entry::<ThreatTable>().or_default();
        }
    }

    fn apply_addon_auras(self, mut actor: EntityWorldMut<'_>, initial_spawn: bool) {
        let spells = self.addon.map_or_else(Vec::new, |addon| addon.auras);
        if spells.is_empty() {
            actor.remove::<SpawnAddonAuras>();
            return;
        }
        actor.insert(SpawnAddonAuras(spells.clone()));
        if initial_spawn {
            actor.insert(AddonAuras(spells));
            return;
        }
        let creature = actor.id();
        let world = actor.into_world_mut();
        for spell in spells {
            if !crate::auras::has_aura(world, creature, spell) {
                apply_addon_aura(world, creature, spell);
            }
        }
    }
}
