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
            22 => Ok(true), // Explicit dummy opcode, only admitted for zero Points.
            24 | 222 => self.damage_effect(caster, target, ability, effect),
            23 | 53 => self.heal_effect(caster, effect),
            31 => {
                self.battle.pet_mut(caster).states.insert(p[0] as u32, p[1]);
                Ok(true)
            }
            26 | 50 | 52 | 54 | 63 => self.add_aura_effect(caster, target, effect),
            49 => self.remove_aura_effect(caster, effect),
            _ => Err(format!(
                "unsupported property {} effect {}",
                effect.property, effect.id
            )),
        }
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
        if self.battle.pet(target).health <= 0 {
            return Ok(false);
        }
        if !self.hit(caster, target, effect) {
            return Ok(false);
        }
        let periodic = effect.property == 24 && effect.params[2] != 0;
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
            * if critical { CRITICAL_MULTIPLIER } else { 1.0 };
        let mut amount = damage.floor().max(0.0) as i32;
        if self.state(target, "Passive_Magic") != 0 {
            let limit = (f64::from(self.battle.pet(target).stats.health) * 0.35).floor() as i32;
            amount = amount.min(limit);
        }
        self.damage_pet(caster, target, amount, critical, periodic)?;
        Ok(amount > 0)
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
        if old > health {
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
        self.battle.pet_mut(target).continuation = None;
        self.events.push(Event::Fainted(target));
        let serials: Vec<_> = self
            .battle
            .auras
            .iter()
            .filter(|a| a.target == target)
            .map(|a| a.serial)
            .collect();
        for serial in serials {
            self.remove_aura(serial)?;
        }
        Ok(())
    }
    fn heal_effect(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        if self.battle.pet(caster).health <= 0 || !self.hit(caster, caster, effect) {
            return Ok(false);
        }
        let p = effect.params;
        let amount = if effect.property == 53 {
            f64::from(self.battle.pet(caster).stats.health) * f64::from(p[0]) / 100.0
        } else {
            let base = f64::from(p[0])
                * (1.0 + POWER_COEFFICIENT * f64::from(self.battle.pet(caster).stats.power));
            self.variance(base, p[2])
        };
        self.heal_pet(caster, amount);
        Ok(true)
    }
    pub fn heal_pet(&mut self, target: PetRef, amount: f64) {
        let amount = (amount
            * self.percent(target, "Mod_HealingDealtPercent")
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
    fn add_aura_effect(
        &mut self,
        caster: PetRef,
        target: PetRef,
        effect: &Effect,
    ) -> Result<bool, String> {
        let positive = matches!(effect.property, 26 | 63);
        let wearer = if positive { caster } else { target };
        if self.battle.pet(wearer).health <= 0 || !self.hit(caster, wearer, effect) {
            return Ok(false);
        }
        if effect.property == 52 && self.control_immune(wearer, effect.aura) {
            self.events.push(Event::Immune {
                effect: effect.id,
                target: wearer,
            });
            return Ok(false);
        }
        let tick_first = if effect.property == 26 {
            effect.params[3] != 0
        } else if effect.property == 52 {
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
        has_control
            && (self.state(wearer, "Passive_Critter") != 0 || self.state(wearer, "resilient") != 0)
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
        let remaining = if effect.params[2] == -1 {
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
        let aura = Aura {
            serial: self.battle.next_aura,
            ability: effect.aura,
            caster,
            target,
            remaining,
            turn: 1,
            applied_round: self.battle.round,
            tick_first,
            team_bound: effect.property == 63,
        };
        self.battle.next_aura += 1;
        self.apply_aura_states(&aura, 1);
        self.battle.auras.push(aura);
        self.events.push(Event::AuraApplied {
            target,
            ability: effect.aura,
        });
        self.cast_turn(caster, target, effect.aura, 0, 0)
    }
    fn remove_aura_effect(&mut self, caster: PetRef, effect: &Effect) -> Result<bool, String> {
        if !self.roll(effect.params[0]) {
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
