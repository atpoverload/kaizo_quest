use std::sync::Arc;
use std::vec::Vec;

use rand::{Rng, random, thread_rng};
use rand::distributions::{Distribution, Standard, Uniform};
use rand::seq::SliceRandom;

use crate::kaizo::{Action, Actions, ActionId, ActionLog, Alignment, Character, Species, Stats, Status};

// action implementations
struct PureAttack { name: String, power: u32 }

impl Action for PureAttack {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Attack for exactly {} damage.", self.power)
    }

    fn act(&self, user: &mut Character, target: &mut Character) -> ActionLog {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Defend) {
            logs.push(format!("{} blocked {}'s attack", target.name, user.name))
        } else {
            target.deal_damage(self.power as i32);
        }
        logs
    }
}

impl Alignment {
    pub fn effectiveness(self, other: Alignment) -> u32 {
        // TODO: we did something stupid here, see the note in attack
        match (self, other) {
            (Alignment::A, Alignment::B) => 20,
            (Alignment::B, Alignment::C) => 20,
            (Alignment::C, Alignment::D) => 20,
            (Alignment::D, Alignment::A) => 20,
            (Alignment::A, Alignment::C) => 5,
            (Alignment::B, Alignment::D) => 5,
            (Alignment::C, Alignment::A) => 5,
            (Alignment::D, Alignment::B) => 5,
            _ => 10,
        }
    }
}

struct Attack {
    name: String,
    power: u32,
    alignment: Alignment,
    priority: i32,
}

impl Action for Attack {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!(
            "{:?}-aligned Attack with {} power.\n{}",
            self.alignment,
            self.power,
            if self.priority > 0 { "Has priority." } else { "" }
        )
    }

    fn priority(&self) -> i32 { self.priority }

    fn act(&self, user: &mut Character, target: &mut Character) -> ActionLog {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Defend) {
            logs.push(format!("{} blocked {}'s {}.", target.name, user.name, self.name))
        } else {
            let level = 2 * user.attributes.level / 5 + 2;
            let stats = user.attributes.stats.attack / target.attributes.stats.defense;
            // TODO: this is a little stupid. this should be 1.5/1.0 but then the compiler gets
            //       mad because of u32 * float. so i offset it to the final computation
            let stab = if user.species.alignment == self.alignment { 15 } else { 10 };
            let effectiveness = self.alignment.effectiveness(target.species.alignment);
            match effectiveness {
                20 => logs.push("It's very effective.".to_string()),
                5 => logs.push("It's not very effective.".to_string()),
                0 => logs.push("It has no effect.".to_string()),
                _ => (),
            };
            // TODO: add crits
            let damage = level * self.power * stats * stab * effectiveness / 50 / 10 / 10 + 2;
            target.deal_damage(damage as i32);
        }
        logs
    }
}

struct Defend { name: String }

impl Action for Defend {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String { format!("Defend against attacks.") }

    fn priority(&self) -> i32 { 2 }

    fn act(&self, user: &mut Character, _: &mut Character) -> ActionLog {
        let mut logs = Vec::new();
        logs.push(format!("{} is defending.", user.name));
        user.state.status.entry(Status::Defend).or_insert(0);
        logs
    }
}

struct Poison { name: String, power: u32 }

impl Action for Poison {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Applies {} poison to the enemy.", self.power)
    }

    fn act(&self, user: &mut Character, target: &mut Character) -> ActionLog {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Sleep) {
            logs.push(format!("But {} is sleeping.", target.name));
        } else {
            target.state.status.entry(Status::Poison).or_insert(0);
            target.state.status.entry(Status::Poison).and_modify(|s| { *s += self.power as i32; });
            logs.push(format!("{} poison was applied to {}.", self.power, target.name));
        }
        logs
    }
}

struct Sleep { name: String }

impl Action for Sleep {
    fn name(&self) -> String { format!("{}", self.name) }

    fn description(&self) -> String {
        format!("Puts the enemy to sleep.")
    }

    fn act(&self, user: &mut Character, target: &mut Character) -> ActionLog {
        let mut logs = Vec::new();
        logs.push(format!("{} used {}.", user.name, self.name));
        if target.state.status.contains_key(&Status::Poison) {
            logs.push(format!("But {} is poisoned.", target.name));
        } else {
            target.state.status.entry(Status::Sleep).or_insert(0);
            target.state.status.entry(Status::Sleep).and_modify(|s| { *s += 1; });
            logs.push(format!("{} was put to sleep.", target.name));
        }
        logs
    }
}

// character generation
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

impl <T> From<Stats<T>> for Vec<T> {
    fn from(stats: Stats<T>) -> Self {
        vec![stats.health, stats.attack, stats.defense, stats.speed]
    }
}

// TODO: these were estimated by hand from me experimenting; i'd like to write some more rigorous
//       tests to see how the equations behave better
fn scale_to(x: &Vec<f64>, a: u32) -> Vec<u32> {
    let z: f64 = x.to_vec().into_iter().sum();
    x.iter().map(|x| a as f64 * *x / z).map(|x| x as u32).collect()
}

fn softmax(x: &Vec<f64>) -> Vec<f64> {
    let z: f64 = x.iter().map(|x| std::f64::consts::E.powf(*x as f64)).sum();
    x.into_iter().map(|x| std::f64::consts::E.powf(*x as f64) / z).collect()
}

fn generate_species<D1, D2, D3>(
    names: Vec<String>,
    stats_dist: D1,
    bst_dist: D2,
    alignment_dist: D3,
) -> Species where
    D1: Distribution<f64>,
    D2: Distribution<u32>,
    D3: Distribution<Alignment>,
{
    // TODO: we need to reweight the species so that health is a little higher; i should
    //       investigate a formalized correction
    let mut stats = thread_rng().sample_iter(stats_dist).take(4).collect::<Vec<f64>>();
    stats[0] += 0.50;
    let stats = softmax(&stats);
    Species {
        name: names.choose(&mut thread_rng()).unwrap().to_string(),
        bst: thread_rng().sample(bst_dist),
        base_stats: Stats::from(stats),
        alignment: thread_rng().sample(alignment_dist),
    }
}

fn log2(x: u32) -> u32 {
    if x > 0 { (x as f32).log(2.0) as u32 } else { 0 }
}

fn effort_value(character: &Character) -> u32 {
    character.species.bst * log2(character.species.bst) * character.attributes.level / log2(character.attributes.level + 1) / 31
}

static BASE_STAT_VALUE: u32 = 30;
static LEVEL_SCALING: u32 = 50;

fn gain_level(character: &mut Character) {
    character.attributes.level += 1;
    character.attributes.stats += scale_to(
        &character.species.base_stats.into(),
        character.species.bst / LEVEL_SCALING,
    ).into();
}

fn generate_character<D: Distribution<ActionId>>(
    name: String,
    species: Species,
    level: u32,
    action_dist: D,
) -> Character {
    let mut character = Character::from_species(species);
    character.name = name;
    character.attributes.level = level;
    character.attributes.stats = scale_to(
        &character.species.base_stats.into(),
        BASE_STAT_VALUE + level * character.species.bst / LEVEL_SCALING,
    ).into();
    character.attributes.actions = thread_rng().sample_iter(action_dist)
        .take(4)
        .collect::<Actions>();
    character.refresh();
    character
}

// public methods for generating/manipulating characters to be used in a battle
impl Distribution<Alignment> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Alignment {
        match rng.gen_range(0..4) {
            0usize => Alignment::A,
            1usize => Alignment::B,
            2usize => Alignment::C,
            _ => Alignment::D,
        }
    }
}

pub fn generate_player_character(actions: ActionId) -> Character {
    let species = generate_species(vec!["kaizo".to_string()], Standard, Uniform::new(450, 550), Standard);
    generate_character("player kaizo".to_string(), species, 1, Uniform::new(0, actions))
}

pub fn generate_enemy_character(actions: ActionId, level: u32) -> Character {
    let species = generate_species(vec!["kaizo".to_string()], Standard, Uniform::new(200, 700), Standard);
    let mut character = generate_character("enemy kaizo".to_string(), species, level, Uniform::new(0, actions));
    character.attributes.experience = effort_value(&character);
    character
}

// methods to generate the actions
pub fn generate_actions() -> Vec<Arc<dyn Action>> {
    vec![
        Arc::new(PureAttack { name: "Blast".to_string(), power: 40}),
        Arc::new(Attack { name: "Tackle".to_string(), power: 40, alignment: Alignment::A, priority: 0}),
        Arc::new(Attack { name: "Slam".to_string(), power: 70, alignment: Alignment::A, priority: 0}),
        Arc::new(Attack { name: "Bash".to_string(), power: 100, alignment: Alignment::A, priority: 0}),
        Arc::new(Attack { name: "Dash".to_string(), power: 30, alignment: Alignment::A, priority: 1}),
        Arc::new(Defend { name: "Block".to_string() }),
        Arc::new(Poison { name: "Venom".to_string(), power: 3 }),
        Arc::new(Sleep { name: "Hypnotize".to_string() }),
    ]
}

// battle logic
fn take_turn(user: &mut Character, target: &mut Character, action: Arc<dyn Action>) -> ActionLog {
    if user.state.status.contains_key(&Status::Sleep) {
        if random::<u32>() % (*user.state.status.get(&Status::Sleep).unwrap() as u32 + 1) == 0 {
            user.state.status.remove(&Status::Sleep);
            let mut logs = Vec::new();
            logs.push(format!("{} woke up.", user.name));
            logs.extend(action.act(user, target));
            logs
        } else {
            vec![format!("{} is sleeping.", user.name)]
        }
    } else if user.state.status.contains_key(&Status::Poison) {
        let mut logs = Vec::new();
        logs.extend(action.act(user, target));
        user.deal_damage(*user.state.status.get(&Status::Poison).unwrap());
        logs.push(format!("{} was hurt by poison.", user.name));
        logs
    } else {
        action.act(user, target)
    }
}

fn gain_experience(character: &mut Character, experience: u32) -> ActionLog {
    let mut logs = Vec::new();
    character.attributes.experience += experience;
    while character.attributes.experience >= 100 {
        character.attributes.experience -= 100;
        gain_level(character);
        logs.push(format!("{} rose to level {}!", character.name, character.attributes.level));
    }
    logs
}

#[derive(Clone)]
pub enum BattleState {
    Defeat,
    InProcess,
    Victory,
}

#[derive(Clone)]
pub struct Battle {
    pub player: Character,
    pub enemy: Character,
}

impl Battle {
    fn battle_state(&self) -> BattleState {
        if self.player.state.health == 0 {
            return BattleState::Defeat
        } else if self.enemy.state.health == 0 {
            return BattleState::Victory
        } else {
            return BattleState::InProcess
        }
    }

    fn clean_up(&mut self) {
        if self.player.state.status.contains_key(&Status::Defend) {
            self.player.state.status.remove(&Status::Defend);
        }
        if self.enemy.state.status.contains_key(&Status::Defend) {
            self.enemy.state.status.remove(&Status::Defend);
        }
    }

    pub fn player_turn(&mut self, action: Arc<dyn Action>) -> ActionLog {
        let state = self.battle_state();
        if let BattleState::InProcess = state {
            take_turn(&mut self.player, &mut self.enemy, action)
        } else { vec![] }
    }

    pub fn enemy_turn(&mut self, action: Arc<dyn Action>) -> ActionLog {
        let state = self.battle_state();
        if let BattleState::InProcess = state {
            take_turn(&mut self.enemy, &mut self.player, action)
        } else { vec![] }
    }

    pub fn end_turn(&mut self) -> (BattleState, ActionLog) {
        let mut logs = Vec::new();
        let state = match self.battle_state() {
            BattleState::Victory => {
                // award xp
                let experience: u32 = self.enemy.attributes.experience / self.player.attributes.level;
                logs.push(format!("defeated {}!", self.enemy.name));
                logs.push(format!("{} gained {} experience!", self.player.name, experience));
                logs.extend(gain_experience(&mut self.player, experience));
                BattleState::Victory
            },
            BattleState::Defeat => {
                logs.push(format!("{} died!", self.player.name));
                BattleState::Defeat
            },
            _ => {
                self.clean_up();
                BattleState::InProcess
            }
        };
        (state, logs)
    }
}
