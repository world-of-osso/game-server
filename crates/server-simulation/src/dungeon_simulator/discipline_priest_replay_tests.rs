use std::path::Path;

#[test]
#[ignore = "requires the selected real Discipline gear and canonical world database"]
fn discipline_soulcoiler_real_item_channels_five_fractional_barriers() {
    use super::super::dps_parity::literal_rotation::{
        Driver, equipment::Equipment, execution::Prepared,
    };
    use crate::simc_apl_native::Bindings;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use server::simc_apl::Program;
    use shared::protocol::CombatLogKind;
    use std::collections::BTreeMap;

    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let equipment = Equipment::load(run.app.world(), healer, &mut Bindings::default()).unwrap();
    let item = equipment
        .uses()
        .find(|(slot, _)| *slot == "trinket1")
        .expect("equipped Soulcoiler")
        .1;
    assert_eq!(item.item_id, 270162);
    assert_eq!(item.on_use.spell_id, 1291894);
    let prepared = Prepared::read(
        run.app.world_mut(),
        healer,
        &run.targets,
        &equipment,
        std::iter::empty(),
        false,
    )
    .unwrap();
    let mut driver = Driver::new(Program::parse(&[("default", "use_item,slot=trinket1")]).unwrap());
    let (action, execution) = driver
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap()
        .expect("real item is ready");
    driver
        .execute(run.app.world_mut(), healer, action, execution)
        .unwrap();
    for _ in 0..55 {
        run.app.update();
    }
    let mut members = party.members.clone();
    members.push(tank);
    let shielded: Vec<_> = members
        .into_iter()
        .filter(|unit| crate::auras::has_aura(run.app.world(), *unit, 1307578))
        .collect();
    assert_eq!(
        shielded.len(),
        5,
        "five channel ticks shield distinct allies"
    );
    let target = shielded[0];
    let fire = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: run.targets[0],
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(
        run.app.world_mut(),
        &ctx,
        target,
        1_000.0,
        false,
        false,
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hit = events
        .iter()
        .rev()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(133)
                && event.target == Some(target.to_bits())
        })
        .expect("actual incoming hit");
    assert!(
        (hit.absorbed - hit.amount).abs() <= 1 && hit.absorbed > 0,
        "Soulcoil absorbs half the post-modifier hit: {hit:?}"
    );
}
#[test]
#[ignore = "requires MID2 profiles, canonical world database and SimC trace"]
fn discipline_priest_heals_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(256, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let casts = report["healer_casts"].as_object().unwrap();
    let total: u64 = casts.values().map(|value| value.as_u64().unwrap()).sum();
    assert!(
        total >= 200,
        "Discipline policy spent too much time idle: {report}"
    );
    assert!(
        casts
            .get("585")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "Discipline did not cast Smite: {report}"
    );
    assert!(
        metrics["spell_healing"]["94472"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "{report}"
    );
    assert!(
        casts
            .get("1291894")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "equipped Soulcoiler must be used by the replay: {report}"
    );
    assert!(
        casts
            .get("186263")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "selected Shadow Mend must cast while applying Atonement: {report}"
    );
    assert!(
        casts
            .get("32379")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "fixed-time execute must enable Shadow Word: Death: {report}"
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("NATIVE_DISCIPLINE_PRIEST={report}");
}
