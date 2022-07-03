use std::sync::Arc;
use std::vec::Vec;

use rand::random;
use yew::{html, Component, Context, Html};

use kaizo_quest::kaizo::{Action, ActionId, Character};
use kaizo_quest::onion::{Battle, BattleState, generate_player_character, generate_enemy_character, generate_actions};
use kaizo_quest::ui::{PlayerDisplay, EnemyDisplay};

// TODO: all these helper enums need to be broken up
enum Scene {
    Battle(Battle),
    Menu(Character),
}

enum BattleAction {
    ActionChosen(ActionId),
    RunAway,
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
    actions: Vec<Arc<dyn Action>>,
    scene: Scene,
    logs: Vec<String>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        let actions = generate_actions();
        Self {
            scene: Scene::Menu(generate_player_character(actions.len())),
            actions,
            logs: Vec::new(),
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        self.logs.clear();
        match (msg, &mut self.scene) {
            (Msg::BattleAction(action), Scene::Battle(battle)) => {
                // get player action
                let player_action = match action {
                    BattleAction::ActionChosen(action) => self.actions.get(action).unwrap().clone(),
                    BattleAction::RunAway => {
                        battle.player.refresh();
                        self.scene = Scene::Menu(battle.player.clone());
                        return true;
                    }
                };
                // get enemy action
                let enemy_action = battle.enemy.attributes.actions.get(random::<usize>() % battle.enemy.attributes.actions.len()).unwrap();
                let enemy_action = self.actions.get(*enemy_action).unwrap().clone();

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
                    self.logs.extend(battle.player_turn(player_action.clone()));
                    self.logs.extend(battle.enemy_turn(enemy_action.clone()));
                } else {
                    self.logs.extend(battle.enemy_turn(enemy_action.clone()));
                    self.logs.extend(battle.player_turn(player_action.clone()));
                }

                match battle.end_turn() {
                    (BattleState::Victory, logs) => {
                        // award xp
                        self.logs.extend(logs);
                        // TODO: have to chose if the battle is over or if we are still going
                        battle.player.refresh();
                        // TODO: if we add evos, it should happen before this
                        self.scene = Scene::Menu(battle.player.clone());
                    },
                    (BattleState::Defeat, logs) => {
                        self.logs.extend(logs);
                        // re-roll player kaizo
                        self.scene = Scene::Menu(generate_player_character(self.actions.len()));
                    },
                    _ => ()
                }
            }
            (Msg::MenuAction(action), Scene::Menu(player)) => match action {
                MenuAction::Battle => {
                    // TODO: we need to think in terms of generating a whole sequence of battles
                    let enemy = generate_enemy_character(self.actions.len(), player.attributes.level);
                    self.logs.push(format!("{} appeared!", enemy.name));
                    self.scene = Scene::Battle(Battle { player: player.clone(), enemy });
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
                <div>{ "kaizo quest" }</div>
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
                                    let action = self.actions.get(action_id).unwrap().name();
                                    let callback = match self.scene {
                                        Scene::Battle(_) => ctx.link().callback(move |_| Msg::BattleAction(BattleAction::ActionChosen(action_id))),
                                        Scene::Menu(_) => ctx.link().callback(
                                            move |_| Msg::MenuAction(MenuAction::Log(format!("{}", action)))),
                                    };
                                    html! {
                                        <button
                                            class="action-button"
                                            title={ self.actions.get(action_id).unwrap().description() }
                                            onclick={ callback }
                                        > {
                                            format!("{}", self.actions.get(action_id).unwrap().name())
                                        } </button>
                                    }
                                })
                            } </div>
                            // scene controls
                            <div> {
                                match &self.scene {
                                    Scene::Battle(_) => html! {
                                        <button class="control-button" onclick={ctx.link().callback(move |_| Msg::BattleAction(BattleAction::RunAway))}>{
                                            "run away"
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
