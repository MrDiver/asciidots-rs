use crate::interpreter::DisplayCommand;

pub trait DisplayBackend: Send {
    fn handle(&mut self, command: DisplayCommand);

    /// Process a batch of commands.  Backends that hold an internal lock
    /// (e.g. ratatui's `Arc<RwLock<DisplayBuffer>>`) should override this to
    /// acquire the lock once for the entire batch instead of once per command.
    fn handle_batch(&mut self, commands: &[DisplayCommand]) {
        for cmd in commands {
            self.handle(cmd.clone());
        }
    }
}
