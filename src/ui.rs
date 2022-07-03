// ui components
// TODO: these are kind of a stand-in. we probably want something better eventually
use yew::prelude::*;

use yew::html;
use yew::html::Properties;

use crate::kaizo::Character;

#[derive(Properties, PartialEq)]
pub struct CharacterProps { pub character: Character }

#[function_component(CharacterOverview)]
pub fn character_overview(CharacterProps { character} : &CharacterProps) -> Html {
    html! {
        format!(
            "{} {:?} (BST: {:}) Lv{} {:?}",
            character.name,
            character.state.status.keys(),
            character.species.bst,
            character.attributes.level,
            character.species.alignment
        )
    }
}

#[function_component(CharacterStats)]
pub fn character_stats(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        format!(
            "ATK:{} DFN:{} SPD:{}",
            character.attributes.stats.attack,
            character.attributes.stats.defense,
            character.attributes.stats.speed,
        )
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
            data-label={ format!("HP:{: >n$}/{}", character.state.health, character.attributes.stats.health) }/>
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
            data-label={ format!("EXP:{: >3}/{}", character.attributes.experience, 100) }></progress>
        </div>
    }
}

#[function_component(PlayerDisplay)]
pub fn player_display(CharacterProps { character } : &CharacterProps) -> Html {
    html! {
        <div>
            <div class="columns">
                <div class="character-display">
                    <div><img src="resources/player.png" style="position: relative;"/></div>
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
                    <div><img src="resources/enemy.png" style="position: relative;"/></div>
                </div>
            </div>
        </div>
    }
}

// #[function_component(ActionControls)]
// pub fn action_controls(CharacterProps { character } : &CharacterProps) -> Html {
//     html! {
//         <div> {
//             for character.attributes.actions.iter().map(|action| {
//                 let action_id = action.clone();
//                 let action = self.actions.get(action_id).unwrap().name();
//                 let callback = match self.scene {
//                     Scene::Battle(_) => ctx.link().callback(move |_| Msg::BattleAction(BattleAction::ActionChosen(action_id))),
//                     Scene::Menu(_) => ctx.link().callback(
//                         move |_| Msg::MenuAction(MenuAction::Log(format!("{}", action)))),
//                 };
//                 html! {
//                     <button
//                         class="action-button"
//                         title={ self.actions.get(action_id).unwrap().description() }
//                         onclick={ callback }
//                     > {
//                         format!("{}", self.actions.get(action_id).unwrap().name())
//                     } </button>
//                 }
//             })
//         } </div>
//     }
// }
