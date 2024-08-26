use std::convert::Infallible;

use crate::App;

/// Encapsulated ECS logic that can be added to an [`App`].
pub trait Plugin {
    /// Error when loading this plugin.
    type Err;

    /// Load this plugin into an app.
    fn load(self, app: &mut App) -> Result<(), Self::Err>;
}

impl<F> Plugin for F
where
    F: FnOnce(&mut App),
{
    type Err = Infallible;

    fn load(self, app: &mut App) -> Result<(), Self::Err> {
        self(app);

        Ok(())
    }
}
