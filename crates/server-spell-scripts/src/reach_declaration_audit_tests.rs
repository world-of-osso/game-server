//! Site-level audit of script target-selection entry points.
//!
//! This is a deliberately bounded source scan, not a Rust semantic analysis. It
//! catches new calls to the named helpers and component queries, including added
//! calls in an existing function. It cannot recognize arbitrary aliases, custom
//! iterators, or target selection hidden in other crates; behavior tests remain
//! necessary. Each exception states why the site cannot acquire an idle enemy.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::combat::spell_cast::script_registry::SpellScriptHook;

type Site = (String, String, String);
type Sites = BTreeMap<Site, usize>;

// These sites are covered by their registered module's DeclaredReach hook, or
// by AdditionalReachActions for autonomous pet actions.
const DECLARED: &[(&str, &str, &str, usize)] = &[
    ("affliction_shared_agony.rs", "select_twin", "area", 1),
    ("demonology_pet_actions.rs", "felstorm", "query", 1),
    ("destruction_roaring_blaze.rs", "nearby_enemies", "query", 1),
    (
        "discipline_atonement.rs",
        "void_discharge_target",
        "query",
        1,
    ),
    (
        "elemental_ascendance.rs",
        "select_flame_shock_eligible_enemies",
        "area",
        1,
    ),
    ("elemental_rod.rs", "apply_quake_rod", "area", 1),
    ("evoker_flames.rs", "leaping_extras", "area", 1),
    ("feral_growth.rs", "apply_twin_sprouts", "area", 1),
    ("feral_unseen.rs", "choose_enemy", "area", 1),
    ("feral_unseen.rs", "read_attack", "area", 1),
    ("frost_death_knight.rs", "launch_fury_damage", "area", 1),
    ("frost_deathbringer.rs", "trigger_soul_rupture", "area", 1),
    ("frost_deathbringer.rs", "launch_wave", "area", 1),
    ("guardian_flashing_claws.rs", "after_thrash", "area", 1),
    ("guardian_galactic.rs", "copy_twin_moonfire", "area", 1),
    ("guardian_impacts.rs", "select_capped_units", "area", 1),
    ("guardian_rampant_thorn.rs", "tick", "area", 2),
    ("guardian_wild_guardian.rs", "spirit_attack", "area", 1),
    ("havoc/children.rs", "apply_hunt_secondary_dots", "area", 1),
    ("hunter_bear.rs", "select_rend_targets", "area", 1),
    ("hunter_bounces.rs", "schedule_bounces", "area", 1),
    ("hunter_howl.rs", "stampede_plan", "area", 1),
    ("marksmanship.rs", "select_trick_targets", "area", 1),
    ("marksmanship_explosive.rs", "cleave_plan", "area", 1),
    ("outlaw_spree.rs", "choose_target", "area", 1),
    ("paladin_sacred_weapon.rs", "units_around", "area", 1),
    ("ret_damage.rs", "area_plan", "area", 1),
    ("secondary_copies.rs", "read_secondary_targets", "area", 1),
    ("shadow_bolts.rs", "read_recipients", "area", 1),
    ("shadow_collapse.rs", "plan_burst", "area", 1),
    ("shadow_link.rs", "read_vampiric_touch_targets", "query", 1),
    ("shadow_rift.rs", "plan_pulse", "area", 1),
    ("spellslinger_hits.rs", "select_secondaries", "area", 1),
    ("subtlety_goremaw.rs", "after_damage", "area", 1),
    ("survival_wildfire.rs", "select_recipients", "area", 1),
    ("unholy_necrotic_coil.rs", "enemies", "area", 1),
    ("unholy_undeath.rs", "spread", "area", 1),
];

// Exceptions are explicit and site-specific. Counts make new calls in the same
// function fail the audit instead of silently inheriting an old exception.
const ALLOWED: &[(&str, &str, &str, usize, &str)] = &[
    (
        "affliction_agony.rs",
        "active_agonies",
        "query",
        1,
        "counts existing owned Agony",
    ),
    (
        "affliction_empowerment.rs",
        "count_active_stacks",
        "query",
        1,
        "counts existing owned effects",
    ),
    (
        "affliction_nightfall.rs",
        "active_dots",
        "query",
        1,
        "counts existing owned dots",
    ),
    (
        "affliction_presence.rs",
        "has_living_darkglare",
        "query",
        1,
        "finds an owned guardian",
    ),
    (
        "affliction_wrath.rs",
        "land_haunt",
        "query",
        1,
        "removes prior owned Haunt",
    ),
    (
        "affliction_wrath.rs",
        "owned_haunt",
        "query",
        1,
        "selects an existing owned Haunt",
    ),
    (
        "arcane_familiar.rs",
        "tick_familiars",
        "query",
        1,
        "finds owned familiars",
    ),
    (
        "assassination_focus.rs",
        "find_owned_mark",
        "query",
        1,
        "finds an existing owned mark",
    ),
    (
        "assassination_runtime/rogue_hero.rs",
        "apply_mark",
        "query",
        1,
        "removes existing owned marks",
    ),
    (
        "blood_ground.rs",
        "sync_fields",
        "query",
        2,
        "finds existing fields and their owners",
    ),
    (
        "blood_rune_weapon.rs",
        "copy_weapon_actions",
        "query",
        1,
        "finds owned rune weapons",
    ),
    (
        "brewmaster_niuzao.rs",
        "active_niuzao",
        "query",
        1,
        "finds an owned guardian",
    ),
    (
        "brewmaster_training.rs",
        "update",
        "query",
        1,
        "finds self-owned training auras",
    ),
    (
        "demon_hunter_otherworldly_focus.rs",
        "multiplier",
        "area",
        1,
        "counts enemies; does not target or damage them",
    ),
    (
        "demonology_hellbent.rs",
        "reconcile",
        "query",
        1,
        "counts owned pets",
    ),
    (
        "destruction_echo.rs",
        "collect_owned_dot_targets",
        "query",
        1,
        "selects existing owned dots",
    ),
    (
        "destruction_incinerate_targets.rs",
        "find_owned_havoc",
        "query",
        1,
        "selects an existing owned Havoc",
    ),
    (
        "discipline_atonement.rs",
        "select_twinsight_recipient",
        "query",
        1,
        "selects allies for healing",
    ),
    (
        "discipline_atonement.rs",
        "transfer_atonement",
        "query",
        1,
        "heals existing Atonement recipients",
    ),
    (
        "elemental_primals.rs",
        "active_elemental",
        "query",
        1,
        "finds an owned elemental",
    ),
    (
        "enhancement/enhancement_nova.rs",
        "select_own_flame_shocks",
        "query",
        1,
        "selects existing owned Flame Shocks",
    ),
    (
        "enhancement/enhancement_rod.rs",
        "mirror_existing_rods",
        "query",
        1,
        "selects existing owned Lightning Rods",
    ),
    (
        "evoker_bombardments.rs",
        "extend_marks",
        "query",
        1,
        "extends existing owned marks",
    ),
    (
        "evoker_command_squadron.rs",
        "owned_commandos",
        "query",
        1,
        "finds owned summons",
    ),
    (
        "feral_growth.rs",
        "count_owned_dots",
        "query",
        1,
        "counts existing owned dots",
    ),
    (
        "fire_phoenix.rs",
        "owned_phoenixes",
        "query",
        1,
        "finds owned summons",
    ),
    (
        "fire_procs.rs",
        "closest_meteor_target",
        "query",
        1,
        "only modifies damage to an already selected Meteor target",
    ),
    (
        "frost_death_knight.rs",
        "frost_fever_runic_power",
        "query",
        1,
        "counts existing owned Frost Fever",
    ),
    (
        "guardian_blood_frenzy.rs",
        "earn_rage",
        "query",
        1,
        "counts existing owned Thrash",
    ),
    (
        "hellcaller_counters.rs",
        "find_owned_wither_targets",
        "query",
        1,
        "selects existing owned Wither",
    ),
    (
        "holy_runtime/holy_priest.rs",
        "distribute_piety",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "holy_runtime/prayer_healing.rs",
        "select_prayer_allies",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "hunter_howl.rs",
        "begin_combat",
        "query",
        1,
        "finds owners with their own Howl aura",
    ),
    (
        "hunter_wildspeaker.rs",
        "read_owned_critters",
        "query",
        1,
        "finds owned summons",
    ),
    (
        "lava_surge.rs",
        "active_flame_shocks",
        "query",
        1,
        "counts existing owned Flame Shock",
    ),
    (
        "mistweaver_ancient_teachings.rs",
        "heal_from_damage",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "preservation_empower.rs",
        "spirit_targets",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "protection_fray.rs",
        "update",
        "query",
        1,
        "counts nearby units for a self buff; does not attack them",
    ),
    (
        "restoration_ancestral_awakening.rs",
        "awaken",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "restoration_everbloom.rs",
        "blooming_frenzy",
        "query",
        1,
        "selects existing owned Lifebloom allies",
    ),
    (
        "restoration_everbloom.rs",
        "everbloom",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "restoration_tidewaters.rs",
        "heal_riptides",
        "area",
        1,
        "selects allies for healing",
    ),
    (
        "shadow_torment.rs",
        "count_owned_pain",
        "query",
        1,
        "counts existing owned Shadow Word Pain",
    ),
    (
        "soulcoiler_ritual.rs",
        "select_recipient",
        "query",
        1,
        "selects allies for shields",
    ),
    (
        "unholy_commander.rs",
        "grant_commander",
        "query",
        1,
        "finds owned summons",
    ),
    (
        "unholy_grave_mastery.rs",
        "initialize_controlled_ghoul",
        "query",
        1,
        "finds owned ghouls",
    ),
    (
        "unholy_permanent_claws.rs",
        "update",
        "query",
        1,
        "finds owned ghouls",
    ),
    (
        "unholy_pet_mastery.rs",
        "update",
        "query",
        2,
        "finds owned pets and summons",
    ),
    (
        "vengeance_fallout.rs",
        "fallout",
        "area",
        1,
        "counts enemies for fragment chance; does not attack them",
    ),
];

fn function_name(line: &str) -> Option<&str> {
    let start = line.find("fn ")? + 3;
    let tail = &line[start..];
    let end = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    (end > 0).then_some(&tail[..end])
}

fn scan_source(path: &str, source: &str, sites: &mut Sites) {
    let lines: Vec<_> = source.lines().collect();
    let mut function = "<module>";
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        if let Some(name) = function_name(line) {
            function = name;
        }
        let areas = line.matches("area_units(").count() + line.matches("chain_targets(").count();
        if areas > 0 {
            *sites
                .entry((path.into(), function.into(), "area".into()))
                .or_default() += areas;
        }
        let query = ["query::<", "query_filtered::<", "try_query::<"]
            .iter()
            .any(|name| line.contains(name));
        if !query {
            continue;
        }
        let signature = lines[index..lines.len().min(index + 5)].join(" ");
        let signature = signature.split(">()").next().unwrap_or(&signature);
        let components = [
            "Health",
            "ActiveAuras",
            "CombatStatus",
            "Position",
            "Npc",
            "InMap",
        ];
        if components.iter().any(|name| signature.contains(name))
            || line.contains("query::<Entity>")
        {
            *sites
                .entry((path.into(), function.into(), "query".into()))
                .or_default() += 1;
        }
    }
}

fn scan_directory(root: &Path, dir: &Path, sites: &mut Sites) {
    for entry in std::fs::read_dir(dir).expect("read script modules") {
        let path = entry.expect("module entry").path();
        if path.is_dir() {
            scan_directory(root, &path, sites);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path.strip_prefix(root).unwrap().to_str().unwrap();
            scan_source(
                relative,
                &std::fs::read_to_string(&path).expect("read script source"),
                sites,
            );
        }
    }
}

fn module_hooks(path: &str) -> Option<&'static [SpellScriptHook]> {
    let exact = format!("modules/{path}");
    if let Some((_, hooks)) = super::modules::NAMED_MODULES
        .iter()
        .find(|(name, _)| *name == exact)
    {
        return Some(hooks);
    }
    let parent = Path::new(path).parent()?.to_str()?;
    let parent_module = format!("modules/{parent}.rs");
    super::modules::NAMED_MODULES
        .iter()
        .find(|(name, _)| *name == parent_module)
        .map(|(_, hooks)| *hooks)
}

#[test]
fn all_scanned_target_selection_sites_have_declarations_or_explanations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules");
    let mut actual = Sites::new();
    scan_directory(&root, &root, &mut actual);
    let mut expected = Sites::new();
    for &(file, function, kind, count) in DECLARED {
        let hooks = module_hooks(file).unwrap_or_else(|| panic!("no registered module for {file}"));
        assert!(
            hooks.iter().any(|hook| matches!(
                hook,
                SpellScriptHook::DeclaredReach(_) | SpellScriptHook::AdditionalReachActions(_)
            )),
            "{file}::{function} lacks a registered reach declaration"
        );
        assert!(
            expected
                .insert((file.into(), function.into(), kind.into()), count)
                .is_none()
        );
    }
    for &(file, function, kind, count, reason) in ALLOWED {
        assert!(
            !reason.is_empty(),
            "{file}::{function} needs an exception reason"
        );
        assert!(
            expected
                .insert((file.into(), function.into(), kind.into()), count)
                .is_none()
        );
    }
    assert_eq!(
        actual, expected,
        "audit every new or removed target-selection site"
    );
}

#[test]
fn scanner_detects_new_sites_in_existing_functions() {
    let mut sites = Sites::new();
    scan_source(
        "sample.rs",
        "fn cast() {\n targets::area_units(world);\n targets::chain_targets(world);\n world.query::<(Entity, &Health)>();\n}",
        &mut sites,
    );
    assert_eq!(
        sites.get(&("sample.rs".into(), "cast".into(), "area".into())),
        Some(&2)
    );
    assert_eq!(
        sites.get(&("sample.rs".into(), "cast".into(), "query".into())),
        Some(&1)
    );
}
