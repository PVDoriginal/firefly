//! Module containing logic for change detection.
//!
//! Change is divided into 2 categories: Form and Function.
//!
//! A function change is a change that only affects the entity's self-contained attributes, such as a light's color
//! or an occluder's opacity. This type of change causes new data to be transferred to the GPU, but doesn't
//! affect the order of the buffers themselves. For instance, changing a light's color won't change the occluders
//! that can possibly block it.    
//!
//! A form change is a change to an entity's worldly attributes, such as
//! an occluder's position, shape, or a light's range. These changes require
//! recomputations, such as a light moving and checking what occluders are now
//! in its vicinity.

use bevy::prelude::*;

use crate::prelude::Occluder2d;

/// Component that stores whether an entity has changes form or not.
#[derive(Component, Default)]
pub struct ChangedForm(pub bool);

/// Component that stores whether an entity has changed function or not.
#[derive(Component, Default)]
pub struct ChangedFunction(pub bool);

/// Plugin that handles change detection. Added automatically by [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct ChangePlugin;

impl Plugin for ChangePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, reset_changes);

        app.add_systems(Update, changed_occluders);
    }
}

fn changed_occluders(
    mut form_occluders: Query<&mut ChangedForm, Or<(Changed<Transform>, Added<Occluder2d>)>>,
) {
    for mut form in &mut form_occluders {
        form.0 = true;
    }
}

fn reset_changes(mut entities: Query<(&mut ChangedForm, &mut ChangedFunction)>) {
    for (mut form, mut function) in &mut entities {
        form.0 = false;
        function.0 = false;
    }
}
