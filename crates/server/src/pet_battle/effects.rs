use super::engine::Runtime;
use super::rules::{BASE_VARIANCE, CRITICAL_MULTIPLIER, POWER_COEFFICIENT, type_multiplier};
use super::{Aura, BattleRng, Effect, Event, PetRef};

/// First row follows the active pet; the next two address its other team members.
fn other_team_slot(active: PetRef, order: u32) -> PetRef {
    let slot = if order == 1 {
        active.slot
    } else {
        (0..3)
            .filter(|slot| *slot != active.slot)
            .nth((order - 2) as usize)
            .expect("admitted three-pet routing")
    };
    PetRef {
        team: active.team,
        slot,
    }
}

impl<R: BattleRng> Runtime<'_, R> {
    pub fn execute_effect(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
        failed: bool,
    ) -> Result<bool, String> {
        let p = effect.params;
        if ability == 1954 && effect.property == 54 {
            return self.apply_poison_protocol_aura(caster, target, effect);
        }
        if matches!(effect.property, 50 | 52 | 54) && p[0] != 0 && failed {
            return Ok(false);
        }
        match effect.property {
            22 | 116 | 157 | 158 | 159 | 177 | 194 => Ok(true),
            117 => {
                let lock = &mut self.battle.pet_mut(caster).lockouts[(p[0] - 1) as usize];
                *lock = (*lock).max(p[1] as u32);
                Ok(true)
            }
            129 => {
                let used = self.used_ability.expect("admitted ability-use callback");
                let pet = self.battle.pet_mut(caster);
                for (index, ability) in pet.abilities.iter().enumerate() {
                    if *ability == used {
                        pet.lockouts[index] = pet.lockouts[index].max(p[0] as u32);
                    }
                }
                Ok(true)
            }
            150 | 168 => self.add_aura_effect(caster, target, effect),
            107 => self.fade_swap(caster, effect),
            67 => Ok(self.equalize_health(caster, target, effect)),
            68 => self.percent_damage(caster, target, ability, effect),
            363 => self.current_health_team_damage(caster, target, ability, effect),
            44 => {
                if !self.hit(caster, caster, effect) {
                    return Ok(false);
                }
                let amount =
                    f64::from(self.state(caster, "Last_HitTaken")) * f64::from(p[0]) / 100.0;
                self.heal_pet(caster, amount);
                Ok(true)
            }
            59 => {
                if self.battle.pet(caster).health >= self.battle.pet(target).health {
                    return Ok(false);
                }
                self.damage_effect(caster, target, ability, effect)
            }
            66 => {
                let mut damage = effect.clone();
                if i64::from(self.battle.pet(target).health) * 4
                    < i64::from(self.battle.pet(target).stats.health)
                {
                    damage.params[0] *= 2;
                }
                damage.params[2] = 0;
                self.damage_effect(caster, target, ability, &damage)
            }
            75 | 76 | 77 => self.charged_damage(caster, target, ability, effect),
            370 => self.damage_with_species_bonus(caster, target, ability, effect),
            96 | 149 => self.damage_effect(caster, target, ability, effect),
            45 | 97 => {
                let caster_speed = self.speed(caster);
                let target_speed = self.speed(target);
                let bonus = if effect.property == 45 {
                    caster_speed < target_speed
                } else {
                    caster_speed > target_speed
                };
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[0] += if bonus { p[0] * p[2] / 100 } else { 0 };
                damage.params[2] = 0;
                self.damage_effect(caster, target, ability, &damage)
            }
            254 => {
                let recipient = self.healing_target.unwrap_or(caster);
                let below_threshold = i64::from(self.battle.pet(recipient).health) * 100
                    < i64::from(self.battle.pet(recipient).stats.health) * i64::from(p[2]);
                let mut heal = effect.clone();
                heal.property = 23;
                heal.params[0] += if below_threshold {
                    p[0] * p[3] / 100
                } else {
                    0
                };
                heal.params[2] = p[4];
                self.heal_effect(caster, &heal)
            }
            100 => self.heal_other_ally(caster, effect),
            223 => self.apply_refuge_shield(caster, effect),
            131 => {
                let first = self
                    .events
                    .iter()
                    .find_map(|e| match e {
                        Event::Order(order) => Some(order[0]),
                        _ => None,
                    })
                    .expect("round order precedes effects");
                if caster.team != first {
                    return Ok(false);
                }
                let mut control = effect.clone();
                control.property = 52;
                self.add_aura_effect(caster, target, &control)
            }
            172 => {
                if p[4] != 0 && failed {
                    return Ok(false);
                }
                let bonus = p[1] != 0 && self.battle.pet(caster).state(p[1] as u32) != p[0]
                    || p[3] != 0 && self.battle.pet(target).state(p[3] as u32) != p[0];
                let mut control = effect.clone();
                control.property = 52;
                control.params[1] = if bonus { 100 } else { p[5] };
                self.add_aura_effect(caster, target, &control)
            }
            65 | 226 => {
                let checked_pet = if effect.property == 65 {
                    caster
                } else {
                    target
                };
                let bonus = if self.battle.pet(checked_pet).state(p[3] as u32) != 0 {
                    p[2]
                } else {
                    0
                };
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[0] += bonus;
                damage.params[2] = p[4];
                damage.params[3] = 0;
                self.damage_effect(caster, target, ability, &damage)
            }
            85 => self.reset_aura_state(p[0] as u32),
            112 => self.resurrect_reserve(caster, effect),
            111 => {
                let pet = self.battle.pet_mut(caster);
                pet.health = pet.stats.health * p[0] / 100;
                pet.states.insert(self.catalog.state_id("Is_Dead")?, 0);
                self.events.push(Event::Revived(caster));
                Ok(true)
            }
            169 => {
                let wearer = if ability == 171 {
                    self.battle.active(if effect.order == 1 {
                        caster.team
                    } else {
                        1 - caster.team
                    })
                } else {
                    target
                };
                if p[2] != 0 && self.battle.pet(caster).state(p[2] as u32) == 0
                    || p[3] != 0 && self.battle.pet(wearer).state(p[3] as u32) != 0
                {
                    return Ok(false);
                }
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[2] = p[4];
                self.damage_effect(caster, wearer, ability, &damage)
            }
            170 => {
                if self.battle.pet(caster).state(p[2] as u32) == 0 {
                    return Ok(false);
                }
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[2] = p[4];
                self.damage_effect(caster, target, ability, &damage)
            }
            171 => {
                let recipient = self.healing_target.unwrap_or(caster);
                if !self.required_states(caster, recipient, p[2], p[3]) {
                    return Ok(false);
                }
                let mut heal = effect.clone();
                heal.property = 23;
                heal.params[2] = p[4];
                self.heal_effect(caster, &heal)
            }
            246 => {
                let recipient = self.healing_target.unwrap_or(caster);
                let id = self.battle.pet(recipient).abilities[(p[0] - 1) as usize];
                self.battle.pet_mut(recipient).cooldowns.remove(&id);
                Ok(true)
            }
            373 => {
                self.battle.pet_mut(caster).abilities[p[0] as usize] = p[1] as u32;
                Ok(true)
            }
            104 => {
                if !self.required_states(caster, target, p[2], p[3]) {
                    return Ok(false);
                }
                let mut heal = effect.clone();
                heal.property = 23;
                heal.params[2] = p[4];
                self.heal_effect(caster, &heal)
            }
            135 => self.kill_effect(caster, ability, effect),
            136 => self.cleanse(caster),
            145 if ability == 1989 => self.force_swap(caster, effect),
            139 | 145 => self.force_swap(target, effect),
            24 | 222 => {
                let target = if super::rules::damages_enemy_team(ability) {
                    let order = match ability {
                        774 => (effect.order - 1) % 3 + 1,
                        920 | 1013 | 2378 | 2457 => effect.order.div_ceil(2),
                        _ => effect.order,
                    };
                    other_team_slot(target, order)
                } else {
                    target
                };
                if ability == 2457 {
                    //A skipped/dead/blocked pair must not drain the preceding pair's hit.
                    let last = self.catalog.state_id("Last_HitDealt")?;
                    self.battle.pet_mut(caster).states.insert(last, 0);
                }
                self.damage_effect(caster, target, ability, effect)
            }
            29 => {
                let target = if ability == 1013 {
                    other_team_slot(target, effect.order.div_ceil(2))
                } else {
                    target
                };
                if !self.required_states(caster, target, p[2], p[3]) {
                    return Ok(false);
                }
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[2] = p[4];
                self.damage_effect(caster, target, ability, &damage)
            }
            62 if matches!(ability, 649 | 1761) => {
                self.percent_damage(caster, caster, ability, effect)
            }
            62 => self.percent_damage(caster, target, ability, effect),
            141 => self.split_damage(caster, target, ability, effect),
            384 | 385 => {
                let first = self
                    .events
                    .iter()
                    .find_map(|e| match e {
                        Event::Order(order) => Some(order[0]),
                        _ => None,
                    })
                    .expect("round order precedes effects");
                if (caster.team == first) != (effect.property == 384) {
                    return Ok(false);
                }
                self.damage_effect(caster, target, ability, effect)
            }
            80 => self.weather_effect(caster, effect, failed),
            178 => {
                if p[4] != 0 && failed
                    || p[3] != 0 && self.battle.pet(target).state(p[3] as u32) == p[0]
                {
                    if p[5] != 0 {
                        self.events.push(Event::Immune {
                            effect: effect.id,
                            target,
                        });
                    }
                    return Ok(false);
                }
                self.add_aura_effect(caster, target, effect)
            }
            27 => self.ramping_damage(caster, target, ability, effect),
            32 => {
                if p[2] != 0 && failed || !self.hit(caster, caster, effect) {
                    return Ok(false);
                }
                let amount =
                    f64::from(self.state(caster, "Last_HitDealt")) * f64::from(p[0]) / 100.0;
                if ability == 2457 {
                    let recipient = other_team_slot(caster, effect.order / 2);
                    if self.battle.pet(recipient).health <= 0 {
                        return Ok(false);
                    }
                    self.heal_between(caster, recipient, amount);
                } else {
                    self.heal_pet(caster, amount);
                }
                Ok(true)
            }
            79 => {
                let old = self.battle.pet(caster).state(p[0] as u32);
                let value = old + p[1];
                let value = if p[2] < p[3] {
                    value.clamp(p[2], p[3])
                } else {
                    value
                };
                self.battle
                    .pet_mut(caster)
                    .states
                    .insert(p[0] as u32, value);
                Ok(true)
            }
            23 if matches!(ability, 258 | 979 | 2378) => {
                let order = if ability == 2378 {
                    effect.order / 2
                } else {
                    effect.order - 3
                };
                let recipient = other_team_slot(caster, order);
                self.heal_recipient(caster, recipient, effect)
            }
            23 | 53 => self.heal_effect(caster, effect),
            31 => {
                let control = matches!(p[0], 22 | 35 | 36 | 64);
                if p[1] > 0 && control && self.state(caster, "Passive_Critter") != 0 {
                    self.events.push(Event::Immune {
                        effect: effect.id,
                        target: caster,
                    });
                    return Ok(false);
                }
                self.battle.pet_mut(caster).states.insert(p[0] as u32, p[1]);
                Ok(true)
            }
            50 if !self.required_states(caster, target, p[4], p[5]) => Ok(false),
            50 if ability == 2365 => self.add_aura_effect(caster, caster, effect),
            26 | 28 | 50 | 52 | 54 | 63 => {
                let target = if matches!(ability, 527 | 920 | 1010) && effect.property == 50 {
                    other_team_slot(target, effect.order.div_ceil(2))
                } else {
                    target
                };
                self.add_aura_effect(caster, target, effect)
            }
            49 => self.remove_aura_effect(caster, effect),
            _ => Err(format!(
                "unsupported property {} effect {}",
                effect.property, effect.id
            )),
        }
    }
    fn damage_with_species_bonus(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let species = self.battle.pet(caster).species;
        let matching_reserves = self.battle.teams[caster.team]
            .pets
            .iter()
            .enumerate()
            .filter(|(slot, pet)| *slot != caster.slot && pet.species == species)
            .count() as i32;
        let mut damage = effect.clone();
        damage.property = 24;
        damage.params[0] += matching_reserves * effect.params[2];
        damage.params[2] = 0;
        self.damage_effect(caster, target, ability, &damage)
    }
    fn charged_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let (wearer, duration) = if effect.property == 77 {
            (target, effect.params[2])
        } else {
            (caster, -1)
        };
        let charge = self
            .battle
            .auras
            .iter()
            .find(|a| a.target == wearer && a.ability == effect.aura)
            .map(|a| a.serial);
        if let Some(serial) = charge {
            let hit = self.damage_effect(caster, target, ability, effect)?;
            self.remove_aura(serial)?;
            return Ok(hit);
        }
        let mut charge = effect.clone();
        charge.property = 26;
        charge.params[2] = duration;
        self.add_aura_to_wearer(caster, wearer, &charge)
    }
    fn equalize_health(&mut self, caster: PetRef, target: PetRef, effect: &Effect) -> bool {
        let mut accuracy = effect.clone();
        accuracy.params[1] = effect.params[0];
        if !self.hit(caster, target, &accuracy) {
            return false;
        }
        let combined_health =
            i64::from(self.battle.pet(caster).health) + i64::from(self.battle.pet(target).health);
        let shared_health = (combined_health / 2) as i32;
        for recipient in [caster, target] {
            let pet = self.battle.pet_mut(recipient);
            pet.health = shared_health.min(pet.stats.health);
        }
        true
    }
    fn apply_poison_protocol_aura(
        &mut self,
        caster: PetRef,
        target: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        let recipient = other_team_slot(target, effect.order - 3);
        let action_start = self
            .events
            .iter()
            .rposition(
                |event| matches!(event, Event::Used { pet, ability: 1954, .. } if *pet == caster),
            )
            .expect("Poison Protocol has its own ability-use event");
        //PET_BATTLE_POISON_PROTOCOL_PER_VICTIM_CHAIN_POLICY: only this action's
        //positive direct hit qualifies the corresponding victim's poison row.
        let qualified = self.events[action_start + 1..].iter().any(|event| {
            matches!(event, Event::Damage { caster: attacker, target: victim, amount, periodic: false, .. }
                if *attacker == caster && *victim == recipient && *amount > 0)
        });
        if !qualified {
            return Ok(false);
        }
        self.add_aura_effect(caster, recipient, effect)
    }
    fn heal_other_ally(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        let p = effect.params;
        let recipient = other_team_slot(caster, effect.order);
        if p[2] != 0 && self.battle.pet(caster).family + 1 != p[2] as usize
            || self.battle.pet(recipient).family + 1 != p[3] as usize
        {
            return Ok(false);
        }
        let mut heal = effect.clone();
        heal.property = 23;
        heal.params[2] = p[4];
        self.heal_recipient(caster, recipient, &heal)
    }
    fn heal_recipient(
        &mut self,
        caster: PetRef,
        recipient: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        let previous = self.healing_target.replace(recipient);
        let result = self.heal_effect(caster, effect);
        self.healing_target = previous;
        result
    }
    fn reset_aura_state(&mut self, state: u32) -> Result<bool, String> {
        let serials: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| {
                self.catalog
                    .ability_states
                    .get(&a.ability)
                    .is_some_and(|s| s.get(&state).copied().unwrap_or(0) != 0)
            })
            .map(|a| a.serial)
            .collect();
        for serial in serials {
            self.remove_aura(serial)?;
        }
        Ok(true)
    }
    fn current_health_team_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let order = self
            .catalog
            .turn_effects(effect.turn)
            .into_iter()
            .filter(|row| row.property == 363 && row.order <= effect.order)
            .count() as u32;
        let team_anchor = if ability == 2388 { caster } else { target };
        let recipient = other_team_slot(team_anchor, order);
        //Each following leech row belongs to this victim, including misses/dead reserves.
        let last = self.catalog.state_id("Last_HitDealt")?;
        self.battle.pet_mut(caster).states.insert(last, 0);
        self.percent_damage(caster, recipient, ability, effect)
    }
    fn percent_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let recipient = if effect.property == 68 && matches!(ability, 451 | 758 | 813 | 1770) {
            caster
        } else {
            target
        };
        if self.battle.pet(recipient).health <= 0 || !self.hit(caster, recipient, effect) {
            return Ok(false);
        }
        if caster != recipient && self.block_attack(recipient)? {
            return Ok(false);
        }
        let health = if effect.property == 363 {
            self.battle.pet(recipient).health
        } else if effect.property == 62 {
            self.battle.pet(target).stats.health
        } else if recipient == caster {
            self.battle.pet(caster).health
        } else {
            self.battle.pet(caster).stats.health
        };
        let family = self.catalog.abilities[&ability].family;
        //PET_BATTLE_PCT_CURRENT_HP_USES_ENGINE_PCT_DAMAGE_CONVENTION:
        //hostile percentages retain type scaling; friendly costs are untyped.
        let friendly_cost = effect.property == 363 && recipient.team == caster.team;
        let multiplier = if recipient == caster || friendly_cost {
            1.0
        } else {
            type_multiplier(family as usize, self.battle.pet(recipient).family)?
        };
        let amount =
            (f64::from(health) * f64::from(effect.params[0]) / 100.0 * multiplier).floor() as i32;
        self.damage_pet(caster, recipient, amount, false, effect.params[2] != 0)?;
        Ok(amount > 0)
    }
    fn kill_effect(
        &mut self,
        caster: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        if effect.params[1] != 0 && self.battle.pet(caster).state(effect.params[1] as u32) != 0
            || !self.roll(effect.params[0])
        {
            return Ok(false);
        }
        let bypass = effect.params[4] != 0 || matches!(ability, 282 | 1025 | 2429);
        self.battle.pet_mut(caster).health = 0;
        if bypass {
            self.battle.pet_mut(caster).resurrection_used = true;
        }
        self.revive_or_faint(caster)?;
        Ok(true)
    }
    fn cleanse(&mut self, pet: PetRef) -> Result<bool, String> {
        let serials: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.target == pet && !a.weather && !a.team_bound)
            .map(|a| a.serial)
            .collect();
        for serial in serials {
            self.remove_aura(serial)?;
        }
        Ok(true)
    }
    fn resurrect_reserve(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        let recipient = other_team_slot(caster, effect.order + 1);
        if self.battle.pet(recipient).health > 0
            || !self.required_states(caster, recipient, effect.params[2], effect.params[3])
        {
            return Ok(false);
        }
        let dead = self.catalog.state_id("Is_Dead")?;
        let pet = self.battle.pet_mut(recipient);
        pet.health = pet.stats.health * effect.params[0] / 100;
        pet.states.insert(dead, 0);
        pet.undead_expires = None;
        self.events.push(Event::Revived(recipient));
        Ok(true)
    }
    fn fade_swap(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        let candidates: Vec<_> = (0..3)
            .filter(|slot| {
                *slot != caster.slot && self.battle.teams[caster.team].pets[*slot].health > 0
            })
            .collect();
        if candidates.is_empty() {
            return Ok(false);
        }
        let choice = if candidates.len() == 1 {
            0
        } else {
            self.draw() as usize * candidates.len() / super::rules::RANDOM_SCALE as usize
        };
        let slot = candidates[choice];
        if self.validate_swap(caster, slot).is_err() {
            self.events.push(Event::Immune {
                effect: effect.id,
                target: caster,
            });
            return Ok(false);
        }
        self.swap_pet(caster.team, slot)?;
        Ok(true)
    }
    fn force_swap(&mut self, target: PetRef, effect: &Effect) -> Result<bool, String> {
        if !self.roll(effect.params[0]) {
            return Ok(false);
        }
        let candidates: Vec<_> = (0..3)
            .filter(|slot| {
                *slot != target.slot && self.battle.teams[target.team].pets[*slot].health > 0
            })
            .collect();
        let Some(extreme) = candidates
            .iter()
            .map(|slot| self.battle.teams[target.team].pets[*slot].health)
            .reduce(|a, b| {
                if effect.property == 139 {
                    a.min(b)
                } else {
                    a.max(b)
                }
            })
        else {
            return Ok(false);
        };
        let tied: Vec<_> = candidates
            .into_iter()
            .filter(|slot| self.battle.teams[target.team].pets[*slot].health == extreme)
            .collect();
        let choice = if tied.len() == 1 {
            0
        } else {
            self.draw() as usize * tied.len() / super::rules::RANDOM_SCALE as usize
        };
        let slot = tied[choice];
        if self.validate_swap(target, slot).is_err() {
            self.events.push(Event::Immune {
                effect: effect.id,
                target,
            });
            return Ok(false);
        }
        self.swap_pet(target.team, slot)?;
        Ok(true)
    }
    fn block_attack(&mut self, target: PetRef) -> Result<bool, String> {
        let Some(index) = self.battle.auras.iter().position(|aura| {
            let blocks_both_sides = super::rules::is_two_way_wall(aura.ability);
            let protects_target =
                aura.target == target || aura.team_bound && aura.target.team == target.team;
            aura.charges > 0 && (blocks_both_sides || protects_target)
        }) else {
            return Ok(false);
        };
        let state = self.catalog.state_id("Special_BlockedAttackCount")?;
        let aura = &mut self.battle.auras[index];
        aura.charges -= 1;
        let serial = aura.serial;
        let expired = aura.charges == 0;
        let mut wearers = Vec::new();
        for (pet, id, value) in &mut aura.state_deltas {
            if *id == state {
                *value -= 1;
                wearers.push(*pet);
            }
        }
        for pet in wearers {
            *self.battle.pet_mut(pet).states.entry(state).or_default() -= 1;
        }
        if expired {
            self.remove_aura(serial)?;
        }
        Ok(true)
    }
    fn required_states(
        &self,
        caster: PetRef,
        target: PetRef,
        caster_state: i32,
        target_state: i32,
    ) -> bool {
        (caster_state == 0 || self.battle.pet(caster).state(caster_state as u32) != 0)
            && (target_state == 0 || self.battle.pet(target).state(target_state as u32) != 0)
    }
    fn split_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let targets: Vec<_> = (0..3)
            .map(|slot| PetRef {
                team: target.team,
                slot,
            })
            .filter(|pet| self.battle.pet(*pet).health > 0)
            .collect();
        if targets.is_empty() {
            return Ok(false);
        }
        let bonus = if effect.params[2] != 0
            && self.battle.pet(caster).state(effect.params[2] as u32) != 0
        {
            effect.params[3]
        } else {
            0
        };
        let mut damage = effect.clone();
        damage.params[0] += bonus;
        damage.params[2] = 0;
        let mut success = false;
        for target in &targets {
            success |=
                self.scaled_damage(caster, *target, ability, &damage, targets.len() as f64)?;
        }
        Ok(success)
    }
    fn ramping_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        let id = self.catalog.state_id("Ramping_DamageID")?;
        let uses = self.catalog.state_id("Ramping_DamageUses")?;
        let count = if self.battle.pet(caster).state(id) == ability as i32 {
            self.battle.pet(caster).state(uses)
        } else {
            0
        };
        let mut scaled = effect.clone();
        scaled.params[0] = (effect.params[0] + count * effect.params[2]).min(effect.params[3]);
        scaled.params[2] = 0;
        let hit = self.damage_effect(caster, target, ability, &scaled)?;
        self.battle
            .pet_mut(caster)
            .states
            .insert(id, ability as i32);
        // Vicious Fang156 / Huge Fang930 say each time it hits; other rampers say used.
        let increment = !matches!(ability, 156 | 930) || hit;
        self.battle
            .pet_mut(caster)
            .states
            .insert(uses, count + i32::from(increment));
        if hit && scaled.params[0] == effect.params[3] && effect.params[4] != 0 {
            self.battle
                .pet_mut(target)
                .states
                .insert(effect.params[4] as u32, 1);
        }
        Ok(hit)
    }
    fn hit(&mut self, caster: PetRef, target: PetRef, effect: &Effect) -> bool {
        let chance = effect.params[1] + self.state(caster, "Stat_Accuracy")
            - self.state(target, "Stat_Dodge");
        let untargetable = caster != target && self.state(target, "untargettable") != 0;
        let guaranteed = effect.property == 96
            && self.required_states(caster, target, effect.params[2], effect.params[3]);
        if untargetable || !guaranteed && !self.roll(chance) {
            self.events.push(Event::Miss {
                effect: effect.id,
                target,
            });
            return false;
        }
        true
    }
    fn damage_effect(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
    ) -> Result<bool, String> {
        self.scaled_damage(caster, target, ability, effect, 1.0)
    }
    fn scaled_damage(
        &mut self,
        caster: PetRef,
        target: PetRef,
        ability: u32,
        effect: &Effect,
        divisor: f64,
    ) -> Result<bool, String> {
        if self.battle.pet(target).health <= 0 {
            return Ok(false);
        }
        if !self.hit(caster, target, effect) {
            let last = self.catalog.state_id("Last_HitDealt")?;
            self.battle.pet_mut(caster).states.insert(last, 0);
            return Ok(false);
        }
        let periodic = effect.property == 24 && effect.params[2] != 0
            || effect.property == 96 && effect.params[4] != 0;
        if !periodic && caster != target && self.block_attack(target)? {
            return Ok(false);
        }
        let points = effect.params[0]
            + self.state(caster, "Add_FlatDamageDealt")
            + self.state(target, "Add_FlatDamageTaken");
        let base = f64::from(points)
            * (1.0 + POWER_COEFFICIENT * f64::from(self.battle.pet(caster).stats.power));
        let width = if effect.property == 222 {
            effect.params[2]
        } else {
            BASE_VARIANCE
        };
        let varied = self.variance(base, width);
        let critical = self.roll(self.state(caster, "Stat_CritChance"));
        let family = self.catalog.abilities[&ability].family;
        if family < 0 {
            return Err(format!("ability {ability} has no damage family"));
        }
        let multiplier = type_multiplier(family as usize, self.battle.pet(target).family)?;
        let damage = varied
            * multiplier
            * self.damage_modifiers(caster, target, periodic)
            * self.family_modifiers(caster, target, family)
            / divisor
            * if critical { CRITICAL_MULTIPLIER } else { 1.0 };
        let amount = damage.floor().max(0.0) as i32;
        let amount = if effect.property == 149 {
            amount.min(self.battle.pet(target).health - 1)
        } else {
            amount
        };
        self.damage_pet(caster, target, amount, critical, periodic)?;
        Ok(amount > 0)
    }
    fn family_modifiers(&self, caster: PetRef, target: PetRef, family: i32) -> f64 {
        let dealt = if self.state(caster, "Mod_PetType_ID") == family + 1 {
            self.percent(caster, "Mod_PetTypeDamageDealtPercent")
        } else {
            1.0
        };
        let taken = if self.state(target, "Mod_PetType_ID") == family + 1 {
            self.percent(target, "Mod_PetTypeDamageTakenPercent")
        } else {
            1.0
        };
        dealt * taken
    }
    fn damage_modifiers(&self, caster: PetRef, target: PetRef, periodic: bool) -> f64 {
        let pet = self.battle.pet(caster);
        let beast = if self.state(caster, "Passive_Beast") != 0 && pet.health * 2 < pet.stats.health
        {
            1.25
        } else {
            1.0
        };
        let dragon = if pet.dragon_bonus_round == Some(self.battle.round) {
            1.5
        } else {
            1.0
        };
        let undead = if pet.undead_expires.is_some() {
            0.75
        } else {
            1.0
        };
        let aquatic_dealt = if periodic && self.state(caster, "Passive_Aquatic") != 0 {
            1.25
        } else {
            1.0
        };
        let aquatic_taken = if periodic && self.state(target, "Passive_Aquatic") != 0 {
            0.5
        } else {
            1.0
        };
        self.percent(caster, "Mod_DamageDealtPercent")
            * self.percent(target, "Mod_DamageTakenPercent")
            * beast
            * dragon
            * undead
            * aquatic_dealt
            * aquatic_taken
    }
    fn damage_pet(
        &mut self,
        caster: PetRef,
        target: PetRef,
        amount: i32,
        critical: bool,
        periodic: bool,
    ) -> Result<(), String> {
        let amount = if self.state(target, "Passive_Magic") != 0 {
            amount.min((f64::from(self.battle.pet(target).stats.health) * 0.35).floor() as i32)
        } else {
            amount
        };
        let old = self.battle.pet(target).health;
        let immortal = self.state(target, "unkillable") != 0
            || self.battle.pet(target).undead_expires.is_some();
        let floor = if immortal { 1 } else { 0 };
        let health = (old - amount).max(floor);
        self.battle.pet_mut(target).health = health;
        self.events.push(Event::Damage {
            caster,
            target,
            amount: old - health,
            critical,
            periodic,
        });
        let dealt = old - health;
        let last_dealt = self.catalog.state_id("Last_HitDealt")?;
        let last_taken = self.catalog.state_id("Last_HitTaken")?;
        self.battle.pet_mut(caster).states.insert(last_dealt, dealt);
        self.battle.pet_mut(target).states.insert(last_taken, dealt);
        if old > health {
            let damaged = self.catalog.state_id("Condition_WasDamagedThisTurn")?;
            let did_damage = self.catalog.state_id("Condition_DidDamageThisRound")?;
            self.battle.pet_mut(target).states.insert(damaged, 1);
            self.battle.pet_mut(caster).states.insert(did_damage, 1);
            self.battle.pet_mut(caster).dealt_damage = true;
            self.trigger_dragon(caster, target, old, health);
        }
        if health == 0 {
            self.revive_or_faint(target)?;
        }
        Ok(())
    }
    fn trigger_dragon(&mut self, caster: PetRef, target: PetRef, old: i32, health: i32) {
        let half = self.battle.pet(target).stats.health;
        if self.state(caster, "Passive_Dragon") != 0 && old * 2 >= half && health * 2 < half {
            self.battle.pet_mut(caster).dragon_bonus_round = Some(self.battle.round + 1);
        }
    }
    fn revive_or_faint(&mut self, target: PetRef) -> Result<(), String> {
        let used = self.battle.pet(target).resurrection_used;
        let undead = self.state(target, "Passive_Undead") != 0;
        let mechanical = self.state(target, "Passive_Mechanical") != 0;
        if !used && (undead || mechanical) {
            let health = if undead {
                1
            } else {
                (f64::from(self.battle.pet(target).stats.health) * 0.20)
                    .floor()
                    .max(1.0) as i32
            };
            let expires = if undead {
                Some(self.battle.round + 1)
            } else {
                None
            };
            let pet = self.battle.pet_mut(target);
            pet.health = health;
            pet.undead_expires = expires;
            pet.resurrection_used = true;
            self.events.push(Event::Revived(target));
            return Ok(());
        }
        let dead = self.catalog.state_id("Is_Dead")?;
        self.battle.pet_mut(target).states.insert(dead, 1);
        self.battle.pet_mut(target).continuation = None;
        self.events.push(Event::Fainted(target));
        let serials: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.target == target && !a.weather && !a.team_bound)
            .map(|a| a.serial)
            .collect();
        for serial in serials {
            self.remove_aura(serial)?;
        }
        Ok(())
    }
    fn heal_effect(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        let recipient = self.healing_target.unwrap_or(caster);
        if self.battle.pet(recipient).health <= 0 || !self.hit(caster, recipient, effect) {
            return Ok(false);
        }
        let p = effect.params;
        let amount = if effect.property == 53 {
            f64::from(self.battle.pet(recipient).stats.health) * f64::from(p[0]) / 100.0
        } else {
            let base = f64::from(p[0])
                * (1.0 + POWER_COEFFICIENT * f64::from(self.battle.pet(caster).stats.power));
            self.variance(base, p[2])
        };
        self.heal_between(caster, recipient, amount);
        Ok(true)
    }
    pub fn heal_pet(&mut self, target: PetRef, amount: f64) {
        self.heal_between(target, target, amount);
    }
    fn heal_between(&mut self, caster: PetRef, target: PetRef, amount: f64) {
        let amount = (amount
            * self.percent(caster, "Mod_HealingDealtPercent")
            * self.percent(target, "Mod_HealingTakenPercent"))
        .floor()
        .max(0.0) as i32;
        let old = self.battle.pet(target).health;
        let health = old
            .saturating_add(amount)
            .min(self.battle.pet(target).stats.health);
        self.battle.pet_mut(target).health = health;
        self.events.push(Event::Heal {
            target,
            amount: health - old,
        });
    }
    fn weather_effect(
        &mut self,
        caster: PetRef,
        effect: &Effect,
        failed: bool,
    ) -> Result<bool, String> {
        if effect.params[3] != 0 && failed || !self.hit(caster, caster, effect) {
            return Ok(false);
        }
        let old: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.weather)
            .map(|a| a.serial)
            .collect();
        for serial in old {
            self.remove_aura(serial)?;
        }
        self.apply_or_refresh_aura(caster, caster, effect, 1, true)?;
        Ok(true)
    }
    fn apply_refuge_shield(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        let wearer = other_team_slot(caster, effect.order);
        if self.battle.pet(wearer).family + 1 != effect.params[3] as usize {
            return Ok(false);
        }
        let mut shield = effect.clone();
        shield.property = 26;
        // 223's family field is not 26's first-round countdown flag.
        shield.params[3] = 0;
        self.add_aura_to_wearer(caster, wearer, &shield)
    }
    fn add_aura_effect(
        &mut self,
        caster: PetRef,
        target: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        let positive = matches!(effect.property, 26 | 28 | 63 | 150);
        let wearer = if positive { caster } else { target };
        self.add_aura_to_wearer(caster, wearer, effect)
    }
    fn add_aura_to_wearer(
        &mut self,
        caster: PetRef,
        wearer: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        // Bend Time1905 uses Accuracy0 for its unconditional next-swap team trigger.
        let unconditional = effect.aura == 1904 && effect.params[1] == 0;
        if self.battle.pet(wearer).health <= 0
            || !unconditional && !self.hit(caster, wearer, effect)
        {
            return Ok(false);
        }
        if matches!(effect.property, 52 | 178) && self.control_immune(wearer, effect.aura) {
            self.events.push(Event::Immune {
                effect: effect.id,
                target: wearer,
            });
            return Ok(false);
        }
        let tick_first = if effect.property == 26 {
            effect.params[3] != 0
        } else if matches!(effect.property, 52 | 178) {
            self.battle.pet(wearer).acted_round != self.battle.round
        } else {
            true
        };
        let max = if matches!(effect.property, 50 | 54) {
            effect.params[3].max(1) as usize
        } else {
            1
        };
        self.apply_or_refresh_aura(caster, wearer, effect, max, tick_first)?;
        Ok(true)
    }
    fn control_immune(&self, wearer: PetRef, ability: u32) -> bool {
        let has_control = self
            .catalog
            .ability_states
            .get(&ability)
            .is_some_and(|states| {
                [
                    "Mechanic_IsStunned",
                    "Mechanic_IsWebbed",
                    "turnLock",
                    "swapOutLock",
                ]
                .iter()
                .any(|name| {
                    let id = self.catalog.state_id(name).expect("checked rule state");
                    states.get(&id).copied().unwrap_or(0) > 0
                })
            });
        let locks_turn = self
            .catalog
            .ability_states
            .get(&ability)
            .is_some_and(|states| {
                let lock = self
                    .catalog
                    .state_id("turnLock")
                    .expect("checked rule state");
                states.get(&lock).copied().unwrap_or(0) > 0
            });
        (has_control && self.state(wearer, "Weather_ArcaneStorm") != 0)
            || (has_control && self.state(wearer, "Passive_Critter") != 0)
            || (locks_turn && self.state(wearer, "resilient") != 0)
    }
    fn apply_or_refresh_aura(
        &mut self,
        caster: PetRef,
        target: PetRef,
        effect: &Effect,
        max: usize,
        tick_first: bool,
    ) -> Result<(), String> {
        let max_turn = self
            .catalog
            .ability_turns(effect.aura)
            .iter()
            .filter(|t| t.kind == 0)
            .map(|t| t.order)
            .max()
            .unwrap_or(1) as i32;
        let persistent_object =
            effect.params[2] == 0 && (effect.aura == 333 || effect.property == 150);
        let remaining = if effect.params[2] == -1 || persistent_object {
            -1
        } else {
            effect.params[2].max(max_turn)
        };
        let matching: Vec<_> = self
            .battle
            .auras
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.ability == effect.aura
                    && (a.target == target || a.team_bound && a.target.team == target.team)
            })
            .map(|(i, _)| i)
            .collect();
        if matching.len() >= max {
            self.restore_aura_charges(matching[0]);
            let aura = &mut self.battle.auras[matching[0]];
            aura.remaining = remaining;
            aura.turn = 1;
            aura.applied_round = self.battle.round;
            aura.tick_first = tick_first;
            aura.caster = caster;
            self.events.push(Event::AuraApplied {
                target,
                ability: effect.aura,
            });
            return Ok(());
        }
        let follows_active_ally = effect.property == 63 && effect.aura != 267;
        let shared_across_team = effect.property == 150 || matches!(effect.aura, 189 | 333 | 1904);
        let mut aura = Aura {
            serial: self.battle.next_aura,
            ability: effect.aura,
            caster,
            target,
            remaining,
            turn: 1,
            applied_round: self.battle.round,
            tick_first,
            team_bound: follows_active_ally || shared_across_team,
            weather: effect.property == 80,
            charges: self
                .catalog
                .ability_states
                .get(&effect.aura)
                .and_then(|s| s.get(&74))
                .copied()
                .unwrap_or(0),
            state_deltas: Vec::new(),
        };
        self.battle.next_aura += 1;
        self.apply_aura_states(&mut aura);
        self.battle.auras.push(aura);
        self.events.push(Event::AuraApplied {
            target,
            ability: effect.aura,
        });
        self.cast_turn(caster, target, effect.aura, 0, 0)
    }
    fn restore_aura_charges(&mut self, index: usize) {
        let aura = &mut self.battle.auras[index];
        let charges = self
            .catalog
            .ability_states
            .get(&aura.ability)
            .and_then(|s| s.get(&74))
            .copied()
            .unwrap_or(0);
        let delta = charges - aura.charges;
        aura.charges = charges;
        let mut wearers = Vec::new();
        for (pet, state, value) in &mut aura.state_deltas {
            if *state == 74 {
                *value += delta;
                wearers.push(*pet);
            }
        }
        for pet in wearers {
            *self.battle.pet_mut(pet).states.entry(74).or_default() += delta;
        }
    }
    fn remove_aura_effect(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        if effect.params[0] != 0 && !self.roll(effect.params[0]) {
            return Ok(false);
        }
        let serials: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.target == caster && a.ability == effect.aura)
            .map(|a| a.serial)
            .collect();
        for serial in serials {
            self.remove_aura(serial)?;
        }
        Ok(true)
    }
}
