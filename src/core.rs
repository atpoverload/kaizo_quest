use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Add, AddAssign};

use num_traits::identities::Zero;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats<T> {
    pub health: T,
    pub attack: T,
    pub defense: T,
    pub speed: T,
}

// convenience methods for adjusting stats
// TODO: this implementation only handles "physical" stats
impl <T> From<Vec<T>> for Stats<T> where T: Clone + Copy {
    fn from(stats: Vec<T>) -> Self {
        Stats {
            health: *stats.get(0).unwrap(),
            attack: *stats.get(1).unwrap(),
            defense: *stats.get(2).unwrap(),
            speed: *stats.get(3).unwrap(),
        }
    }
}

impl <T: Copy> From<Stats<T>> for Vec<T> {
    fn from(stats: Stats<T>) -> Self {
        vec![stats.health, stats.attack, stats.defense, stats.speed]
    }
}

impl <T: Copy> From<&Stats<T>> for Vec<T> {
    fn from(stats: &Stats<T>) -> Self {
        vec![stats.health, stats.attack, stats.defense, stats.speed]
    }
}

impl <T: Add<T, Output = T>> Add<Stats<T>> for Stats<T> {
    type Output = Stats<T>;

    fn add(self, other: Stats<T>) -> Stats<T> {
        Stats {
            health: self.health + other.health,
            attack: self.attack + other.attack,
            defense: self.defense + other.defense,
            speed: self.speed + other.speed,
        }
    }
}

impl <T: AddAssign> AddAssign for Stats<T> {
    fn add_assign(&mut self, other: Stats<T>) {
        self.health += other.health;
        self.attack += other.attack;
        self.defense += other.defense;
        self.speed += other.speed;
    }
}

impl <T: Zero + PartialEq> Zero for Stats<T> {
    fn zero() -> Self {
        Stats {
            health: T::zero(),
            attack: T::zero(),
            defense: T::zero(),
            speed: T::zero(),
        }
    }

    fn is_zero(&self) -> bool {
        let zero = T::zero();
        self.health == zero && self.attack == zero && self.defense == zero && self.speed == zero
    }

    fn set_zero(&mut self) {
        *self = Stats::zero();
    }
}

impl <T> Stats<T> {
    pub fn from_values(health: T, attack: T, defense: T, speed: T) -> Stats<T> {
        Stats {
            health,
            attack,
            defense,
            speed,
        }
    }
}

#[cfg(test)]
mod stats_tests {
    use super::*;

    fn zero_test<T: Add + Zero + Debug + PartialEq>() {
        let mut zero: Stats<T> = Stats::zero();
        assert_eq!(zero.is_zero(), true);
        zero.set_zero();
        assert_eq!(zero.is_zero(), true);
    }

    #[test]
    fn zero_tests() {
        zero_test::<f64>();
        zero_test::<u32>();
    }

    #[test]
    fn add_test() {
        let mut stats = Stats { health: 1, attack: 1, defense: 1, speed: 1 };
        assert_eq!(stats + Stats::zero(), stats);
        let stats2 = Stats { health: 0, attack: 1, defense: 2, speed: 3 };
        assert_eq!(stats + stats, Stats { health: 2, attack: 2, defense: 2, speed: 2 });
        stats += stats2;
        assert_eq!(stats, Stats { health: 1, attack: 2, defense: 3, speed: 4 });
    }
}

// properties describing the character generally
// TODO: it would be nice for this to have a notion of the actions the species would learn
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species<A> {
    pub name: String,
    pub bst: u32,
    pub stats: Stats<f64>,
    pub alignment: A,
}

// TODO: This needs to be abstracted but then we will need to pipe forward generics
pub type ActionId = usize;
pub type Actions = Vec<ActionId>;

// describes the fixed state in a battle
// TODO: abstract the level + experience
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attributes {
    pub level: u32,
    pub experience: u32,
    pub stats: Stats<u32>,
    pub actions: Actions,
}

// describes the changing state within a battle
// TODO: push status into a trait or function
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State<A, S: Eq + Hash + PartialEq> {
    pub alignment: A,
    pub health: i32,
    pub status: HashMap<S, i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character<A, S: Eq + Hash + PartialEq> {
    pub name: String,
    pub species: Species<A>,
    pub attributes: Attributes,
    pub state: State<A, S>,
}

impl <A: Clone, S: Eq + Hash + PartialEq> Character<A, S> {
    pub fn from_species(species: Species<A>) -> Character<A, S> {
        let alignment = species.alignment.clone();
        Character {
            name: species.name.to_string(),
            species,
            attributes: Attributes {
                level: 0,
                experience: 0,
                stats: Stats::zero(),
                actions: Vec::new(),
            },
            state: State {
                alignment,
                health: 0,
                status: HashMap::new(),
            }
        }
    }

    pub fn from_species_and_actions(species: Species<A>, actions: Actions) -> Character<A, S> {
        let mut character = Character::from_species(species);
        character.attributes.actions = actions;
        character
    }

    pub fn priority(&self) -> i32 { self.attributes.stats.speed as i32 }

    pub fn refresh(&mut self) {
        self.state.alignment = self.species.alignment.clone();
        self.state.health = self.attributes.stats.health as i32;
        self.state.status = HashMap::new();
    }
}

// TODO: This needs to be abstracted but then we will need to pipe forward generics
// TODO: should be a list of states that can be applied sequentially
pub type States = Vec<String>;

pub trait Action<A, S: Eq + Hash + PartialEq> {
    fn name(&self) -> String;
    fn description(&self) -> String { self.name() }
    fn priority(&self) -> i32 { 0 }
    fn act(&self, user: &mut Character<A, S>, target: &mut Character<A, S>) -> States;
}
