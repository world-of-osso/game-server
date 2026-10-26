//! SimC76f8118 sc_druid.cpp9372–9385/9469–9522: Cat cards and weights.
use super::combat::random::{self, Rng, seq::SliceRandom};
use bevy::prelude::Component;

#[derive(Clone, Copy)]
pub(super) enum Card {
    Offspec,
    Main,
    Spec,
    Exceptional,
}

pub(super) enum Action {
    Heal,
    Moonfire,
    Wrath,
    Bite,
    Rake,
    Shred,
    Frenzy,
}

#[derive(Component, Default)]
pub(super) struct ExceptionalDeck(Vec<bool>);

impl ExceptionalDeck {
    // sc_druid.cpp12415: one success per five channels, or two with Guidance.
    pub(super) fn draw(&mut self, guidance: bool) -> bool {
        if self.0.is_empty() {
            self.0.resize(if guidance { 2 } else { 5 }, false);
            self.0[0] = true;
            self.0.shuffle(&mut random::thread_rng());
        }
        self.0.pop().expect("refilled native Convoke exceptional deck")
    }
}

pub(super) fn roll_cards(ticks: usize, guidance: bool, exceptional: bool) -> Vec<Card> {
    let offspec_range = if guidance { 2.5..7.5 } else { 4.0..9.0 };
    let offspec = random::thread_rng().gen_range(offspec_range) as usize;
    let mut cards = vec![Card::Offspec; offspec];
    if exceptional {
        cards.push(Card::Exceptional);
    }
    assert!(ticks > cards.len(), "native Convoke clock must fit its Cat deck");
    let bites = roll_bite_count(guidance, ticks - cards.len());
    cards.extend(std::iter::repeat_n(Card::Main, bites));
    cards.resize(ticks, Card::Spec);
    cards
}

fn roll_bite_count(guidance: bool, remaining: usize) -> usize {
    let (mean, deviation) = if guidance { (3.0, 0.8) } else { (4.2, 0.9360890055) };
    // gauss_ab is a bounded normal, not a clamped count. Native RNG remains seeded
    // only by the ordinary simulation scope; no expected damage enters this draw.
    loop {
        let uniform: f64 = random::thread_rng().gen_range(f64::MIN_POSITIVE..1.0);
        let angle = random::thread_rng().gen_range(0.0..std::f64::consts::TAU);
        let normal = (-2.0 * uniform.ln()).sqrt() * angle.cos();
        let sample = mean + deviation * normal;
        if (0.0..remaining as f64).contains(&sample) {
            return sample as usize;
        }
    }
}

pub(super) fn roll_action(card: Card) -> Action {
    match card {
        Card::Main => Action::Bite,
        Card::Exceptional => Action::Frenzy,
        Card::Spec => {
            if random::thread_rng().gen_bool(0.10 / (0.10 + 0.22)) {
                Action::Shred
            } else {
                Action::Rake
            }
        }
        Card::Offspec => roll_offspec(),
    }
}

fn roll_offspec() -> Action {
    let heal_weight = 0.35;
    let moonfire_weight = 0.50;
    let roll = random::thread_rng().gen_range(0.0..1.0);
    if roll < heal_weight {
        Action::Heal
    } else if roll < heal_weight + moonfire_weight {
        Action::Moonfire
    } else {
        Action::Wrath
    }
}
