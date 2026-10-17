use super::rules::{self, RANDOM_SCALE};
use super::{Action, Aura, Battle, BattleRng, Catalog, Event, Outcome, PetRef};

const RULE_STATES: &[&str] = &[
    "Stat_CritChance",
    "Stat_Accuracy",
    "Stat_Dodge",
    "Mod_SpeedPercent",
    "Mod_MaxHealthPercent",
    "Weather_ArcaneStorm",
    "Mod_PetType_ID",
    "Mod_PetTypeDamageDealtPercent",
    "Mod_PetTypeDamageTakenPercent",
    "Passive_Elemental",
    "Stat_Speed",
    "Last_HitDealt",
    "Last_HitTaken",
    "Condition_WasDamagedThisTurn",
    "Condition_DidDamageThisRound",
    "Ramping_DamageID",
    "Ramping_DamageUses",
    "Mod_DamageDealtPercent",
    "Mod_DamageTakenPercent",
    "Mod_HealingDealtPercent",
    "Mod_HealingTakenPercent",
    "Add_FlatDamageDealt",
    "Add_FlatDamageTaken",
    "Passive_Beast",
    "Passive_Critter",
    "Passive_Flying",
    "Passive_Magic",
    "Passive_Undead",
    "Passive_Mechanical",
    "Passive_Humanoid",
    "Passive_Aquatic",
    "Passive_Dragon",
    "turnLock",
    "swapOutLock",
    "swapInLock",
    "untargettable",
    "unkillable",
    "resilient",
    "Mechanic_IsStunned",
    "Mechanic_IsWebbed",
];

impl Battle {
    /// Immutable input; returns a new state and ordered semantic events.
    /// Rejected decisions/admission do not consume the injected RNG.
    pub fn step(
        &self,
        catalog: &Catalog,
        actions: [Action; 2],
        rng: &mut impl BattleRng,
    ) -> Result<(Self, Vec<Event>), String> {
        if self.outcome.is_some() {
            return Err("battle already finished".into());
        }
        for name in RULE_STATES {
            catalog.state_id(name)?;
        }
        if actions.contains(&Action::Forfeit) {
            let outcome = match actions {
                [Action::Forfeit, Action::Forfeit] => Outcome::Draw,
                [Action::Forfeit, _] => Outcome::Winner(1),
                _ => Outcome::Winner(0),
            };
            let mut next = self.clone();
            next.outcome = Some(outcome);
            return Ok((next, vec![Event::Finished(outcome)]));
        }
        let mut rt = Runtime {
            battle: self.clone(),
            catalog,
            rng,
            events: Vec::new(),
            healing_target: None,
            used_ability: None,
        };
        if rt.replace_fainted(actions)? {
            return Ok((rt.battle, rt.events));
        }
        rt.begin_round();
        for (team, action) in actions.iter().enumerate() {
            rt.validate_action(team, *action)?;
        }
        for aura in &rt.battle.auras {
            rules::validate_ability(catalog, aura.ability)?;
        }
        rt.execute_round(actions)?;
        Ok((rt.battle, rt.events))
    }
}

pub(super) struct Runtime<'a, R> {
    pub battle: Battle,
    pub catalog: &'a Catalog,
    pub rng: &'a mut R,
    pub events: Vec<Event>,
    pub healing_target: Option<PetRef>,
    pub used_ability: Option<u32>,
}
impl<R: BattleRng> Runtime<'_, R> {
    pub fn state(&self, pet: PetRef, name: &str) -> i32 {
        let id = self
            .catalog
            .state_id(name)
            .expect("rule metadata checked before round");
        self.battle.pet(pet).state(id)
    }
    pub fn percent(&self, pet: PetRef, name: &str) -> f64 {
        1.0 + f64::from(self.state(pet, name)) / 100.0
    }
    pub fn roll(&mut self, chance: i32) -> bool {
        if chance >= 100 {
            return true;
        }
        if chance <= 0 {
            return false;
        }
        let draw = self.draw();
        draw < chance as u32 * (RANDOM_SCALE / 100)
    }
    pub fn draw(&mut self) -> u32 {
        let draw = self.rng.draw();
        assert!(draw < RANDOM_SCALE, "BattleRng must draw in [0,1_000_000)");
        draw
    }
    pub fn variance(&mut self, amount: f64, width: i32) -> f64 {
        if width == 0 {
            return amount;
        }
        let unit = f64::from(self.draw()) / f64::from(RANDOM_SCALE);
        amount * (1.0 + (unit - 0.5) * f64::from(width) / 100.0)
    }
    fn replace_fainted(&mut self, actions: [Action; 2]) -> Result<bool, String> {
        let dead = [0, 1].map(|team| self.battle.pet(self.battle.active(team)).health <= 0);
        if !dead.iter().any(|v| *v) {
            return Ok(false);
        }
        for team in 0..2 {
            if dead[team] {
                let Action::Swap(slot) = actions[team] else {
                    return Err("fainted active pet requires replacement".into());
                };
                self.validate_swap(self.battle.active(team), slot)?;
            } else if actions[team] != Action::Pass {
                return Err("opponent must pass during faint replacement".into());
            }
        }
        for team in 0..2 {
            if let Action::Swap(slot) = actions[team] {
                self.swap_pet(team, slot)?;
            }
        }
        Ok(true)
    }
    fn begin_round(&mut self) {
        self.battle.round += 1;
        let damaged = self
            .catalog
            .state_id("Condition_WasDamagedThisTurn")
            .expect("checked state");
        let did_damage = self
            .catalog
            .state_id("Condition_DidDamageThisRound")
            .expect("checked state");
        for team in &mut self.battle.teams {
            for pet in &mut team.pets {
                pet.dealt_damage = false;
                pet.states.insert(damaged, 0);
                pet.states.insert(did_damage, 0);
                for cooldown in pet.cooldowns.values_mut() {
                    *cooldown = cooldown.saturating_sub(1);
                }
                for lockout in &mut pet.lockouts {
                    *lockout = lockout.saturating_sub(1);
                }
            }
        }
    }
    fn validate_action(&self, team: usize, action: Action) -> Result<(), String> {
        let current = self.battle.active(team);
        let pet = self.battle.pet(current);
        if let Some((ability, _)) = pet.continuation {
            if !matches!(action, Action::Pass) && action != Action::Ability(ability) {
                return Err("multi-turn ability locks action".into());
            }
            return rules::validate_ability(self.catalog, ability);
        }
        match action {
            Action::Ability(ability) => {
                if pet.health <= 0 {
                    return Err("fainted active pet requires replacement".into());
                }
                if !pet.abilities.contains(&ability) {
                    return Err(format!("ability {ability} not equipped"));
                }
                if pet.cooldowns.get(&ability).copied().unwrap_or(0) > 0 {
                    return Err(format!("ability {ability} on cooldown"));
                }
                if pet
                    .abilities
                    .iter()
                    .enumerate()
                    .any(|(index, id)| *id == ability && pet.lockouts[index] > 0)
                {
                    return Err(format!("ability {ability} locked out"));
                }
                rules::validate_ability(self.catalog, ability)
            }
            Action::Swap(slot) => self.validate_swap(current, slot),
            Action::Pass if pet.health > 0 => Ok(()),
            Action::Pass => Err("fainted active pet requires replacement".into()),
            Action::Forfeit => Ok(()),
        }
    }
    pub(super) fn validate_swap(&self, current: PetRef, slot: usize) -> Result<(), String> {
        if slot >= 3 || slot == current.slot {
            return Err("invalid swap slot".into());
        }
        let incoming = PetRef {
            team: current.team,
            slot,
        };
        if self.battle.pet(incoming).health <= 0 || self.state(incoming, "swapInLock") != 0 {
            return Err("incoming pet unavailable".into());
        }
        if self.battle.pet(current).health > 0 && self.state(current, "swapOutLock") != 0 {
            return Err("active pet cannot swap".into());
        }
        Ok(())
    }
    pub(super) fn speed(&self, pet: PetRef) -> i32 {
        let data = self.battle.pet(pet);
        let flying =
            if self.state(pet, "Passive_Flying") != 0 && data.health * 2 > data.stats.health {
                1.5
            } else {
                1.0
            };
        (f64::from(data.stats.speed) * self.percent(pet, "Mod_SpeedPercent") * flying)
            .floor()
            .max(0.0) as i32
    }
    fn turn_order(&mut self) -> [usize; 2] {
        let left = self.speed(self.battle.active(0));
        let right = self.speed(self.battle.active(1));
        // Quick Attack563 and SpeedPriority853 promise first strike, not a finite
        // displayed-speed increment. Base speed/tie RNG ranks equal priorities.
        let priority = [0, 1].map(|team| self.state(self.battle.active(team), "Stat_Speed") > 0);
        let first = if priority[0] != priority[1] {
            usize::from(priority[1])
        } else if left == right {
            usize::from(self.draw() >= RANDOM_SCALE / 2)
        } else {
            usize::from(right > left)
        };
        [first, 1 - first]
    }
    fn execute_round(&mut self, actions: [Action; 2]) -> Result<(), String> {
        let pending =
            [0, 1].map(|team| {
                let caster = self.battle.active(team);
                let ability = self.battle.pet(caster).continuation.map(|(id, _)| id).or(
                    match actions[team] {
                        Action::Ability(id) => Some(id),
                        _ => None,
                    },
                );
                ability.map(|ability| (caster, ability))
            });
        self.trigger_actions(pending, 6)?;
        self.trigger_all(6)?;
        let order = self.turn_order();
        self.events.push(Event::Order(order));
        // Voluntary swaps precede attacks. Replacement swaps consume no combat round.
        for team in order {
            if let Action::Swap(slot) = actions[team] {
                self.swap_pet(team, slot)?;
            }
        }
        let actors = [self.battle.active(0), self.battle.active(1)];
        for team in order {
            if self.battle.active(team) == actors[team] {
                self.execute_action(team, actions[team])?;
            }
        }
        self.tick_auras()?;
        self.trigger_all(7)?;
        self.trigger_actions(pending, 7)?;
        self.finish_passives()?;
        self.finish_battle();
        Ok(())
    }
    fn trigger_actions(
        &mut self,
        pending: [Option<(PetRef, u32)>; 2],
        event: u32,
    ) -> Result<(), String> {
        for (caster, ability) in pending.into_iter().flatten() {
            let target = self.battle.active(1 - caster.team);
            self.cast_turn(caster, target, ability, 0, event)?;
        }
        Ok(())
    }
    pub(super) fn swap_pet(&mut self, team: usize, slot: usize) -> Result<(), String> {
        let old = self.battle.active(team);
        self.trigger_for_pet(old, 11)?;
        self.battle.teams[team].active = slot;
        self.events.push(Event::Swapped { team, slot });
        self.trigger_for_pet(self.battle.active(team), 10)
    }
    fn execute_action(&mut self, team: usize, action: Action) -> Result<(), String> {
        if matches!(action, Action::Swap(_)) {
            return Ok(());
        }
        let caster = self.battle.active(team);
        if self.battle.pet(caster).health <= 0 {
            return Ok(());
        }
        self.battle.pet_mut(caster).acted_round = self.battle.round;
        if self.state(caster, "turnLock") != 0 {
            self.battle.pet_mut(caster).continuation = None;
            self.events.push(Event::Skipped(caster));
            return Ok(());
        }
        let ability_turn = self.battle.pet(caster).continuation.or(match action {
            Action::Ability(id) => Some((id, 1)),
            _ => None,
        });
        let Some((ability, turn)) = ability_turn else {
            return Ok(());
        };
        let target = self.battle.active(1 - team);
        self.events.push(Event::Used {
            pet: caster,
            ability,
            turn,
        });
        self.cast_turn(caster, target, ability, turn, 255)?;
        let last = self
            .catalog
            .ability_turns(ability)
            .iter()
            .filter(|t| t.kind == 0)
            .map(|t| t.order)
            .max()
            .unwrap_or(1);
        self.battle.pet_mut(caster).continuation = if turn < last {
            Some((ability, turn + 1))
        } else {
            None
        };
        if turn == 1 {
            self.trigger_used_ability(caster, ability)?;
            self.battle
                .pet_mut(caster)
                .cooldowns
                .insert(ability, self.catalog.abilities[&ability].cooldown);
        }
        Ok(())
    }
    /// DB2 Nevermore/Spell Gulp consume their wearer's next ability use once.
    /// No generic event9 ownership or recursive callback dispatcher is admitted.
    fn trigger_used_ability(&mut self, wearer: PetRef, ability: u32) -> Result<(), String> {
        let auras: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.target == wearer && matches!(a.ability, 738 | 1859))
            .map(|a| a.ability)
            .collect();
        self.used_ability = Some(ability);
        for aura in auras {
            self.cast_turn(wearer, wearer, aura, 0, 9)?;
        }
        self.used_ability = None;
        Ok(())
    }
    pub fn cast_turn(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        order: u32,
        event: u32,
    ) -> Result<(), String> {
        let turns = self.catalog.ability_turns(ability);
        let mut failed = false;
        for turn in turns {
            let matches = if event == 255 {
                turn.kind == 0 && turn.order == order
            } else {
                turn.kind == 1 && turn.event == event
            };
            if !matches {
                continue;
            }
            let events_before_turn = self.events.len();
            let mut effect_target = target;
            for effect in self.catalog.turn_effects(turn.id) {
                if effect.property == 194 {
                    let missed = self.events[events_before_turn..]
                        .iter()
                        .any(|event| matches!(event, Event::Miss { .. }));
                    if !missed {
                        break;
                    }
                }
                if effect.property == 159 && matches!(ability, 762 | 989) {
                    effect_target = caster;
                }
                if effect.property == 158 && failed {
                    break;
                }
                let checked_pet = if effect.property == 157 {
                    caster
                } else {
                    target
                };
                if matches!(effect.property, 157 | 177)
                    && self.battle.pet(checked_pet).state(effect.params[0] as u32)
                        != effect.params[1]
                {
                    break;
                }
                let success =
                    self.execute_effect(caster, effect_target, ability, effect, failed)?;
                // Leech Life383 explicitly restores health on damage dealt. Its
                // event2 contains only healing/state guards, not recursive attacks.
                if ability == 383 && event == 255 && effect.property == 24 && success {
                    self.cast_turn(caster, target, ability, 0, 2)?;
                }
                failed |= !success;
            }
        }
        Ok(())
    }
    fn trigger_all(&mut self, event: u32) -> Result<(), String> {
        let auras = self.battle.auras.clone();
        for aura in auras {
            if self.battle.auras.iter().any(|a| a.serial == aura.serial) {
                self.cast_turn(aura.caster, aura.target, aura.ability, 0, event)?;
            }
        }
        Ok(())
    }
    fn trigger_for_pet(&mut self, pet: PetRef, event: u32) -> Result<(), String> {
        let auras = self.battle.auras.clone();
        for aura in auras {
            if aura.target == pet || aura.team_bound && aura.target.team == pet.team {
                let previous = self.healing_target;
                self.healing_target = (aura.caster.team == pet.team).then_some(pet);
                self.cast_turn(aura.caster, pet, aura.ability, 0, event)?;
                self.healing_target = previous;
            }
        }
        Ok(())
    }
    fn tick_auras(&mut self) -> Result<(), String> {
        let snapshot = self.battle.auras.clone();
        for aura in snapshot {
            if !self.battle.auras.iter().any(|a| a.serial == aura.serial) {
                continue;
            }
            if aura.applied_round == self.battle.round && !aura.tick_first {
                continue;
            }
            let turns = self.catalog.ability_turns(aura.ability);
            let max = turns
                .iter()
                .filter(|t| t.kind == 0)
                .map(|t| t.order)
                .max()
                .unwrap_or(1);
            let order = if max == 1 { 1 } else { aura.turn };
            let caster = aura.caster;
            let target = if aura.team_bound {
                self.battle.active(aura.target.team)
            } else {
                aura.target
            };
            self.healing_target = (caster.team == target.team).then_some(target);
            self.cast_turn(caster, target, aura.ability, order, 255)?;
            self.cast_turn(caster, target, aura.ability, 0, 8)?;
            self.healing_target = None;
            let Some(current) = self
                .battle
                .auras
                .iter_mut()
                .find(|a| a.serial == aura.serial)
            else {
                continue;
            };
            current.turn += 1;
            if current.remaining > 0 {
                current.remaining -= 1;
            }
            if current.remaining == 0 {
                self.remove_aura(aura.serial)?;
            }
        }
        Ok(())
    }
    pub fn remove_aura(&mut self, serial: u32) -> Result<(), String> {
        let Some(index) = self.battle.auras.iter().position(|a| a.serial == serial) else {
            return Ok(());
        };
        let aura = self.battle.auras.remove(index);
        for (pet, state, value) in &aura.state_deltas {
            *self.battle.pet_mut(*pet).states.entry(*state).or_default() -= value;
            self.refresh_max_health(*pet);
        }
        self.events.push(Event::AuraRemoved {
            target: aura.target,
            ability: aura.ability,
        });
        // Aura-local positive effects on removal belong to the wearer, e.g. Resilient924.
        self.cast_turn(aura.target, aura.target, aura.ability, 0, 5)
    }
    pub fn apply_aura_states(&mut self, aura: &mut Aura) {
        let targets: Vec<_> = if aura.weather {
            (0..2)
                .flat_map(|team| (0..3).map(move |slot| PetRef { team, slot }))
                .collect()
        } else if aura.team_bound && aura.charges > 0 {
            (0..3)
                .map(|slot| PetRef {
                    team: aura.target.team,
                    slot,
                })
                .collect()
        } else {
            vec![aura.target]
        };
        let Some(states) = self.catalog.ability_states.get(&aura.ability) else {
            return;
        };
        for pet in targets {
            for (state, value) in states {
                let negative_weather =
                    *value < 0 || matches!(*state, 21 | 22 | 34 | 36 | 52 | 64 | 82);
                if aura.weather && negative_weather && self.state(pet, "Passive_Elemental") != 0 {
                    continue;
                }
                *self.battle.pet_mut(pet).states.entry(*state).or_default() += value;
                aura.state_deltas.push((pet, *state, *value));
            }
            self.refresh_max_health(pet);
        }
    }
    fn refresh_max_health(&mut self, pet: PetRef) {
        let multiplier = self.percent(pet, "Mod_MaxHealthPercent");
        let level_bonus =
            self.state(pet, "maxHealthBonus") * self.state(pet, "Internal_InitialLevel") / 100;
        let max = (f64::from(self.battle.pet(pet).base_health + level_bonus) * multiplier)
            .floor()
            .max(1.0) as i32;
        let data = self.battle.pet_mut(pet);
        data.stats.health = max;
        data.health = data.health.min(max);
    }
    fn finish_passives(&mut self) -> Result<(), String> {
        for team in 0..2 {
            for slot in 0..3 {
                let pet = PetRef { team, slot };
                if self.battle.pet(pet).health <= 0 {
                    continue;
                }
                if self.state(pet, "Passive_Humanoid") != 0 && self.battle.pet(pet).dealt_damage {
                    // Recovery726 property53 supplies4%, no invented passive heal value.
                    let effect = self
                        .catalog
                        .ability_turns(726)
                        .into_iter()
                        .flat_map(|t| self.catalog.turn_effects(t.id))
                        .find(|e| e.property == 53)
                        .ok_or("missing Recovery726 percentage")?;
                    let amount = f64::from(self.battle.pet(pet).stats.health)
                        * f64::from(effect.params[0])
                        / 100.0;
                    self.heal_pet(pet, amount);
                }
                if self.battle.pet(pet).undead_expires == Some(self.battle.round) {
                    self.battle.pet_mut(pet).health = 0;
                    self.battle.pet_mut(pet).continuation = None;
                    self.events.push(Event::Fainted(pet));
                }
            }
        }
        Ok(())
    }
    fn finish_battle(&mut self) {
        let alive = self
            .battle
            .teams
            .each_ref()
            .map(|t| t.pets.iter().any(|p| p.health > 0));
        let result = match alive {
            [true, true] => None,
            [true, false] => Some(Outcome::Winner(0)),
            [false, true] => Some(Outcome::Winner(1)),
            [false, false] => Some(Outcome::Draw),
        };
        if let Some(outcome) = result {
            self.battle.outcome = Some(outcome);
            self.events.push(Event::Finished(outcome));
        }
    }
}
