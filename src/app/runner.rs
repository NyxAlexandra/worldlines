use crate::App;

// TODO: `Unsize`?

/// A trait for types that can run an [`App`].
pub trait AppRunner: 'static {
    /// Run the schedules once.
    fn tick(&mut self, app: &mut App) {
        app.tick_all();
    }

    /// Run this app.
    fn run(self: Box<Self>, app: App);
}

impl<F> AppRunner for F
where
    F: FnMut(&mut App) + 'static,
{
    fn tick(&mut self, app: &mut App) {
        self(app);
    }

    fn run(mut self: Box<Self>, mut app: App) {
        loop {
            self.tick(&mut app);
        }
    }
}
