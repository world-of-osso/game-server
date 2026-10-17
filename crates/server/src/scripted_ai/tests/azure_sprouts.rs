//! TC a352b1fa374364 once-effect destinations and native374362→374360 summons.
use super::*;

const SPROUT: u32 = 190509;
const ARCANE_POWER: u32 = 374736;
const ORIGINS: [(f32, f32, f32); 7] = [
    (-5129.39, 1253.30, 555.58),
    (-5101.68, 1253.71, 555.90),
    (-5114.70, 1230.28, 555.89),
    (-5141.62, 1230.33, 555.83),
    (-5155.62, 1253.60, 555.87),
    (-5141.42, 1276.70, 555.89),
    (-5114.78, 1277.42, 555.87),
];

pub(super) fn prepare_sprout_templates(h: &mut Harness) {
    let (_directory, path) = prepare_encounter_database("leymor");
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = h.app.world().resource::<WorldData>().expected_stats.clone();
    let mut templates = crate::world_data::load_templates(&connection, &expected);
    templates.retain(|entry, _| *entry == SPROUT);
    assert_eq!(
        templates.len(),
        1,
        "authentic runtime sprout template prerequisite"
    );
    crate::world_data::load_template_models(&connection, &mut templates);
    let difficulties =
        crate::world_data::load_difficulty_templates(&connection, &expected, &templates);
    let mut data = h.app.world_mut().resource_mut::<WorldData>();
    data.templates.extend(templates);
    data.difficulty_templates.extend(difficulties);
    data.summon_levels = crate::world_data::summon_levels::SummonLevels::load(&connection)
        .expect("prepared native summon level inputs");
}

pub(super) fn install_azure_ground(h: &mut Harness, map: InMap) {
    let wdt = h.app.world().resource::<MapStore>().maps[&2515].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(2515, wdt)]),
    ));
    for (x, y, z) in ORIGINS {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        let probe = h
            .app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .ground_in(map, Vec3::new(x, y, z));
        assert!(
            matches!(probe, crate::ground::GroundProbe::Supported(_)),
            "authentic origin floor prerequisite: {probe:?}"
        );
    }
}

fn prove_native_summon_prerequisite(h: &mut Harness, leymor: Entity) {
    let mut spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(374360)
        .expect("native sprout summon spell")
        .clone();
    let (x, y, z) = ORIGINS[0];
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    spell.cast_destination = Some([x, y, z]);
    crate::spell_cast::execute_cast(h.app.world_mut(), leymor, &spell, None)
        .expect("native ground-projected summon prerequisite");
    h.advance(0.1);
    let children = h.creatures_of(SPROUT);
    assert_eq!(
        children.len(),
        1,
        "native summon fixture works before parent RED"
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(children[0])
            .expect("native prerequisite child owner")
            .owner,
        leymor
    );
    // Remove only the setup control, before observing the parent cast's new actors.
    h.app.world_mut().despawn(children[0]);
}

#[test]
fn azure_leymor_sprouts_native_children_apply_their_authored_spawn_aura() {
    let (mut h, leymor) = super::azure_combat::prepare_leymor_for_later_clock();
    h.advance(0.1);
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    let children = h.creatures_of(SPROUT);
    assert_eq!(
        children.len(),
        14,
        "ordinary native children before aura RED"
    );
    for &child in &children {
        assert_eq!(h.app.world().get::<InMap>(child), Some(&map));
        assert_eq!(
            h.app
                .world()
                .get::<crate::spell_cast::summon::SummonedCreature>(child)
                .unwrap()
                .owner,
            leymor,
            "actual native summoner prerequisite"
        );
    }
    let missing: Vec<_> = children
        .into_iter()
        .filter(|&child| !h.has_aura(child, 374161))
        .collect();
    assert!(
        missing.is_empty(),
        "native374161 self-aura missing on {missing:?}"
    );
}

pub(super) fn prepare_arcane_power_prerequisites() -> (Harness, Entity) {
    let (mut h, leymor) = super::azure_combat::prepare_leymor_for_later_clock();
    h.advance(0.1);
    let children = h.creatures_of(SPROUT);
    assert_eq!(
        children.len(),
        14,
        "ordinary native summons before power RED"
    );
    for child in children {
        assert!(
            h.has_aura(child, 374161),
            "actual spawn self-aura prerequisite"
        );
        assert_eq!(
            h.app
                .world()
                .get::<crate::spell_cast::summon::SummonedCreature>(child)
                .unwrap()
                .owner,
            leymor
        );
    }
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ARCANE_POWER)
        .expect("native Arcane Power data");
    assert_eq!(
        spell.aura_options.as_ref().unwrap().cumulative_aura,
        99,
        "native stack-capacity prerequisite, not a manufactured budget"
    );
    (h, leymor)
}

#[test]
fn azure_leymor_sprout_power_adds_one_native_aura_count_per_real_child() {
    let (mut h, leymor) = prepare_arcane_power_prerequisites();
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14,
        "one native Arcane Power application per actual ordinary summon"
    );
    let children = h.creatures_of(SPROUT);
    let view = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(leymor)
        .unwrap();
    let powers: Vec<_> = view
        .auras
        .iter()
        .filter(|aura| aura.spell_id == ARCANE_POWER)
        .collect();
    assert_eq!(powers.len(), 1, "native shared slot is one replicated aura");
    assert_eq!(powers[0].stacks, 14);
    assert!(
        children
            .iter()
            .any(|child| Some(child.to_bits()) == powers[0].caster),
        "preserve a real sprout caster, never recast from Leymor"
    );
    for child in children {
        assert!(!h.has_aura(child, ARCANE_POWER), "not the casting sprout");
    }
    for &(player, _, _) in &h.players {
        assert!(!h.has_aura(player, ARCANE_POWER), "not a nearby player");
    }
}

#[test]
fn azure_leymor_sprout_power_unowned_caster_does_not_use_explicit_or_self() {
    let (mut h, leymor) = prepare_arcane_power_prerequisites();
    assert!(
        h.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(leymor)
            .is_none()
    );
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ARCANE_POWER)
        .unwrap()
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let explicit = h.player();
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        leymor,
        Some(explicit),
        &spell,
        effect,
        0,
        true,
    )
    .expect("native summoner92 selection resolves without an invented target");
    assert!(
        selected.is_empty(),
        "an unowned caster has no summoner target"
    );
}

#[test]
fn azure_leymor_sprouts_parent_summons_two_native_children_per_authored_origin() {
    let (mut h, leymor) = super::azure_combat::wake_leymor();
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    prepare_sprout_templates(&mut h);
    install_azure_ground(&mut h, map);
    assert!(
        h.creatures_of(SPROUT).is_empty(),
        "producer inserts no placements"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(leymor).insert(visibility);
    prove_native_summon_prerequisite(&mut h, leymor);
    // Observe the ordinary3s cast, not a manual parent plus the natural combat clock.
    h.engage(leymor);
    h.advance(6.0);
    let parents = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == leymor.to_bits() && go.spell_id == 374364)
        .count();
    assert_eq!(parents, 1, "one completed ordinary native parent cast");
    let children = h.creatures_of(SPROUT);
    assert_eq!(
        children.len(),
        14,
        "seven source groups, two actual native summons each"
    );
    for child in &children {
        assert_eq!(h.app.world().get::<InMap>(*child), Some(&map));
        assert_eq!(
            h.app
                .world()
                .get::<crate::spell_cast::summon::SummonedCreature>(*child)
                .expect("native summon owner")
                .owner,
            leymor
        );
    }
    let positions: Vec<_> = children
        .into_iter()
        .map(|child| h.wow_position(child))
        .collect();
    for (x, y, _) in ORIGINS {
        let origin = Vec2::new(x, y);
        let count = positions
            .iter()
            .filter(|position| Vec2::new(position.x, position.y).distance(origin) <= 7.001)
            .count();
        assert_eq!(count, 2, "native target86/radius37 group at {origin:?}");
    }
}
