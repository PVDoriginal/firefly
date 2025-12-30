//! Module containing logic for change detection.

use bevy::prelude::*;

use crate::prelude::Occluder2d;

/// Component that stores whether an entity has changed different attributes.
#[derive(Component, Clone, Default)]
pub struct Changes {
    pub translation: bool,
    pub shape: bool,
}

/// Plugin that handles change detection. Added automatically by [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct ChangePlugin;

impl Plugin for ChangePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, reset_changes);

        app.add_systems(Update, changed_occluders);
    }
}

// TODO: make this work better
fn changed_occluders(
    mut occluders: Query<&mut Changes, Or<(Changed<Transform>, Added<Occluder2d>)>>,
) {
    for mut changed in &mut occluders {
        changed.translation = true;
        changed.shape = true;
    }
}

fn reset_changes(mut entities: Query<&mut Changes>) {
    for mut changed in &mut entities {
        *changed = default();
    }
}
