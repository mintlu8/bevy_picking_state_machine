use std::{iter::Copied, ops::Deref, slice::Iter};

use bevy::{ecs::entity::Entity, input::mouse::MouseButton, math::Vec2};

use crate::{GlobalPickingState, PickingStateMachine};

/// A channel for picking transitions.
#[derive(Debug, Clone, Copy, Default)]
pub enum PickingTransitions {
    #[default]
    None,
    One(PickingTransition),
    FromTo([PickingTransition; 2]),
}

impl AsRef<[PickingTransition]> for PickingTransitions {
    fn as_ref(&self) -> &[PickingTransition] {
        self
    }
}

impl Deref for PickingTransitions {
    type Target = [PickingTransition];

    fn deref(&self) -> &Self::Target {
        match self {
            PickingTransitions::None => &[],
            PickingTransitions::One(item) => core::array::from_ref(item),
            PickingTransitions::FromTo(arr) => arr,
        }
    }
}

impl PickingTransitions {
    pub fn iter(&self) -> Copied<Iter<PickingTransition>> {
        self.into_iter()
    }
}

impl<'t> IntoIterator for &'t PickingTransitions {
    type Item = PickingTransition;

    type IntoIter = Copied<Iter<'t, PickingTransition>>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter().copied()
    }
}

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
        use PickingTransitions::{FromTo, One};
        self.transitions = PickingTransitions::None;
        let time = self.press.map(|x| now - x.time).unwrap_or(0.0);
        let button = self.press.map(|x| x.button).unwrap_or(MouseButton::Left);
        let down = self.press.map(|x| x.position).unwrap_or(Vec2::ZERO);
        match (self.previous, self.current) {
            (None, None) => (),
            (None, Hover { entity }) => {
                self.transitions = One(PickingTransition::HoverEnter { entity })
            }
            (None, Pressed { entity }) => {
                self.transitions = One(PickingTransition::Pressed { entity, button })
            }
            (Hover { entity }, None) => {
                self.transitions = One(PickingTransition::HoverExit { entity })
            }
            (Hover { entity: e1 }, Hover { entity: e2 }) => {
                if e1 != e2 {
                    self.transitions = FromTo([
                        PickingTransition::HoverExit { entity: e1 },
                        PickingTransition::HoverEnter { entity: e2 },
                    ]);
                }
            }
            (Hover { entity: e1 }, Pressed { entity: e2 }) => {
                if e1 == e2 {
                    self.transitions = One(PickingTransition::Pressed { entity: e1, button });
                } else {
                    self.transitions = FromTo([
                        PickingTransition::HoverExit { entity: e1 },
                        PickingTransition::Pressed { entity: e2, button },
                    ]);
                }
            }
            (Pressed { entity }, None) => {
                if self.is_post_cancellation_state {
                    self.transitions = One(PickingTransition::Cancelled {
                        entity,
                        down,
                        time,
                        button,
                    });
                } else {
                    self.transitions = One(PickingTransition::Released {
                        entity,
                        button,
                        down,
                        time,
                        outside: true,
                    });
                }
            }
            (Pressed { entity: e1 }, Hover { entity: e2 }) => {
                if e1 == e2 {
                    self.transitions = One(PickingTransition::Released {
                        entity: e1,
                        button,
                        down,
                        time,
                        outside: false,
                    });
                } else {
                    self.transitions = FromTo([
                        PickingTransition::Released {
                            entity: e1,
                            button,
                            down,
                            time,
                            outside: true,
                        },
                        PickingTransition::HoverEnter { entity: e2 },
                    ]);
                }
            }
            (Pressed { entity: e1 }, Pressed { entity: e2 }) => {
                // Both of these situations should be forbidden, but just in case.
                if e1 != e2 || self.current_btn_just_pressed {
                    self.transitions = FromTo([
                        PickingTransition::Released {
                            entity: e1,
                            button,
                            down,
                            time,
                            outside: true,
                        },
                        PickingTransition::Pressed { entity: e2, button },
                    ]);
                }
            }
        }
    }
}
