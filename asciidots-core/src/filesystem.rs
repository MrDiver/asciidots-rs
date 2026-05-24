use std::collections::HashMap;
use std::path::Path;

use miette::{Context, IntoDiagnostic};

pub trait Filesystem {
    fn read(&self, path: &str) -> miette::Result<String>;
    /// Resolve `import_path` relative to `from_file` (None = entry point).
    /// Returns an opaque canonical path key used for subsequent read() calls.
    fn resolve(&self, from_file: Option<&str>, import_path: &str) -> miette::Result<String>;
}

pub struct NativeFilesystem;

impl Filesystem for NativeFilesystem {
    fn read(&self, path: &str) -> miette::Result<String> {
        std::fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read '{path}'"))
    }

    fn resolve(&self, from_file: Option<&str>, import_path: &str) -> miette::Result<String> {
        let raw = Path::new(import_path);
        let candidate = match from_file {
            Some(f) if !raw.is_absolute() => {
                let parent = Path::new(f).parent().unwrap_or(Path::new("."));
                parent.join(raw)
            }
            _ => raw.to_path_buf(),
        };
        canonicalize_with_dots_fallback(&candidate)
    }
}

/// In-memory virtual filesystem for environments without a real filesystem (e.g. WASM).
/// Keys are normalized virtual paths like `"main.dots"` or `"lib/helper.dots"`.
pub struct MemFilesystem {
    files: HashMap<String, String>,
}

impl MemFilesystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        let normalized = normalize_virtual_path(&path.into());
        self.files.insert(normalized, content.into());
    }
}

impl Default for MemFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Filesystem for MemFilesystem {
    fn read(&self, path: &str) -> miette::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| miette::miette!("file not found in virtual filesystem: {path}"))
    }

    fn resolve(&self, from_file: Option<&str>, import_path: &str) -> miette::Result<String> {
        let normalized = match from_file {
            Some(from) if !import_path.starts_with('/') => {
                let parent = from.rfind('/').map(|i| &from[..i]).unwrap_or("");
                normalize_virtual_path(&format!("{parent}/{import_path}"))
            }
            _ => normalize_virtual_path(import_path),
        };

        if self.files.contains_key(&normalized) {
            return Ok(normalized);
        }

        let with_dots = format!("{normalized}.dots");
        if self.files.contains_key(&with_dots) {
            return Ok(with_dots);
        }

        Err(miette::miette!(
            "file not found in virtual filesystem: {normalized}"
        ))
    }
}

fn normalize_virtual_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn canonicalize_with_dots_fallback(path: &Path) -> miette::Result<String> {
    if path.exists() {
        return path
            .canonicalize()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to resolve '{}'", path.display()))
            .map(|p| p.display().to_string());
    }

    let with_ext = if path.extension().is_none() {
        path.with_extension("dots")
    } else {
        path.to_path_buf()
    };

    with_ext
        .canonicalize()
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve '{}'", with_ext.display()))
        .map(|p| p.display().to_string())
}
