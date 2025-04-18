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
