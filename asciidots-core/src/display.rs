use crate::interpreter::{DisplayCommand};

pub trait DisplayBackend: Send {
    fn handle(&mut self, command: DisplayCommand);
}
