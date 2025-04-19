//! An opinionated global state machine for `bevy_picking`.
//!
//! # Rules
//!
//! * One action at a time
//!
//!     Only one entity can be "active", i.e. hovered or pressed.
//!
//!     There is no multi-cursor support.
//!
//! * Single button only
//!
//!     The state only tracks one button.
//!     Pressing multiple buttons is treated as canceling the current click or drag.
//!     This state persists until all buttons are released.
//!
//!     If cancellation happens after hovering, you can customize the behavior to
//!     either stop hovering immediately or maintain the hovering state.
//!
//! * Clean interactions
//!
//!     If any registered button is already pressed, no new entities can be registered as hovered or pressed.

use core::f32;
use std::cmp::Reverse;
mod local;
mod transitions;
pub use local::ButtonFilter;

use bevy::{
    app::{Plugin, PreUpdate},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{In, IntoSystem, Query, Res, ResMut},
    },
    input::{ButtonInput, mouse::MouseButton},
    math::Vec2,
    picking::{PickSet, backend::PointerHits},
    time::{Time, Virtual},
    window::{PrimaryWindow, Window},
};
pub use transitions::{PickingTransition, PickingTransitions};

/// Plugin for [`PickingStateMachine`].
#[derive(Debug, Clone, Resource)]
pub struct PickingStateMachinePlugin {
    /// Only buttons in this list will be considered.
    pub allowed_buttons: Vec<MouseButton>,
    /// If true, pressing multiple buttons will immediately cancel `Hover` to `None`.
    pub cancel_hover: bool,
}

impl Default for PickingStateMachinePlugin {
    fn default() -> Self {
        Self {
            allowed_buttons: vec![MouseButton::Left],
            cancel_hover: false,
        }
    }
}

impl Plugin for PickingStateMachinePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.insert_resource(self.clone());
        app.init_resource::<PickingStateMachine>();
        app.add_systems(
            PreUpdate,
            picking_window_system
                .pipe(picking_button_system)
                .pipe(picking_state_machine_system)
                .in_set(PickSet::Hover),
        );
    }
}

/// Picking state of an entity.
#[derive(Debug, Clone, Copy, Default)]
pub enum EntityPickingState {
    #[default]
    None,
    Hover,
    Pressed,
}

/// Picking state globally.
#[derive(Debug, Clone, Copy, Default)]
pub enum GlobalPickingState {
    #[default]
    None,
    Hover {
        entity: Entity,
    },
    Pressed {
        entity: Entity,
    },
}

impl GlobalPickingState {
    pub fn current_entity(&self) -> Option<Entity> {
        match self {
            GlobalPickingState::None => None,
            GlobalPickingState::Hover { entity } => Some(*entity),
            GlobalPickingState::Pressed { entity } => Some(*entity),
        }
    }
}

/// State for a button press.
#[derive(Debug, Clone, Copy)]
pub struct PressState {
    pub button: MouseButton,
    pub position: Vec2,
    pub time: f32,
}

/// Global state machine for `bevy_picking`.
#[derive(Debug, Clone, Default, Resource)]
pub struct PickingStateMachine {
    /// State of the previous frame.
    pub previous: GlobalPickingState,
    /// State of the current frame.
    pub current: GlobalPickingState,
    /// Pointer position.
    pub pointer: Vec2,
    /// If mouse is pressed, contains position, button and time of the button press.
    ///
    /// # Note
    ///
    /// This will not be present on button release, use `transitions` instead.
    pub press: Option<PressState>,
    /// If true, current button is just pressed.
    pub current_btn_just_pressed: bool,
    /// If true, [`PickingStateMachine::pointer`]
    /// is not retrieved from the current frame.
    pub pointer_is_out_of_bounds: bool,
    /// True if multiple valid buttons are pressed as the same time.
    /// Lasts until all valid buttons are released.
    pub is_post_cancellation_state: bool,
    /// An internal event channel for picking events.
    ///
    /// Use `as_ref` or `iter` to access items.
    pub transitions: PickingTransitions,
}

impl PickingStateMachine {
    pub fn get_state(&self, entity: Entity) -> EntityPickingState {
        match self.current {
            GlobalPickingState::None => EntityPickingState::None,
            GlobalPickingState::Hover { entity: e } => {
                if entity == e {
                    EntityPickingState::Hover
                } else {
                    EntityPickingState::None
                }
            }
            GlobalPickingState::Pressed { entity: e } => {
                if entity == e {
                    EntityPickingState::Hover
                } else {
                    EntityPickingState::None
                }
            }
        }
    }

    pub fn get_transition(&self, entity: Entity) -> Option<PickingTransition> {
        self.transitions.iter().find(|x| x.entity() == entity)
    }

    pub fn get_active_entity(&self) -> Option<Entity> {
        self.current.current_entity()
    }

    pub fn is_hovering(&self) -> bool {
        matches!(self.current, GlobalPickingState::Hover { .. })
    }

    pub fn is_pressing(&self) -> bool {
        matches!(self.current, GlobalPickingState::Pressed { .. })
    }

    /// We allow acquiring new target if
    /// * Not post-cancellation state.
    /// * Not pressed.
    /// * Just pressed with no current entity.
    fn can_acquire_new_target(&self) -> bool {
        !self.is_post_cancellation_state
            && (self.press.is_none() || self.current_btn_just_pressed)
    }
}

fn picking_window_system(
    mut state_machine: ResMut<PickingStateMachine>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let mouse_position = match window.single() {
        Ok(window) => window.cursor_position(),
        Err(_) => None,
    };
    match mouse_position {
        Some(position) => {
            state_machine.pointer = position;
            state_machine.pointer_is_out_of_bounds = false;
        }
        None => {
            state_machine.pointer_is_out_of_bounds = true;
        }
    }
}

fn picking_button_system(
    time: Res<Time<Virtual>>,
    mut state_machine: ResMut<PickingStateMachine>,
    settings: Res<PickingStateMachinePlugin>,
    input: Res<ButtonInput<MouseButton>>,
) -> bool {
    let mut current_button = None;
    let mut cancel = false;
    let mut just_pressed = false;
    let time = time.elapsed_secs();
    for button in &settings.allowed_buttons {
        if input.pressed(*button) {
            if input.just_pressed(*button) {
                just_pressed = true;
            }
            if current_button.is_none() {
                current_button = Some(*button)
            } else {
                current_button = None;
                cancel = true;
                break;
            }
        }
    }
    // To make state transitions less weird,
    // if you release one button and press another in the same frame,
    // treat it as entering cancellation state,
    // this ensures one event per frame.
    if let Some(press) = state_machine.press {
        if current_button.is_some_and(|b| b != press.button) {
            cancel = true;
        }
    }
    state_machine.current_btn_just_pressed = false;
    if cancel {
        state_machine.is_post_cancellation_state = true;
    } else if state_machine.is_post_cancellation_state && current_button.is_none() {
        state_machine.is_post_cancellation_state = false;
    } else if just_pressed {
        state_machine.current_btn_just_pressed = true;
    }
    // We need to keep this for events so deletion is delayed.
    if let Some(button) = current_button {
        state_machine.press = Some(PressState {
            button,
            position: state_machine.pointer,
            time,
        });
    }
    current_button.is_some()
}

fn picking_state_machine_system(
    pressed: In<bool>,
    time: Res<Time<Virtual>>,
    settings: Res<PickingStateMachinePlugin>,
    mut pick: EventReader<PointerHits>,
    mut state_machine: ResMut<PickingStateMachine>,
    filters: Query<&ButtonFilter>,
) {
    let pressed = *pressed;
    let time = time.elapsed_secs();
    let mut min = (f32::NEG_INFINITY, Reverse(f32::INFINITY));
    let mut target = None;
    let current = match state_machine.current {
        GlobalPickingState::None => None,
        GlobalPickingState::Hover { .. } => None,
        GlobalPickingState::Pressed { entity } => Some(entity),
    };
    let can_acquire = state_machine.can_acquire_new_target();
    'main: for hits in pick.read() {
        for (entity, hit) in &hits.picks {
            if Some(*entity) == current {
                target = current;
                break 'main;
            }
            if !can_acquire {
                continue;
            }
            let priority = (hits.order, Reverse(hit.depth));
            if priority > min {
                min = priority;
                target = Some(*entity);
            }
        }
    }
    state_machine.previous = state_machine.current;
    match target {
        None => {
            if pressed && !state_machine.current_btn_just_pressed {
                match state_machine.current {
                    GlobalPickingState::Pressed { .. } => (),
                    _ => state_machine.current = GlobalPickingState::None,
                }
            } else {
                state_machine.current = GlobalPickingState::None;
            }
        }
        Some(entity) if state_machine.is_post_cancellation_state => {
            match state_machine.current {
                // If hovering, maintain it, otherwise cancel to base state.
                GlobalPickingState::Hover { entity: e }
                    if e == entity && !settings.cancel_hover =>
                {
                    state_machine.current = GlobalPickingState::Hover { entity };
                }
                _ => {
                    state_machine.current = GlobalPickingState::None;
                }
            }
        }
        Some(entity) if !pressed => state_machine.current = GlobalPickingState::Hover { entity },
        Some(entity) => {
            let filter = if let Ok(filter) = filters.get(entity) {
                filter.contains(state_machine.press.unwrap().button)
            } else {
                true
            };
            if filter {
                state_machine.current = GlobalPickingState::Pressed { entity }
            } else {
                state_machine.current = GlobalPickingState::Hover { entity }
            }
        }
    }
    state_machine.queue_transitions(time);
    if !pressed {
        state_machine.press = None;
    }
}
