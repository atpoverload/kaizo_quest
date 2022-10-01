use std::cmp::{Eq, PartialEq};

use std::hash::Hash;
use std::ops::Index;
use std::vec::Vec;

use rand::{Rng, random, thread_rng};
use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

use crate::core::{Action, ActionId, Character, Species, States, Stats};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Status { Defend, Bleed, Stun }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Alignment { Rock, Paper, Scissors }

pub type OnionCharacter = Character<Alignment, Status>;

#[cfg(test)]
mod testing {
    use super::*;

    pub fn fake_stats() -> Stats<f64> {
        Stats::from_values(0.25, 0.25, 0.25, 0.25)
    }

    pub fn fake_stats_with_value<T: Copy>(value: T) -> Stats<T> {
        Stats::from_values(value, value, value, value)
    }

    pub fn fake_species() -> Species<Alignment> {
        fake_species_with_bst(0)
    }

    pub fn fake_species_with_bst(bst: u32) -> Species<Alignment> {
        Species {
            name: "fake".to_string(),
            bst,
            stats: fake_stats(),
            alignment: Alignment::Rock,
        }
    }

    pub fn fake_character() -> OnionCharacter {
        Character::from_species(fake_species())
    }

    pub fn fake_character_with_bst(bst: u32) -> OnionCharacter {
        Character::from_species(fake_species_with_bst(bst))
    }
}

// action implementations
trait Effectiveness {
    fn effectiveness(self, other: Alignment) -> u32;
}

impl Effectiveness for Alignment {
    fn effectiveness(self, other: Alignment) -> u32 {
        // TODO: we did something stupid here, see the note in attack
        match (self, other) {
            (Alignment::Rock, Alignment::Paper) |
            (Alignment::Paper, Alignment::Scissors) |
            (Alignment::Scissors, Alignment::Rock) => 5,
            (Alignment::Rock, Alignment::Scissors) |
            (Alignment::Scissors, Alignment::Paper) |
            (Alignment::Paper, Alignment::Rock) => 20,
            _ => 10,
        }
    }
}

trait Damage {
    fn deal_damage(&mut self, damage: u32);
}

impl Damage for OnionCharacter {
    fn deal_damage(&mut self, damage: u32) {
        self.state.health = std::cmp::max(0, self.state.health - damage as i32);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Attack {
    name: String,
    power: u32,
    alignment: Alignment,
    priority: i32,
}

impl Action<Alignment, Status> for Attack {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!(
            "{:?}-aligned Attack with {} power.{}",
            self.alignment,
            self.power,
            if self.priority > 0 { "\nHas priority." } else { "" }
        )
    }

    fn priority(&self) -> i32 { self.priority }

    fn act(&self, user: &mut OnionCharacter, target: &mut OnionCharacter) -> States {
        // target: &mut Character<A, S>) where A: Alignment, S: Status -> States {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Defend) {
            logs.push(format!("{} blocked {}'s {}.", target.name, user.name, self.name))
        } else {
            let level = 2 * user.attributes.level / 5 + 2;
            // TODO: this only handles "physical" alignments
            let stats = user.attributes.stats.attack / target.attributes.stats.defense;
            // TODO: this is a little stupid. this should be 1.5/1.0 but then the compiler gets
            //       mad because of u32 * float. so i offset it to the final computation
            let stab = if user.state.alignment == self.alignment { 15 } else { 10 };
            let effectiveness = self.alignment.effectiveness(target.state.alignment);
            match effectiveness {
                20 => logs.push("It's very effective.".to_string()),
                5 => logs.push("It's not very effective.".to_string()),
                0 => logs.push("It has no effect.".to_string()),
                _ => (),
            };
            // TODO: add crits
            let damage = level * self.power * stats * stab * effectiveness / 50 / 10 / 10 + 2;
            target.deal_damage(damage);
        }
        logs
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct PureAttack { name: String, power: u32 }

impl Action<Alignment, Status> for PureAttack {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Attack for exactly {} damage.", self.power)
    }

    fn act(&self, user: &mut OnionCharacter, target: &mut OnionCharacter) -> States {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Defend) {
            logs.push(format!("{} blocked {}'s attack", target.name, user.name))
        } else {
            target.deal_damage(self.power);
        }
        logs
    }
}

// TODO: i broke the status up into separate structs but it might be easier to manage as a match-like
#[derive(Clone, Serialize, Deserialize)]
struct Defend { name: String }

impl Action<Alignment, Status> for Defend {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String { format!("Defend against attacks.") }

    fn priority(&self) -> i32 { 2 }

    fn act(&self, user: &mut OnionCharacter, _: &mut OnionCharacter) -> States {
        let mut logs = Vec::new();
        logs.push(format!("{} is defending.", user.name));
        user.state.status.entry(Status::Defend).or_insert(0);
        logs
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Bleed { name: String, power: u32 }

impl Action<Alignment, Status> for Bleed {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Applies {} bleeding to the enemy.", self.power)
    }

    fn act(&self, user: &mut OnionCharacter, target: &mut OnionCharacter) -> States {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Stun) {
            logs.push(format!("But {} is stunned.", target.name));
        } else {
            target.state.status.entry(Status::Bleed).or_insert(0);
            target.state.status.entry(Status::Bleed).and_modify(|s| { *s += self.power as i32; });
            logs.push(format!("{} gained {} bleeding.", target.name, self.power));
        }
        logs
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Stun { name: String }

impl Action<Alignment, Status> for Stun {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Stuns the enemy.")
    }

    fn act(&self, user: &mut OnionCharacter, target: &mut OnionCharacter) -> States {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Bleed) {
            logs.push(format!("But {} is poisoned.", target.name));
        } else {
            target.state.status.entry(Status::Stun).or_insert(0);
            target.state.status.entry(Status::Stun).and_modify(|s| { *s += 1; });
            logs.push(format!("{} is stunned.", target.name));
        }
        logs
    }
}

struct Skip;

impl Action<Alignment, Status> for Skip {
    fn name(&self) -> String { "Skip".to_string() }

    fn description(&self) -> String {
        "User skips their next turn.".to_string()
    }

    fn act(&self, user: &mut OnionCharacter, _: &mut OnionCharacter) -> States {
        vec![format!("{} used {}.", user.name, self.name())]
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;

    fn fake_character_with_health(health: u32) -> OnionCharacter {
        let mut character = testing::fake_character();
        character.attributes.stats.health = health;
        character.refresh();
        character
    }

    pub fn fake_attack(power: u32) -> Attack {
        Attack {
            name: "fake".to_string(),
            power,
            alignment: Alignment::Scissors,
            priority: 0
        }
    }

    // TODO: non-exhaustive cases
    #[test]
    fn attack_test() {
        let mut user = testing::fake_character();
        user.attributes.stats.attack = 17;
        user.attributes.level = 19;

        let mut target = fake_character_with_health(100);
        target.attributes.stats.defense = 13;

        let action = fake_attack(11);

        action.act(&mut user, &mut target);
        assert_eq!(target.state.health, 98);
    }

    #[test]
    fn pure_attack_test() {
        let mut user = testing::fake_character();
        let mut target = fake_character_with_health(10);
        let action = PureAttack { name: "fake".to_string(), power: 5 };

        action.act(&mut user, &mut target);
        assert_eq!(target.state.health, 5);

        let mut user = user.clone();
        let mut target = target.clone();
        action.act(&mut user, &mut target);
        assert_eq!(target.state.health, 0);

        let mut user = user.clone();
        let mut target = fake_character_with_health(4);
        action.act(&mut user, &mut target);
        assert_eq!(target.state.health, 0);
    }

    #[test]
    fn defend_test() {
        let mut user = testing::fake_character();
        let mut target = fake_character_with_health(10);
        let defend = Defend { name: "fake".to_string() };

        let attack = PureAttack { name: "fake".to_string(), power: 5 };

        defend.act(&mut target, &mut user);
        assert_eq!(target.state.status.contains_key(&Status::Defend), true);

        let mut user = user.clone();
        let mut target = target.clone();
        attack.act(&mut user, &mut target);

        assert_eq!(target.state.health, 10);

        let attack = Attack { name: "fake".to_string(), power: 5, alignment: Alignment::Rock, priority: 0 };

        let mut user = user.clone();
        let mut target = target.clone();
        defend.act(&mut target, &mut user);

        let mut user = user.clone();
        let mut target = target.clone();
        attack.act(&mut user, &mut target);

        assert_eq!(target.state.health, 10);
    }

    #[test]
    fn stun_test() {
        let mut user = testing::fake_character();
        let mut target = testing::fake_character();
        let action = Stun { name: "fake".to_string() };

        action.act(&mut user, &mut target);
        assert_eq!(target.state.status.contains_key(&Status::Stun), true);
        assert_eq!(target.state.status.get(&Status::Stun), Some(&1));

        let mut user = user.clone();
        let mut target = target.clone();
        action.act(&mut user, &mut target);
        assert_eq!(target.state.status.contains_key(&Status::Stun), true);
        assert_eq!(target.state.status.get(&Status::Stun), Some(&2));
    }

    #[test]
    fn bleed_test() {
        let mut user = testing::fake_character();
        let mut target = testing::fake_character();
        let action = Bleed { name: "fake".to_string(), power: 1 };

        action.act(&mut user, &mut target);
        assert_eq!(target.state.status.contains_key(&Status::Bleed), true);
        assert_eq!(target.state.status.get(&Status::Bleed), Some(&1));

        let mut user = user.clone();
        let mut target = target.clone();
        action.act(&mut user, &mut target);
        assert_eq!(target.state.status.contains_key(&Status::Bleed), true);
        assert_eq!(target.state.status.get(&Status::Bleed), Some(&2));
    }
}

// growth functions
pub trait Experience<E> {
    fn experience(&self) -> E;

    fn gain_experience(&mut self, experience: E) -> States;
}

// TODO: maybe these should be configurable? might have to be part of the species
static BASE_EXPERIENCE: u32 = 31;
static GROWTH_FACTOR: u32 = 47;

pub static EXPERIENCE_TO_LEVEL: u32 = 100;
pub static SCALING_FACTOR: u32 = 100;

impl <A, S: Eq + Hash + PartialEq> Experience<u32> for Character<A, S> {
    fn experience(&self) -> u32 {
        if self.attributes.level == 0 || self.species.bst == 0 { return 0; }
        let log2u32 = |x| if x > 0 { (x as f64).log(2.0) as u32 } else { 0 };
        let bst = self.species.bst * log2u32(self.species.bst + 1);
        let level = self.attributes.level / log2u32(self.attributes.level + 1);
        bst * level / BASE_EXPERIENCE
    }

    fn gain_experience(&mut self, experience: u32) -> States {
        let mut logs = vec![];
        logs.push(format!("Gained {} experience!", experience));
        let experience = self.attributes.experience + experience;
        self.attributes.experience = experience % EXPERIENCE_TO_LEVEL;
        let levels = experience / EXPERIENCE_TO_LEVEL;
        self.attributes.level += levels;
        if levels > 0 {
            let stats = self.species.stats.scale(SCALING_FACTOR);
            logs.push(format!("Stats increased by {:?}", stats));
            self.attributes.stats += stats;
        }
        logs
    }
}

#[cfg(test)]
mod experience_tests {
    use super::*;

    #[test]
    fn experience_sanity_test() {
        let mut character = testing::fake_character();

        // not set up
        assert_eq!(character.experience(), 0);

        // no bst
        character.attributes.level = 1;
        assert_eq!(character.experience(), 0);

        // no level
        character.attributes.level = 0;
        character.species.bst = 1;
        assert_eq!(character.experience(), 0);
    }

    // TODO: make parameterized tests
    // TODO: we should get this from ground truth values
    #[test]
    fn experience_table_test1() {
        let mut character = testing::fake_character();

        character.attributes.level = 1;

        character.species.bst = 100;
        assert_eq!(character.experience(), 19);

        character.species.bst = 200;
        assert_eq!(character.experience(), 45);

        character.species.bst = 300;
        assert_eq!(character.experience(), 77);

        character.species.bst = 400;
        assert_eq!(character.experience(), 103);

        character.species.bst = 500;
        assert_eq!(character.experience(), 129);

        character.species.bst = 600;
        assert_eq!(character.experience(), 174);
    }

    #[test]
    fn experience_table_test2() {
        let mut character = testing::fake_character();

        character.species.bst = 450;

        character.attributes.level = 1;
        assert_eq!(character.experience(), 116);

        character.attributes.level = 5;
        assert_eq!(character.experience(), 232);

        character.attributes.level = 10;
        assert_eq!(character.experience(), 348);

        character.attributes.level = 25;
        assert_eq!(character.experience(), 696);

        character.attributes.level = 50;
        assert_eq!(character.experience(), 1161);

        character.attributes.level = 100;
        assert_eq!(character.experience(), 1858);
    }

    // TODO: fix this once the states aren't strings
    #[test]
    fn gain_experience_test() {
        let mut character = testing::fake_character();

        let _ = character.gain_experience(1);
        assert_eq!(character.attributes.experience, 1);
        // assert_eq!(levels, 0);

        let _ = character.gain_experience(100);
        assert_eq!(character.attributes.experience, 1);
        // assert_eq!(levels, 1);

        let _ = character.gain_experience(99);
        assert_eq!(character.attributes.experience, 0);
        // assert_eq!(levels, 1);

        let _ = character.gain_experience(234);
        assert_eq!(character.attributes.experience, 34);
        // assert_eq!(levels, 2);
    }
}

pub trait Scale {
    fn scale(&self, a: u32) -> Stats<u32>;
}

impl Scale for Stats<f64> {
    // linearly scales floats to have a total sum equal to some integer; there may be a rounding error
    fn scale(&self, a: u32) -> Stats<u32> {
        let x: Vec<f64> = self.into();
        let z: f64 = x.to_vec().into_iter().sum();
        x.iter().map(|x| a as f64 * *x / z).map(|x| x as u32).collect::<Vec<u32>>().into()
    }
}

impl <A> Scale for Species<A> {
    fn scale(&self, a: u32) -> Stats<u32> {
        let growth_factor = a * self.bst / GROWTH_FACTOR;
        let mut stats: Vec<u32> = self.stats.scale(growth_factor).into();
        // TODO: randomly correct the stats if they don't add up to the growth factor
        let growth_factor = (growth_factor - stats.clone().iter().sum::<u32>()) as usize;
        let n = stats.len();
        let _ = &thread_rng().sample_iter(Standard).take(growth_factor).for_each(|i: usize| stats[i % n] += 1);
        return stats.into();
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;

    #[test]
    fn scale_stats_test() {
        let base_stats = testing::fake_stats();

        let scaled_stats = testing::fake_stats_with_value(25);

        assert_eq!(base_stats.scale(100), scaled_stats);

        let scaled_stats = testing::fake_stats_with_value(560);

        assert_eq!(base_stats.scale(2243), scaled_stats);
    }

    // TODO: this test doesn't do anything useful
    #[test]
    fn scale_species_test() {
        let species = testing::fake_species_with_bst(400);

        let scaled_stats = Stats {
            health: 2,
            attack: 2,
            defense: 2,
            speed: 2,
        };

        assert_eq!(species.scale(1), scaled_stats);

        let species = testing::fake_species_with_bst(450);

        let scaled_stats = Stats {
            health: 2,
            attack: 2,
            defense: 2,
            speed: 2,
        };

        assert_ne!(species.scale(1), scaled_stats);

        let species = testing::fake_species_with_bst(550);

        let scaled_stats = Stats {
            health: 3,
            attack: 3,
            defense: 3,
            speed: 3,
        };

        assert_ne!(species.scale(1), scaled_stats);
    }
}

// battle logic
fn take_turn(user: &mut OnionCharacter, target: &mut OnionCharacter, action: &dyn Action<Alignment, Status>) -> States {
    if user.state.status.contains_key(&Status::Stun) {
        if random::<u32>() % (*user.state.status.get(&Status::Stun).unwrap() as u32 + 1) == 0 {
            user.state.status.remove(&Status::Stun);
            let mut logs = Vec::new();
            logs.push(format!("{} is no longer stunned.", user.name));
            logs.extend(action.act(user, target));
            logs
        } else {
            vec![format!("{} is stunned.", user.name)]
        }
    } else if user.state.status.contains_key(&Status::Bleed) {
        let mut logs = Vec::new();
        logs.extend(action.act(user, target));
        user.state.health = std::cmp::max(0, user.state.health - *user.state.status.get(&Status::Bleed).unwrap());
        logs.push(format!("{} was hurt by bleed.", user.name));
        logs
    } else {
        action.act(user, target)
    }
}

fn clean_up(character: &mut OnionCharacter) {
    if character.state.status.contains_key(&Status::Defend) {
        character.state.status.remove(&Status::Defend);
    }
}

#[derive(Clone)]
pub enum OnionBattleState {
    Defeat,
    InProcess,
    Victory,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OnionBattle {
    pub player: OnionCharacter,
    pub enemy: OnionCharacter,
}

// TODO: this is better but is still messy
impl OnionBattle {
    fn battle_state(&self) -> OnionBattleState {
        if self.player.state.health == 0 {
            return OnionBattleState::Defeat
        } else if self.enemy.state.health == 0 {
            return OnionBattleState::Victory
        } else {
            return OnionBattleState::InProcess
        }
    }

    fn clean_up(&mut self) {
        clean_up(&mut self.player);
        clean_up(&mut self.enemy);
    }

    pub fn player_turn(&mut self, action: &dyn Action<Alignment, Status>) -> States {
        let state = self.battle_state();
        if let OnionBattleState::InProcess = state {
            take_turn(&mut self.player, &mut self.enemy, action)
        } else { vec![] }
    }

    pub fn enemy_turn(&mut self, action: &dyn Action<Alignment, Status>) -> States {
        let state = self.battle_state();
        if let OnionBattleState::InProcess = state {
            take_turn(&mut self.enemy, &mut self.player, action)
        } else { vec![] }
    }

    pub fn end_turn(&mut self) -> (OnionBattleState, States) {
        let mut logs = Vec::new();
        let state = match self.battle_state() {
            OnionBattleState::Victory => {
                // award xp
                logs.push(format!("Defeated {}!", self.enemy.name));
                let experience: u32 = self.enemy.experience() / self.player.attributes.level;
                logs.extend(self.player.gain_experience(experience));
                OnionBattleState::Victory
            },
            OnionBattleState::Defeat => {
                logs.push(format!("{} died!", self.player.name));
                OnionBattleState::Defeat
            },
            _ => {
                self.clean_up();
                OnionBattleState::InProcess
            }
        };
        (state, logs)
    }
}

#[cfg(test)]
mod battle_tests {
    use super::*;

    fn fake_character(level: u32) -> OnionCharacter {
        let mut character = testing::fake_character_with_bst(400);
        character.attributes.level = level;
        character.attributes.stats = character.species.stats.scale(10 * level);
        character.refresh();
        character
    }

    // TODO: this does nothing; exercise all cases
    #[test]
    fn battle_test() {
        let mut battle = OnionBattle { player: fake_character(5), enemy: fake_character(5) };

        let action = action_tests::fake_attack(30);
        battle.player_turn(&action);

        assert_eq!(battle.enemy.state.health, 9);
    }
}

// tools to generate content
// TODO: figure out how to implement sample_iter?
impl Distribution<Stats<f64>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Stats<f64> {
        let x = self.sample_iter(rng).take(4).collect::<Vec<f64>>();
        let z: f64 = x.iter().sum();
        x.iter().map(|x| x / z).collect::<Vec<f64>>().into()
    }
}

impl Distribution<Alignment> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Alignment {
        match rng.gen_range(0..3) {
            0 => Alignment::Rock,
            1 => Alignment::Paper,
            _ => Alignment::Scissors,
        }
    }
}

// TODO: this is only generatable through rust. we want to define this stuff externally
static WORST_BST: u32 = 200u32;
static BEST_BST: u32 = 700u32;

#[derive(Debug)]
enum OnionName {
    Pawn,
    Knight,
    Rook,
    Bishop,
    Queen,
    King,
}

impl Distribution<OnionName> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OnionName {
        match rng.gen_range(0..6) {
            0 => OnionName::Pawn,
            1 => OnionName::Knight,
            2 => OnionName::Rook,
            3 => OnionName::Bishop,
            4 => OnionName::Queen,
            _ => OnionName::King,
        }
    }
}

impl Distribution<Species<Alignment>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Species<Alignment> {
        let alignment = self.sample(rng);
        let suffix: OnionName = self.sample(rng);
        Species {
            name: format!("{:?} {:?}", alignment, suffix), // TODO: generate species name
            bst: rng.gen_range(WORST_BST..BEST_BST),
            stats: self.sample(rng),
            alignment,
        }
    }
}

impl Distribution<OnionCharacter> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OnionCharacter {
        Character::from_species(self.sample(rng))
    }
}

#[derive(Debug)]
enum AttackName {
    Fist,
    Punch,
    Kick,
    Jab,
    Chop,
    Slam,
    Foot,
    Knee,
    Elbow,
    Headbutt,
    Charge,
}

impl Distribution<AttackName> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> AttackName {
        match rng.gen_range(0..11) {
            0 => AttackName::Fist,
            1 => AttackName::Punch,
            2 => AttackName::Kick,
            3 => AttackName::Jab,
            4 => AttackName::Chop,
            5 => AttackName::Slam,
            6 => AttackName::Foot,
            7 => AttackName::Knee,
            8 => AttackName::Elbow,
            9 => AttackName::Headbutt,
            _ => AttackName::Charge,
        }
    }
}

static WORST_ATTACK: u32 = 10u32;
static BEST_ATTACK: u32 = 150u32;
static PRIORITY_MOVE_CHANCE: i32 = 4i32;

impl Distribution<Attack> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Attack {
        let alignment: Alignment = self.sample(rng);
        let suffix: AttackName = self.sample(rng);
        Attack {
            name: format!("{:?} {:?}", alignment, suffix),
            power: rng.gen_range(WORST_ATTACK..BEST_ATTACK),
            alignment,
            priority: rng.gen::<i32>() % PRIORITY_MOVE_CHANCE / PRIORITY_MOVE_CHANCE,
        }
    }
}

// TODO: do we need any tests?

// TODO: this is a stupid hack since the actions for characters are usize
static SKIP: Skip = Skip;

#[derive(Clone, Serialize, Deserialize)]
pub struct ActionPool {
    attack: Vec<Attack>,
    pure_attack: Vec<PureAttack>,
    defend: Vec<Defend>,
    bleed: Vec<Bleed>,
    stun: Vec<Stun>,
    padding: usize,
}

impl ActionPool {
    fn empty_pool() -> ActionPool {
        ActionPool {
            attack: vec![],
            pure_attack: vec![],
            defend: vec![],
            bleed: vec![],
            stun: vec![],
            padding: 0,
        }
    }

    fn with_padding(attack: Vec<Attack>, padding: usize) -> ActionPool {
        ActionPool {
            attack,
            pure_attack: vec![
                PureAttack { name: "Burst".to_string(), power: 20 },
                PureAttack { name: "Blast".to_string(), power: 40 },
            ],
            defend: vec![
                Defend { name: "Block".to_string() },
                Defend { name: "Dodge".to_string() },
            ],
            bleed: vec![
                Bleed { name: "Cut".to_string(), power: 1 },
                Bleed { name: "Slice".to_string(), power: 1 },
            ],
            stun: vec![
                Stun { name: "Lullabye".to_string() },
                Stun { name: "Paralyze".to_string() },
                Stun { name: "Yawn".to_string() },
            ],
            padding
        }
    }

    fn with_attacks(attack: Vec<Attack>) -> ActionPool {
        ActionPool::with_padding(attack, 0usize)
    }

    fn len(&self) -> usize {
        self.attack.len() +
        self.pure_attack.len() +
        self.defend.len() +
        self.bleed.len() +
        self.stun.len()
    }
}

impl Index<ActionId> for ActionPool {
    type Output = dyn Action<Alignment, Status>;

    fn index(&self, action: ActionId) -> &Self::Output {
        let mut id = action.clone();
        if id < self.attack.len() {
            return &self.attack[id];
        } else {
            id -= self.attack.len();
        }

        if id < self.pure_attack.len() {
            return &self.pure_attack[id];
        } else {
            id -= self.pure_attack.len();
        }

        if id < self.defend.len() {
            return &self.defend[id];
        } else {
            id -= self.defend.len();
        }

        if id < self.bleed.len() {
            return &self.bleed[id];
        } else {
            id -= self.bleed.len();
        }

        if id < self.stun.len() {
            return &self.stun[id];
        }

        &SKIP
    }
}

// TODO: figure out how to implement sample_iter
impl Distribution<ActionId> for ActionPool {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ActionId {
        rng.gen_range(0..(self.len() + self.padding))
    }
}

#[cfg(test)]
mod action_pool_tests {
    use super::*;

    #[test]
    fn empty_action_pool_test() {
        let pool = ActionPool::empty_pool();

        let name = SKIP.name();
        assert_eq!(pool[0].name(), name);
        assert_eq!(pool[1].name(), name);
        assert_eq!(pool[std::usize::MAX].name(), name);
        assert_eq!(pool[std::usize::MIN].name(), name);
    }

    #[test]
    fn action_pool_test1() {
        let action = action_tests::fake_attack(0);
        let action_name = action.name();
        let mut pool = ActionPool::empty_pool();
        pool.attack.push(action);

        let skip_name = SKIP.name();
        assert_eq!(pool[0].name(), action_name);
        assert_eq!(pool[1].name(), skip_name);
        assert_eq!(pool[std::usize::MIN].name(), action_name);
        assert_eq!(pool[std::usize::MAX].name(), skip_name);
    }
}

#[derive(Serialize, Deserialize)]
pub struct OnionWorld {
    species: Vec<Species<Alignment>>,
    pub actions: ActionPool,
}

impl Distribution<OnionCharacter> for OnionWorld {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OnionCharacter {
        Character::from_species_and_actions(
            self.species.choose(rng).unwrap().clone(),
            self.actions.clone().sample_iter(&mut thread_rng()).take(4).collect()
        )
    }
}

impl Distribution<ActionPool> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ActionPool {
        let padding = rng.gen_range(0..20);
        let attacks = 20 * 3 - padding;
        ActionPool::with_padding(self.sample_iter(rng).take(attacks).collect(), padding)
    }
}

static SPECIES_COUNT: usize = 351usize;

impl Distribution<OnionWorld> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OnionWorld {
        OnionWorld {
            actions: rng.gen(),
            species: self.sample_iter(rng).take(SPECIES_COUNT).collect(),
        }
    }
}

impl OnionWorld {
    pub fn sample_at_level<R: Rng + ?Sized>(&self, level: u32, rng: &mut R) -> OnionCharacter {
        let mut character = self.sample(rng);
        character.gain_experience(level * EXPERIENCE_TO_LEVEL);
        character.attributes.stats = character.species.stats.scale(level * SCALING_FACTOR);
        character.refresh();
        character
    }
}

// fn generate_world() {
//     let world: OnionWorld = Standard.sample(&mut thread_rng());
//     let mut character: OnionCharacter = world.sample(&mut thread_rng());
//     character.gain_experience(&mut thread_rng().gen(0..100) * 100);
//     character.actions = world.actions.sample_iter(&mut thread_rng()).take(4);
// }
