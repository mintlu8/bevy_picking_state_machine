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
        system::{Query, Res, ResMut},
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
        app.add_systems(
            PreUpdate,
            picking_state_machine_system.in_set(PickSet::Hover),
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
}

pub fn picking_state_machine_system(
    time: Res<Time<Virtual>>,
    settings: Res<PickingStateMachinePlugin>,
    mut pick: EventReader<PointerHits>,
    mut state_machine: ResMut<PickingStateMachine>,
    filters: Query<&ButtonFilter>,
    input: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let time = time.elapsed_secs();
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
    let mut current_button = None;
    let mut cancel = false;
    for button in &settings.allowed_buttons {
        if input.pressed(*button) {
            if current_button.is_none() {
                current_button = Some(*button)
            } else {
                cancel = true;
                break;
            }
        }
    }
    if cancel {
        state_machine.is_post_cancellation_state = true;
    } else if state_machine.is_post_cancellation_state && current_button.is_none() {
        state_machine.is_post_cancellation_state = false;
    }
    let button_changed = match (state_machine.press, current_button) {
        (Some(press), Some(button)) => {
            if press.button == button {
                false
            } else {
                state_machine.press = Some(PressState {
                    button,
                    position: state_machine.pointer,
                    time,
                });
                true
            }
        }
        (Some(_), None) => true,
        (None, Some(button)) => {
            state_machine.press = Some(PressState {
                button,
                position: state_machine.pointer,
                time,
            });
            true
        }
        (None, None) => false,
    };
    let mut min = (f32::NEG_INFINITY, Reverse(f32::INFINITY));
    let mut target = None;
    let current = state_machine.current.current_entity();
    // If pressed, lock in.
    let can_acquire_new_current =
        current_button.is_none() || state_machine.is_post_cancellation_state;
    'main: for hits in pick.read() {
        for (entity, hit) in &hits.picks {
            if Some(*entity) == current {
                target = current;
                break 'main;
            }
            if !can_acquire_new_current {
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
            if current_button.is_some() && !button_changed {
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
        Some(entity) if current_button.is_none() => {
            state_machine.current = GlobalPickingState::Hover { entity }
        }
        Some(entity) => {
            let filter = if let Ok(filter) = filters.get(entity) {
                filter.contains(current_button.unwrap())
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
    if current_button.is_none() {
        state_machine.press = None;
    }
}
