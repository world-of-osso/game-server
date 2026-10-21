use super::effects2_tests::use_ability;
use super::*;

struct AttackDraws(usize);
impl BattleRng for AttackDraws {
    fn draw(&mut self) -> u32 {
        assert!(self.0 < 2, "counter must not rescale crit/variance");
        self.0 += 1;
        500000
    }
}

#[test]
fn pet_battle_last_hit_power_pct_uses_real_hit_and_both_authored_power_values() {
    let catalog = catalog();
    for (ability, expected) in [(997, 120), (2432, 180)] {
        let mut initial = battle(&catalog, 9, 6, [ability, 110, 123]);
        initial.teams[0].pets[0].stats.speed = 50;
        initial.teams[0].pets[0].stats.power = 10000;
        initial.teams[0].pets[0]
            .states
            .insert(catalog.state_id("Stat_CritChance").unwrap(), 100);
        let mut rng = AttackDraws(0);
        let (after, events) = initial
            .step(
                &catalog,
                [Action::Ability(ability), Action::Ability(110)],
                &mut rng,
            )
            .unwrap();
        assert_eq!(rng.0, 2);
        assert_eq!(damage(&events), [120, expected]);
        assert_eq!(after.teams[0].pets[0].health, 9880);
        assert_eq!(after.teams[1].pets[0].health, 10000 - expected);
    }
}

#[test]
fn pet_battle_last_hit_power_pct_preserves_previous_round_hit_and_zero_history() {
    let catalog = catalog();
    for (ability, expected) in [(997, 120), (2432, 180)] {
        let mut initial = battle(&catalog, 9, 6, [ability, 110, 123]);
        initial.teams[0].pets[0].stats.speed = 200;
        let (empty, events) = initial
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut NoDraw,
            )
            .unwrap();
        assert_eq!(damage(&events), [0]);
        assert_eq!(empty.teams[1].pets[0].health, 10000);
        let (hit, _) = initial
            .step(
                &catalog,
                [Action::Pass, Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        let (after, events) = hit
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut NoDraw,
            )
            .unwrap();
        assert_eq!(damage(&events), [expected]);
        assert_eq!(after.teams[0].pets[0].health, 9880);
    }
}

#[test]
fn pet_battle_last_hit_power_pct_reuses_hostile_family_scaling_and_floor() {
    let catalog = catalog();
    for (ability, family, expected) in [(997, 4, 180), (997, 2, 80), (2432, 1, 270), (2432, 7, 120)]
    {
        let mut initial = battle(&catalog, 9, family, [ability, 110, 123]);
        initial.teams[0].pets[0].stats.speed = 50;
        let (_, events) = initial
            .step(
                &catalog,
                [Action::Ability(ability), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        assert_eq!(damage(&events), [120, expected]);
    }
}

#[test]
fn pet_battle_last_hit_power_pct_respects_misses_and_real_wall_blocks() {
    let catalog = catalog();
    for ability in [997, 2432] {
        let mut initial = battle(&catalog, 9, 6, [ability, 110, 123]);
        initial.teams[0].pets[0].stats.speed = 200;
        let (mut hit, _) = initial
            .step(
                &catalog,
                [Action::Pass, Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        hit.teams[1].pets[0]
            .states
            .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
        let (missed, events) = hit
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut NoDraw,
            )
            .unwrap();
        assert_eq!(missed.teams[1].pets[0].health, 10000);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Miss { .. }))
        );
        let (mut prior_hit, _) = initial
            .step(
                &catalog,
                [Action::Pass, Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        prior_hit.teams[1].pets[0].abilities = [479, 110, 123];
        let (shielded, _) = prior_hit
            .step(
                &catalog,
                [Action::Pass, Action::Ability(479)],
                &mut Fixed(500000),
            )
            .unwrap();
        let (blocked, events) = shielded
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut Fixed(500000),
            )
            .unwrap();
        assert!(damage(&events).is_empty());
        assert_eq!(blocked.teams[0].pets[0].health, 9880);
        assert_eq!(blocked.teams[1].pets[0].health, 10000);
        assert_eq!(blocked.auras.len(), 1);
        assert_eq!(blocked.auras[0].charges, 1);
    }
}

#[test]
fn pet_battle_last_hit_power_pct_uses_actual_last_periodic_hit_without_second_power_scaling() {
    let catalog = catalog();
    for (ability, percentage) in [(997, 100), (2432, 150)] {
        let mut initial = battle(&catalog, 9, 6, [ability, 110, 123]);
        initial.teams[0].pets[0].stats.speed = 200;
        initial.teams[1].pets[0].abilities = [152, 110, 123];
        let (hit, events) = initial
            .step(
                &catalog,
                [Action::Pass, Action::Ability(152)],
                &mut Fixed(500000),
            )
            .unwrap();
        let last_periodic_hit = events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::Damage {
                    amount,
                    periodic: true,
                    ..
                } => Some(*amount),
                _ => None,
            })
            .expect("authored Poison Fang ticks this round");
        assert!(last_periodic_hit > 0);
        let (_, events) = use_ability(&hit, &catalog, ability);
        let counter = events
            .iter()
            .find_map(|event| match event {
                Event::Damage {
                    caster: PetRef { team: 0, slot: 0 },
                    amount,
                    periodic: false,
                    ..
                } => Some(*amount),
                _ => None,
            })
            .expect("real counter hit");
        assert_eq!(counter, last_periodic_hit * percentage / 100);
    }
}
