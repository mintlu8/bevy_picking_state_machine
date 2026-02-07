use bevy::{ecs::component::Component, input::mouse::MouseButton};

/// Filters which button can trigger an entity's `Pressed`.
#[derive(Debug, Clone, Default, Component)]
pub struct ButtonFilter(Vec<MouseButton>);

impl ButtonFilter {
    pub fn new(iter: impl IntoIterator<Item = MouseButton>) -> Self {
        ButtonFilter(iter.into_iter().collect())
    }
    pub fn contains(&self, btn: MouseButton) -> bool {
        self.0.contains(&btn)
    }
}

/// Optional component that increase or decrease the priority of the item.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct PickPriority {
    /// Modifies the backend order, bigger value gets prioritized.
    pub order: f32,
    /// Modifies (subtracts from) the distance from camera, bigger value gets prioritized.
    pub distance: f32,
}
