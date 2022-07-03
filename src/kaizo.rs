use std::collections::HashMap;
use std::ops::{Add, AddAssign};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats<T> {
    pub health: T,
    pub attack: T,
    pub defense: T,
    pub speed: T,
}

impl Stats<u32> {
    pub fn zero() -> Stats<u32> {
        Stats {
            health: 0,
            attack: 0,
            defense: 0,
            speed: 0,
        }
    }
}

// convenience methods for adjusting stats
impl Add<Stats<u32>> for Stats<u32> {
    type Output = Stats<u32>;

    fn add(self, other: Stats<u32>) -> Stats<u32> {
        Stats {
            health: self.health + other.health,
            attack: self.attack + other.attack,
            defense: self.defense + other.defense,
            speed: self.speed + other.speed,
        }
    }
}

impl AddAssign for Stats<u32> {
    fn add_assign(&mut self, other: Stats<u32>) {
        self.health += other.health;
        self.attack += other.attack;
        self.defense += other.defense;
        self.speed += other.speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_zero(stats: Stats<u32>) -> bool {
        stats.health == 0 && stats.attack == 0 && stats.defense == 0 && stats.speed == 0
    }

    #[test]
    fn stats_test() {
        let zero = Stats::zero();
        assert_eq!(is_zero(zero), true);
        assert_eq!(zero, Stats::zero());
        assert_eq!(zero + Stats::zero(), Stats::zero());

        let stats = Stats { health: 1, attack: 1, defense: 1, speed: 1 };
        let stats2 = Stats { health: 0, attack: 1, defense: 2, speed: 3 };
        assert_eq!(stats + stats, Stats { health: 2, attack: 2, defense: 2, speed: 2 });
        assert_eq!(stats + stats2, Stats { health: 1, attack: 2, defense: 3, speed: 4 });
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Status {
    Defend,
    Poison,
    Sleep,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment { A, B, C, D }

#[derive(Clone, Debug, PartialEq)]
pub struct Species {
    pub name: String,
    pub bst: u32,
    pub base_stats: Stats<f64>,
    pub alignment: Alignment,
}

pub type ActionId = usize;
pub type Actions = Vec<ActionId>;

#[derive(Clone, Debug, PartialEq)]
pub struct Attributes {
    pub level: u32,
    pub experience: u32,
    pub stats: Stats<u32>,
    pub actions: Actions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub health: i32,
    pub status: HashMap<Status, i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub name: String,
    pub species: Species,
    pub attributes: Attributes,
    pub state: State,
}

impl Character {
    pub fn from_species(species: Species) -> Character {
        Character {
            name: species.name.to_string(),
            species: species,
            attributes: Attributes {
                level: 0,
                experience: 0,
                stats: Stats::zero(),
                actions: Vec::new(),
            },
            state: State {
                health: 0,
                status: HashMap::new(),
            }
        }
    }

    pub fn priority(&self) -> i32 { self.attributes.stats.speed as i32 }

    pub fn deal_damage(&mut self, damage: i32) {
        self.state.health = std::cmp::max(0, self.state.health - damage);
    }

    pub fn refresh(&mut self) {
        self.state.health = self.attributes.stats.health as i32;
        self.state.status = HashMap::new();
    }
}

// TODO: this should be a list of states that can be applied sequentially
pub type ActionLog = Vec<String>;

pub trait Action {
    fn name(&self) -> String;
    fn description(&self) -> String { self.name() }
    fn priority(&self) -> i32 { 0 }
    fn act(&self, user: &mut Character, target: &mut Character) -> ActionLog;
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn stats_test() {
//         let health = 1;
//         let attack = 2;
//         let defense = 3;
//         let speed = 4;
//
//         let stats = Stats { health, attack, defense, speed };
//
//         assert_eq!(stats.total(), health + attack + defense + speed);
//         assert_eq!(stats, Stats::from(health, attack, defense, speed));
//         assert_eq!(stats + stats, Stats { health: 2 * health, attack: 2 * attack, defense: 2 * defense, speed: 2 * speed});
//     }
// }
