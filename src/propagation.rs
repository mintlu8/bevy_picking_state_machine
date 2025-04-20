use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    system::{Query, Res, SystemParam},
};

use crate::{EntityPickingState, PickingStateMachine, PickingTransition};

/// Determines what additional entities count as active by [`PropagatedPickingStateMachine`].
///
/// This component does not affect anything else defined in the crate.
#[derive(Debug, Clone, Copy, Component, Default)]
pub enum PickingPropagation {
    /// Same behavior as not having this component, propagate to descendants.
    #[default]
    PropagateDown,
    /// Propagate to `x` parent entities and all their descendants.
    PropagateUp(usize),
    /// Propagate to `x` parent entities, and **this** entity's descendants.
    AndPropagateUp(usize),
    /// Don't propagate to parents or descendants.
    NoPropagation,
}

/// [`SystemParam`] that evaluates active entities through hierarchical propagation.
#[derive(Debug, SystemParam)]
pub struct PropagatedPickingStateMachine<'w, 's> {
    pub state_machine: Res<'w, PickingStateMachine>,
    pub parents: Query<'w, 's, &'static ChildOf>,
    pub propagation: Query<'w, 's, &'static PickingPropagation>,
}

impl PropagatedPickingStateMachine<'_, '_> {
    /// Events from `active` can propagate to `to`.
    pub fn entity_equivalent(&self, active: Entity, to: Entity) -> bool {
        if active == to {
            return true;
        }
        match self.propagation.get(active) {
            Ok(PickingPropagation::NoPropagation) => active == to,
            Ok(PickingPropagation::PropagateDown) | Err(_) => {
                let mut current = to;
                while let Ok(parent) = self.parents.get(current) {
                    if parent.parent() == active {
                        return true;
                    }
                    current = parent.parent();
                }
                false
            }
            Ok(PickingPropagation::PropagateUp(count)) => {
                let mut root = active;
                for _ in 0..*count {
                    if let Ok(parent) = self.parents.get(root) {
                        root = parent.parent();
                    } else {
                        break;
                    }
                }
                let mut current = to;
                while let Ok(parent) = self.parents.get(current) {
                    if parent.parent() == active || parent.parent() == root {
                        return true;
                    }
                    current = parent.parent();
                }
                false
            }
            Ok(PickingPropagation::AndPropagateUp(count)) => {
                let mut current = to;
                while let Ok(parent) = self.parents.get(current) {
                    if parent.parent() == active {
                        return true;
                    }
                    current = parent.parent();
                }
                let mut current = active;
                for _ in 0..*count {
                    if let Ok(parent) = self.parents.get(current) {
                        current = parent.parent();
                        if current == to {
                            return true;
                        }
                    } else {
                        return false;
                    }
                }
                false
            }
        }
    }

    /// Get the state of an entity, accounting for event propagations.
    pub fn get_state(&self, entity: Entity) -> EntityPickingState {
        let Some(active_entity) = self.state_machine.get_active_entity() else {
            return EntityPickingState::None;
        };
        if self.entity_equivalent(active_entity, entity) {
            self.state_machine.active_state()
        } else {
            EntityPickingState::None
        }
    }

    /// Get the transition of an entity, accounting for event propagations.
    pub fn get_transition(&self, entity: Entity) -> Option<PickingTransition> {
        self.state_machine
            .transitions
            .iter()
            .find(|x| self.entity_equivalent(x.entity(), entity))
    }
}
