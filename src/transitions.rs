use crate::{GlobalPickingState, PickingStateMachine};
use bevy::{ecs::entity::Entity, input::mouse::MouseButton, math::Vec2};

/// A picking transition event.
#[derive(Debug, Clone, Copy)]
pub enum PickingTransition {
    Pressed {
        entity: Entity,
        button: MouseButton,
    },
    Released {
        entity: Entity,
        button: MouseButton,
        down: Vec2,
        time: f32,
        outside: bool,
    },
    HoverEnter {
        entity: Entity,
    },
    HoverExit {
        entity: Entity,
    },
    Cancelled {
        entity: Entity,
        button: MouseButton,
        down: Vec2,
        time: f32,
    },
}

impl PickingTransition {
    pub fn entity(&self) -> Entity {
        match *self {
            PickingTransition::Pressed { entity, .. } => entity,
            PickingTransition::Released { entity, .. } => entity,
            PickingTransition::HoverEnter { entity } => entity,
            PickingTransition::HoverExit { entity } => entity,
            PickingTransition::Cancelled { entity, .. } => entity,
        }
    }
}

impl PickingStateMachine {
    pub(crate) fn queue_transitions(&mut self, now: f32) {
        use GlobalPickingState::*;
        self.transitions.clear();
        let time = self.press.map(|x| now - x.time).unwrap_or(0.0);
        let button = self.press.map(|x| x.button).unwrap_or(MouseButton::Left);
        let down = self.press.map(|x| x.position).unwrap_or(Vec2::ZERO);
        match (self.previous, self.current) {
            (None, None) => (),
            (None, Hover { entity }) => {
                self.transitions
                    .push(PickingTransition::HoverEnter { entity });
            }
            (None, Pressed { entity }) => {
                self.transitions
                    .push(PickingTransition::Pressed { entity, button });
            }
            (Hover { entity }, None) => {
                self.transitions
                    .push(PickingTransition::HoverExit { entity });
            }
            (Hover { entity: e1 }, Hover { entity: e2 }) => {
                if e1 != e2 {
                    self.transitions
                        .push(PickingTransition::HoverExit { entity: e1 });
                    self.transitions
                        .push(PickingTransition::HoverEnter { entity: e2 });
                }
            }
            (Hover { entity: e1 }, Pressed { entity: e2 }) => {
                if e1 == e2 {
                    self.transitions
                        .push(PickingTransition::Pressed { entity: e1, button });
                } else {
                    self.transitions
                        .push(PickingTransition::HoverExit { entity: e1 });
                    self.transitions
                        .push(PickingTransition::HoverEnter { entity: e2 });
                    self.transitions
                        .push(PickingTransition::Pressed { entity: e2, button });
                }
            }
            (Pressed { entity }, None) => {
                if self.is_post_cancellation_state {
                    self.transitions.push(PickingTransition::Cancelled {
                        entity,
                        down,
                        time,
                        button,
                    });
                } else {
                    self.transitions.push(PickingTransition::Released {
                        entity,
                        button,
                        down,
                        time,
                        outside: true,
                    });
                    self.transitions
                        .push(PickingTransition::HoverExit { entity });
                }
            }
            (Pressed { entity: e1 }, Hover { entity: e2 }) => {
                if e1 == e2 {
                    self.transitions.push(PickingTransition::Released {
                        entity: e1,
                        button,
                        down,
                        time,
                        outside: false,
                    });
                } else {
                    self.transitions.push(PickingTransition::Released {
                        entity: e1,
                        button,
                        down,
                        time,
                        outside: true,
                    });
                    self.transitions
                        .push(PickingTransition::HoverExit { entity: e1 });
                    self.transitions
                        .push(PickingTransition::HoverEnter { entity: e2 });
                }
            }
            (Pressed { entity: e1 }, Pressed { entity: e2 }) => {
                // Both of these situations should be forbidden, but just in case.
                if e1 != e2 || self.current_btn_just_pressed {
                    self.transitions.push(PickingTransition::Released {
                        entity: e1,
                        button,
                        down,
                        time,
                        outside: true,
                    });
                    self.transitions
                        .push(PickingTransition::HoverExit { entity: e1 });
                    self.transitions
                        .push(PickingTransition::HoverEnter { entity: e2 });
                    self.transitions
                        .push(PickingTransition::Pressed { entity: e2, button });
                }
            }
        }
    }
}
