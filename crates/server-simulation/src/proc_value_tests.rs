//! Native70448 plus audited1291 TDB rows; TC a352b1fa SpellMgr.cpp1497–1908.
use server::spell_info::{
    ProcDefinitionSource, SpellAuraOptions, SpellInfo, SpellProcEntry, SpellStore,
};
use std::sync::OnceLock;

fn spell(id: u32) -> &'static SpellInfo {
    static STORE: OnceLock<SpellStore> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let conn = rusqlite::Connection::open_with_flags(
                server::test_data::world_db_path(),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            server::spell_info::load_spell_data(&conn).unwrap().0
        })
        .get(id)
        .unwrap()
}

fn options(id: u32) -> &'static SpellAuraOptions {
    spell(id).aura_options.as_deref().unwrap()
}

fn entry(id: u32) -> &'static SpellProcEntry {
    options(id)
        .proc_entry
        .as_ref()
        .unwrap_or_else(|| panic!("native spell{id} must expose its effective TC proc entry"))
}

#[test]
#[ignore = "native full-row TDB fixture; raw metadata must remain separate"]
fn native_proc_values_raw_db2_fields_remain_unchanged_control() {
    let native = options(4341);
    assert_eq!(
        native.proc_definition_source,
        Some(ProcDefinitionSource::ExplicitTdb)
    );
    assert_eq!(native.proc_type_mask, [65536, 0]);
    assert_eq!(native.proc_chance, 100);
    assert_eq!(native.proc_charges, 5);
}

#[test]
#[ignore = "native full-row TDB fixture; BodyAndSoul exact effective row"]
fn native_proc_values_body_and_soul_complete_effective_row() {
    assert_eq!(
        entry(64129),
        &SpellProcEntry {
            school_mask: 0,
            spell_family_name: 6,
            spell_family_mask: [1, 0, 524288, 0],
            proc_flags: [16384, 0],
            spell_type_mask: 4,
            spell_phase_mask: 2,
            hit_mask: 3,
            attributes_mask: 0,
            disable_effects_mask: 0,
            procs_per_minute: 0.0,
            chance: 100.0,
            cooldown_ms: 0,
            charges: 0,
        }
    );
}

#[test]
#[ignore = "native full-row TDB fixture; Expurgation scope is not native zero mask"]
fn native_proc_values_expurgation_retains_authored_family_and_damage_scope() {
    let row = entry(383344);
    assert_eq!(row.spell_family_name, 10);
    assert_eq!(row.spell_family_mask, [0, 0, 0, 1073741824]);
    assert_eq!(row.proc_flags, [16, 0]);
    assert_eq!((row.spell_type_mask, row.spell_phase_mask), (1, 2));
    assert_eq!(row.chance, 100.0);
}

#[test]
#[ignore = "native full-row TDB fixture; physical school and authored hit mask"]
fn native_proc_values_predatory_strikes_retains_school_and_hit_scope() {
    let row = entry(48484);
    assert_eq!(row.school_mask, 1);
    assert_eq!(row.spell_family_name, 7);
    assert_eq!(row.spell_family_mask, [4096, 0, 0, 0]);
    assert_eq!(row.proc_flags, [16, 0]);
    assert_eq!(row.hit_mask, 1027);
    assert_eq!((row.spell_type_mask, row.spell_phase_mask), (1, 1));
}

#[test]
#[ignore = "native full-row TDB fixture; legacy PPM suppresses native chance default"]
fn native_proc_values_legacy_ppm_does_not_inherit_native_hundred_percent_chance() {
    assert_eq!(options(15277).proc_chance, 100);
    let row = entry(15277);
    assert_eq!(row.procs_per_minute, 6.0);
    assert_eq!(row.chance, 0.0);
    assert_eq!(row.proc_flags, [20, 0]);
}

#[test]
#[ignore = "native full-row TDB fixture; TC cooldown units are milliseconds"]
fn native_proc_values_explicit_recovery_is_not_native_recovery_or_seconds() {
    assert_eq!(options(16620).proc_category_recovery_ms, 30000);
    let row = entry(16620);
    assert_eq!(row.cooldown_ms, 30);
    assert_eq!(row.chance, 5.0);
}

#[test]
#[ignore = "native full-row TDB fixture; authored chance wins over DB2"]
fn native_proc_values_explicit_chance_wins_over_native_chance() {
    assert_eq!(options(16864).proc_chance, 100);
    let row = entry(16864);
    assert_eq!(row.chance, 6.0);
    assert_eq!(row.cooldown_ms, 10);
}

#[test]
#[ignore = "native full-row TDB fixture; zero charges inherit DB2, nonzero flags override"]
fn native_proc_values_native_charges_default_but_explicit_flags_win() {
    let row = entry(4341);
    assert_eq!(row.charges, 5);
    assert_eq!(row.chance, 100.0);
    assert_eq!(row.proc_flags, [4096, 0]);
}

#[test]
#[ignore = "native full-row TDB fixture; second flags, cast phase and explicit charges"]
fn native_proc_values_second_flags_and_zero_phase_use_tc_cast_default() {
    assert_eq!(options(48108).proc_type_mask, [0, 0]);
    let row = entry(48108);
    assert_eq!(row.proc_flags, [0, 4]);
    assert_eq!(row.spell_phase_mask, 1);
    assert_eq!(row.charges, 1);
    assert_eq!(row.chance, 101.0);
    assert_eq!(row.attributes_mask, 8);
}

#[test]
#[ignore = "native generated FranticMomentum retains canonical defaults"]
fn native_proc_values_generated_frantic_momentum_remains_defined() {
    assert_eq!(
        options(391875).proc_definition_source,
        Some(ProcDefinitionSource::Generated)
    );
    let row = entry(391875);
    assert_eq!(row.proc_flags, [1040, 0]);
    assert_eq!(row.spell_family_name, 0);
    assert_eq!(row.spell_family_mask, [0; 4]);
    assert_eq!((row.spell_type_mask, row.spell_phase_mask), (7, 2));
    assert_eq!(row.chance, 100.0);
    assert_eq!(row.cooldown_ms, 100);
}

fn assert_generated_hit(id: u32, hit: u32) {
    assert_eq!(
        options(id).proc_definition_source,
        Some(ProcDefinitionSource::Generated)
    );
    assert_eq!(entry(id).hit_mask, hit);
}

#[test]
#[ignore = "native generated reflect hit scope"]
fn native_proc_values_generated_reflect_only_hit_mask() {
    assert_generated_hit(57643, 0x800);
}

#[test]
#[ignore = "native generated critical-only charge scope"]
fn native_proc_values_generated_critical_only_hit_mask() {
    assert_generated_hit(40087, 2);
}

#[test]
#[ignore = "native generated block-only charge scope"]
fn native_proc_values_generated_block_only_hit_mask() {
    assert_generated_hit(69580, 0x40);
}

#[test]
#[ignore = "native generated missed-attack-only scope"]
fn native_proc_values_generated_miss_only_hit_mask() {
    assert_generated_hit(62052, 4);
}

#[test]
#[ignore = "native full-row TDB fixture; authored effect-disable bits"]
fn native_proc_values_preserves_effect_disable_mask() {
    let row = entry(32045);
    assert_eq!(row.disable_effects_mask, 1);
    assert_eq!(row.proc_flags, [65552, 0]);
    assert_eq!((row.spell_type_mask, row.spell_phase_mask), (0, 2));
}

#[test]
#[ignore = "native ordered TDB update; second family-mask word"]
fn native_proc_values_ordered_patch_preserves_second_family_word() {
    let row = entry(1259886);
    assert_eq!(row.spell_family_name, 5);
    assert_eq!(row.spell_family_mask, [0, 272, 0, 0]);
    assert_eq!(row.proc_flags, [0, 4]);
    assert_eq!((row.spell_type_mask, row.spell_phase_mask), (4, 1));
    assert_eq!(row.chance, 101.0);
}
