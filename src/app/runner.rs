use crate::App;

// TODO: `Unsize`?

/// A trait for types that can run an [`App`].
pub trait AppRunner: 'static {
    /// Run this app.
    fn run(self: Box<Self>, app: App);
}

impl<F> AppRunner for F
where
    F: FnOnce(App) + 'static,
{
    fn run(self: Box<F>, app: App) {
        self(app);
    }
}
