// ── Stable public API ─────────────────────────────────────────────────────────
// Consumers of asciidots-core should import from these re-exports.

pub use display::DisplayBackend;
pub use filesystem::{Filesystem, MemFilesystem, NativeFilesystem};
pub use interpreter::{
    ColorMode, DisplayCommand, DotsInterpreter, DotsInterpreterBuilder, ExecutionBackend,
    InterpreterCommand, InterpreterResult,
};
pub use native::native_fun::NativeFunction;
pub use presentation::{
    AppCommand, AsciiMapModel, DotDebugTableModel, DotsView, GridCell, GridCellKind, GridModel,
    ListCell, ListCellKind, ListModel, OutputEntry, OutputListModel, RuntimeController,
    SettingsTableModel, TableCell, TableCellKind, TableModel, apply_result_to_output,
};
pub use state_machine::{DotMachine, RuntimeDiagnostic};

// ── Modules ───────────────────────────────────────────────────────────────────

pub mod display;
pub mod engine;
pub mod filesystem;
pub mod interpreter;
pub mod linker;
pub mod native;
pub mod parser;
pub mod presentation;
pub mod primitives;
pub mod state_machine;

pub use native::native_fun;
