use yew::prelude::*;

use yew::html;
use yew::html::Properties;

use rand::{random, thread_rng};
use rand::distributions::{Distribution, Standard};

use kaizo_quest::core::ActionId;
use kaizo_quest::onion::{EXPERIENCE_TO_LEVEL, Experience, OnionBattle, OnionBattleState, OnionCharacter, OnionWorld, Scale};

static RESOURCES: &str = "resources";

fn get_resource(resource: &str) -> String {
    format!("{}/{}.png", RESOURCES, resource)
}

#[derive(Properties, PartialEq)]
pub struct CharacterProps { pub character: OnionCharacter }

#[function_component(CharacterOverview)]
pub fn character_overview(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <p style="text-align:left;">
                <img title={
                    format!("{:?}", character.species.alignment)
                } style="alignment:left;" src={ get_resource(&format!("{:?}", character.species.alignment)).to_lowercase() }
                width={"5%"} height={"5%"}/>
                { format!(" {} (BST: {}) Lv{} ", character.name.clone(), character.species.bst, character.attributes.level) }
                // { format!(" {} ", character.name.clone()) }
                { for character.state.status.keys().map(|status|
                    html! {
                        <img title={
                            format!("{:?}", status)
                        } style="alignment:left;" src={ get_resource(&format!("{:?}", status).to_lowercase()) }
                        width={"5%"} height={"5%"}/>
                    })
                }
            </p>
        </div>
    }
}

#[function_component(CharacterStats)]
pub fn character_stats(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <img title={
                format!("Attack determines damage dealt.")
            } src={ get_resource("attack") } width={"15%"} height={"15%"}/>
            { format!("{}", character.attributes.stats.attack) }
            { " " }
            <img title={
                format!("Defense determines damage taken.")
            } src={ get_resource("defense") } width={"15%"} height={"15%"}/>
            { format!("{}", character.attributes.stats.defense) }
            { " " }
            <img title={
                format!("Speed determines turn order.")
            } src={ get_resource("speed") } width={"15%"} height={"15%"}/>
            { format!("{}", character.attributes.stats.speed) }
        </div>
    }
}

#[function_component(HealthBar)]
pub fn health_bar(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <div><CharacterOverview character={character.clone()}/></div>
            <progress id="health" value={
                format!("{}", character.state.health)
            } max={
                format!("{}", character.attributes.stats.health)
            }/>
        </div>
    }
}

#[function_component(HealthBarWithValue)]
pub fn health_bar_with_value(CharacterProps { character } : &CharacterProps) -> Html {
    let n = character.attributes.stats.health.to_string().len();
    html! {
        <div>
            <div><CharacterOverview character={character.clone()}/></div>
            <progress id="health" value={
                format!("{}", character.state.health)
            } max={
                format!("{}", character.attributes.stats.health)
            }
            data-label={ format!("HP:{: >n$}/{}", character.state.health, character.attributes.stats.health) }
            title={ format!("{} will die if their health reaches 0.", character.name) }/>
        </div>
    }
}

#[function_component(ExperienceBar)]
pub fn experience_bar(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <progress id="experience" value={
                format!("{}", character.attributes.experience)
            } max={"100"}
            data-label={ format!("EXP:{: >3}/{}", character.attributes.experience, EXPERIENCE_TO_LEVEL) }
            title={ format!(
                "{} will gain a level after gaining {} experience.",
                character.name,
                EXPERIENCE_TO_LEVEL - character.attributes.experience
            )} ></progress>
        </div>
    }
}

#[function_component(PlayerDisplay)]
pub fn player_display(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <div class="columns">
                <div class="character-display">
                    <div><img src={ get_resource("player") } style="position: relative;"/></div>
                    <div><CharacterStats character={character.clone()} /></div>
                </div>
                <div class="character-info">
                    <div><HealthBarWithValue character={character.clone()} /></div>
                    <div><ExperienceBar character={character.clone()} /></div>
                </div>
            </div>
        </div>
    }
}

#[function_component(EnemyDisplay)]
pub fn enemy_display(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <div class="columns">
                <div class="character-info">
                    <div><HealthBarWithValue character={character.clone()} /></div>
                </div>
                <div class="character-display">
                    <div><img src={ get_resource("enemy") } style="position: relative;"/></div>
                </div>
            </div>
        </div>
    }
}

// TODO: all these helper enums need to be broken up
enum Scene {
    Battle(OnionBattle),
    Menu(OnionCharacter),
}

enum BattleAction {
    ActionChosen(ActionId),
    Flee,
}

enum MenuAction {
    Log(String),
    Battle,
    Scout,
}

enum Msg {
    BattleAction(BattleAction),
    MenuAction(MenuAction),
}

struct App {
    world: OnionWorld,
    scene: Scene,
    logs: Vec<String>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        let world: OnionWorld = Standard.sample(&mut thread_rng());
        let mut character: OnionCharacter = world.sample(&mut thread_rng());
        character.gain_experience(EXPERIENCE_TO_LEVEL);
        character.attributes.stats = character.species.stats.scale(EXPERIENCE_TO_LEVEL);
        character.refresh();
        Self {
            scene: Scene::Menu(character),
            world,
            logs: Vec::new(),
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        self.logs.clear();
        match (msg, &mut self.scene) {
            (Msg::BattleAction(action), Scene::Battle(battle)) => {
                // get player action
                let player_action = match action {
                    BattleAction::ActionChosen(action) => &self.world.actions[action],
                    BattleAction::Flee => {
                        battle.player.refresh();
                        self.scene = Scene::Menu(battle.player.clone());
                        return true;
                    }
                };
                // get enemy action
                let enemy_action = battle.enemy.attributes.actions.get(random::<usize>() % battle.enemy.attributes.actions.len()).copied().unwrap();
                let enemy_action = &self.world.actions[enemy_action];

                // determine action order:
                //  - highest priority wins
                //  - if a priority tie, highest speed wins
                //  - if a speed tie, flip a coin
                let player_first = if player_action.priority() > enemy_action.priority() {
                    true
                } else if player_action.priority() == enemy_action.priority() &&
                    (battle.player.priority() > battle.enemy.priority() ||
                        (battle.player.priority() == battle.enemy.priority() && random::<bool>())) {
                    true
                } else {
                    false
                };

                if player_first {
                    self.logs.extend(battle.player_turn(player_action));
                    self.logs.extend(battle.enemy_turn(enemy_action));
                } else {
                    self.logs.extend(battle.enemy_turn(enemy_action));
                    self.logs.extend(battle.player_turn(player_action));
                }

                match battle.end_turn() {
                    (OnionBattleState::Victory, logs) => {
                        // award xp
                        self.logs.extend(logs);
                        // TODO: have to chose if the battle is over or if we are still going
                        // TODO: if we learned moves, it needs to happen here
                        battle.player.refresh();
                        // TODO: if we add evos, it should happen before this
                        self.scene = Scene::Menu(battle.player.clone());
                    },
                    (OnionBattleState::Defeat, logs) => {
                        self.logs.extend(logs);
                        // re-roll player kaizo
                        let mut character = self.world.sample(&mut thread_rng());
                        character.gain_experience(EXPERIENCE_TO_LEVEL);
                        character.attributes.stats = character.species.stats.scale(EXPERIENCE_TO_LEVEL);
                        character.refresh();
                        self.scene = Scene::Menu(character);
                    },
                    _ => ()
                }
            }
            (Msg::MenuAction(action), Scene::Menu(player)) => match action {
                MenuAction::Battle => {
                    // TODO: we need to think in terms of generating a whole sequence of battles
                    let player = player.clone();
                    let enemy = self.world.sample_at_level(player.attributes.level, &mut thread_rng());
                    self.logs.push(format!("{} appeared!", enemy.name));
                    self.scene = Scene::Battle(OnionBattle { player, enemy });
                },
                MenuAction::Log(log) => self.logs.push(log),
                MenuAction::Scout => (),
            },
            _ => (),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let player = match &self.scene {
            Scene::Battle(battle) => battle.player.clone(),
            Scene::Menu(player) => player.clone(),
        };
        // TODO: i don't know enough html/css/etc to know how to decouple this well; the ui
        //       probably will be redesigned eventually anyways...
        html! {
            <div>
                <div>{ "Kaizo Quest" }</div>
                <div class="columns">
                    <div class="game-area">
                        <div> {
                            match &self.scene {
                                Scene::Battle(battle) => html! {
                                    <div>
                                        // enemy details
                                        <div><EnemyDisplay character={battle.enemy.clone()} /></div>
                                    </div>
                                },
                                Scene::Menu(_) => html! { },
                            }
                        } </div>
                        // player details
                        <div><PlayerDisplay character={ player.clone() } /></div>
                        // player controls
                        <div>
                            // action controls
                            <div> {
                                for player.attributes.actions.iter().map(|action| {
                                    let action_id = action.clone();
                                    let action = self.world.actions[action_id].name();
                                    let callback = match self.scene {
                                        Scene::Battle(_) => ctx.link().callback(move |_| Msg::BattleAction(BattleAction::ActionChosen(action_id))),
                                        Scene::Menu(_) => ctx.link().callback(
                                            move |_| Msg::MenuAction(MenuAction::Log(format!("{}", action)))),
                                    };
                                    html! {
                                        <button
                                            class="action-button"
                                            title={ self.world.actions[action_id].description() }
                                            onclick={ callback }
                                        > {
                                            format!("{}", self.world.actions[action_id].name())
                                        } </button>
                                    }
                                })
                            } </div>
                            // scene controls
                            <div> {
                                match &self.scene {
                                    Scene::Battle(_) => html! {
                                        <button class="control-button" onclick={ctx.link().callback(move |_| Msg::BattleAction(BattleAction::Flee))} title="Escape from this battle and return to the menu">{
                                            "Flee"
                                        } </button>
                                    },
                                    Scene::Menu(_) => html! {
                                        <div>
                                            <button class="control-button" onclick={ctx.link().callback(move |_| Msg::MenuAction(MenuAction::Battle))} title="Battle the next kaizo master.">{
                                                "Battle"
                                            }</button>
                                            <button class="control-button" onclick={ctx.link().callback(move |_| Msg::MenuAction(MenuAction::Scout))} title="Search for a new kaizo.">{
                                                "Scout"
                                            }</button>
                                        </div>
                                    },
                                }
                            } </div>
                        </div>
                    </div>
                    <div class="logs">
                    { for self.logs.iter().map(move |log| { html! { <div>{ format!("{}", log) }</div> } }) }
                    </div>
                </div>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<App>();
}
