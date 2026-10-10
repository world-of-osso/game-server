use super::engine::Runtime;
use super::rules::{BASE_VARIANCE, CRITICAL_MULTIPLIER, POWER_COEFFICIENT, type_multiplier};
use super::{Aura, BattleRng, Effect, Event, PetRef};

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
        if matches!(effect.property, 50 | 52 | 54) && p[0] != 0 && failed {
            return Ok(false);
        }
        match effect.property {
            22 | 116 | 157 | 158 | 177 => Ok(true),
            68 => self.percent_damage(caster, target, ability, effect),
            85 => self.reset_aura_state(p[0] as u32),
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
            139 | 145 => self.force_swap(target, effect),
            24 | 222 => {
                let target = if matches!(ability, 404 | 419) && effect.property == 24 {
                    // Sunlight404's three15-point effects target the three enemy slots.
                    PetRef {
                        team: target.team,
                        slot: (effect.order - 1) as usize,
                    }
                } else {
                    target
                };
                self.damage_effect(caster, target, ability, effect)
            }
            29 => {
                if !self.required_states(caster, target, p[2], p[3]) {
                    return Ok(false);
                }
                let mut damage = effect.clone();
                damage.property = 24;
                damage.params[2] = p[4];
                self.damage_effect(caster, target, ability, &damage)
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
                self.heal_pet(caster, amount);
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
            23 | 53 => self.heal_effect(caster, effect),
            31 => {
                self.battle.pet_mut(caster).states.insert(p[0] as u32, p[1]);
                Ok(true)
            }
            50 if !self.required_states(caster, target, p[4], p[5]) => Ok(false),
            26 | 28 | 50 | 52 | 54 | 63 => self.add_aura_effect(caster, target, effect),
            49 => self.remove_aura_effect(caster, effect),
            _ => Err(format!(
                "unsupported property {} effect {}",
                effect.property, effect.id
            )),
        }
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
        let health = if effect.property == 62 {
            self.battle.pet(target).stats.health
        } else if recipient == caster {
            self.battle.pet(caster).health
        } else {
            self.battle.pet(caster).stats.health
        };
        let family = self.catalog.abilities[&ability].family;
        let multiplier = if recipient == caster {
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
        let bypass = effect.params[4] != 0 || matches!(ability, 282 | 2429);
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
        let Some(index) = self.battle.auras.iter().position(|a| {
            a.charges > 0 && (a.target == target || a.team_bound && a.target.team == target.team)
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
        if untargetable || !self.roll(chance) {
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
        let periodic = effect.property == 24 && effect.params[2] != 0;
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
        let mut amount = damage.floor().max(0.0) as i32;
        if self.state(target, "Passive_Magic") != 0 {
            let limit = (f64::from(self.battle.pet(target).stats.health) * 0.35).floor() as i32;
            amount = amount.min(limit);
        }
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
    fn add_aura_effect(
        &mut self,
        caster: PetRef,
        target: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        let positive = matches!(effect.property, 26 | 28 | 63);
        let wearer = if positive { caster } else { target };
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
        let remaining = if effect.params[2] == -1 || effect.aura == 333 && effect.params[2] == 0 {
            -1
        } else {
            effect.params[2].max(max_turn)
        };
        let matching: Vec<_> = self
            .battle
            .auras
            .iter()
            .enumerate()
            .filter(|(_, a)| a.ability == effect.aura && a.target == target)
            .map(|(i, _)| i)
            .collect();
        if matching.len() >= max {
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
        let mut aura = Aura {
            serial: self.battle.next_aura,
            ability: effect.aura,
            caster,
            target,
            remaining,
            turn: 1,
            applied_round: self.battle.round,
            tick_first,
            team_bound: effect.property == 63 && effect.aura != 267
                || matches!(effect.aura, 333 | 1904),
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
