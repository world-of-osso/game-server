//! Immutable gameplay definitions handed between fresh seeded worlds.
use bevy::prelude::*;

// Only definitions: no Entity/SystemId, queues, clocks, histories or instance state.
macro_rules! catalogs {
    ($($field:ident: $ty:path),+ $(,)?) => {
        pub(super) struct Catalogs {
            $($field: Option<$ty>,)+
        }

        impl Catalogs {
            pub(super) fn take(world: &mut World) -> Self {
                Self { $($field: world.remove_resource::<$ty>(),)+ }
            }

            pub(super) fn install(self, world: &mut World) {
                $(
                    // Preserve native absence (e.g. ItemLocks after a loader error).
                    world.remove_resource::<$ty>();
                    if let Some(value) = self.$field {
                        world.insert_resource(value);
                    }
                )+
            }
        }
    };
}

catalogs! {
    world: crate::world_data::WorldData,
    armor: crate::level_scaling::ArmorConstants,
    spells: crate::spell_info::SpellStore,
    classes: crate::spell_info::ClassSpells,
    powers: crate::spell_cast::powers::PowerTypeTable,
    item_scaling: crate::item_stats::ItemScalingTables,
    factions: crate::faction::FactionTemplates,
    maps: crate::maps::MapStore,
    reaches: crate::scripted_ai::model_reach::ModelReaches,
    races: crate::unit_creature_type::RaceCreatureTypes,
    conditions: crate::conditions::SpellTargetConditions,
    followers: crate::followers::FollowerCatalog,
    texts: crate::scripted_ai::creature_text::CreatureTexts,
    smart: crate::scripted_ai::smart_ai::SmartScripts,
    bosses: crate::scripted_ai::bossmod::Catalog,
    aura_scripts: crate::auras::scripts::AuraScripts,
    forms: crate::auras::shapeshift::ShapeshiftForms,
    mounts: crate::auras::mount::MountStore,
    overrides: crate::auras::override_spell_data::OverrideSpellData,
    guardians: crate::spell_cast::guardian::GuardianSpells,
    summons: crate::spell_cast::summon::SummonPropertiesStore,
    destinations: crate::spell_cast::teleport_positions::SpellTargetPositions,
    disenchant: crate::spell_cast::disenchant::DisenchantData,
    locks: crate::spell_cast::open_lock::ItemLocks,
}

#[test]
fn native_felguard_learns_axe_toss_across_cached_seeded_worlds() {
    let mut catalogs = None;
    for _ in 0..2 {
        let mut app = super::runtime::build_cached_app(&mut catalogs);
        let copy = super::runtime::create_copy(&mut app);
        let party = crate::followers::simulator::spawn_party(
            app.world_mut(),
            copy,
            shared::components::Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        )
        .unwrap();
        let warlock =
            crate::followers::native_fixture::spawn(app.world_mut(), party.owner, 230861).unwrap();
        crate::spell_cast::creature_cast(app.world_mut(), warlock, 30146, Some(warlock), false)
            .unwrap();
        for _ in 0..200 {
            app.update();
        }
        let pet =
            crate::hunter_pet::summoned_pet(app.world(), warlock).expect("native30146 Felguard");
        let bar = app
            .world()
            .get::<crate::pet_control::PetCharm>(pet)
            .expect("native pet action bar")
            .action_bar;
        assert!(
            bar.iter()
                .any(|button| shared::protocol::pet_action_button_action(*button) == 89766),
            "the actual level32 Felguard must expose learned Axe Toss in its native action bar, including cached worlds: {bar:?}"
        );
        catalogs = Some(Catalogs::take(app.world_mut()));
    }
}

#[test]
fn seeded_apps_reuse_catalog_allocations_but_reset_dynamic_state() {
    let mut first = super::runtime::build_app();
    let spawn_storage = first
        .world()
        .resource::<crate::world_data::WorldData>()
        .spawns
        .as_ptr();
    let spell_storage = first
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap() as *const _;
    first.world_mut().spawn(shared::components::Health {
        current: 7.0,
        max: 100.0,
    });
    for _ in 0..20 {
        first.update();
    }
    let mut catalogs = Some(Catalogs::take(first.world_mut()));
    let mut second = super::runtime::build_cached_app(&mut catalogs);
    assert_eq!(
        second
            .world()
            .resource::<crate::world_data::WorldData>()
            .spawns
            .as_ptr(),
        spawn_storage,
        "source spawn allocation must move, never reload or clone"
    );
    assert_eq!(
        second
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap() as *const _,
        spell_storage,
        "spell catalog must retain its allocation"
    );
    assert!(
        second
            .world()
            .resource::<Time<Virtual>>()
            .elapsed_secs_f64()
            < 0.1
    );
    assert_eq!(
        second
            .world_mut()
            .query::<&shared::components::Health>()
            .iter(second.world())
            .count(),
        0
    );
    assert!(
        second
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .is_empty()
    );
    assert!(
        second
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap()
            .effects
            .iter()
            .any(|effect| effect.effect == 2)
    );
}
