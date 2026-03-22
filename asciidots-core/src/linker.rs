use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use miette::{Context, IntoDiagnostic, NamedSource};

use crate::{
    interpreter::AsciiField,
    parser::{
        AsciiCell, AsciiFunc, DotsModule, DotsSource, LexError, Lexer, LinkingErrorCollection,
        Parser, ParsingErrorCollection, PreambleStatement,
    },
    primitives::Position,
};

struct ParsedModule {
    field: AsciiField,
    functions: Vec<AsciiFunc>,
}

pub fn link_file(path: &Path) -> Result<DotsModule, LinkingErrorCollection> {
    let resolved_path =
        resolve_entry(path).map_err(|error| LinkingErrorCollection::from_report(error))?;

    let mut seen_order = Vec::new();
    let mut modules = BTreeMap::new();
    let mut active_stack = Vec::new();
    let mut collected_errors = LinkingErrorCollection::new();

    load_recursive(
        &resolved_path,
        &mut seen_order,
        &mut modules,
        &mut active_stack,
        &mut collected_errors,
    )?;

    if !collected_errors.is_empty() {
        return Err(collected_errors);
    }

    let parsed_modules: Vec<ParsedModule> = seen_order
        .into_iter()
        .map(|module_path| {
            let module = modules
                .remove(&module_path)
                .expect("module order and module map must stay in sync");
            let (field, functions) = module.unpack();
            ParsedModule { field, functions }
        })
        .collect();

    Ok(merge_modules(parsed_modules))
}

fn load_recursive(
    current_path: &Path,
    seen_order: &mut Vec<PathBuf>,
    modules: &mut BTreeMap<PathBuf, DotsModule>,
    active_stack: &mut Vec<PathBuf>,
    collected_errors: &mut LinkingErrorCollection,
) -> Result<(), LinkingErrorCollection> {
    if modules.contains_key(current_path) {
        return Ok(());
    }

    if let Some(cycle_start) = active_stack.iter().position(|path| path == current_path) {
        let mut cycle = active_stack[cycle_start..].to_vec();
        cycle.push(current_path.to_path_buf());
        let cycle_repr = cycle
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(LinkingErrorCollection::from_report(miette::miette!(
            "cyclic import detected: {cycle_repr}"
        )));
    }

    active_stack.push(current_path.to_path_buf());

    let source_text = match fs::read_to_string(current_path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read module '{}'", current_path.display()))
    {
        Ok(source) => source,
        Err(error) => {
            active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let dots_source = match Lexer::new(&source_text).run() {
        Ok(source) => source,
        Err(error) => {
            if let Some(lex_error) = error.downcast_ref::<LexError>() {
                let report = miette::Report::new(lex_error.clone()).with_source_code(
                    NamedSource::new(current_path.display().to_string(), source_text.clone()),
                );
                collected_errors.push_error(report);
                active_stack.pop();
                return Ok(());
            }

            active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let import_paths: Vec<String> = iter_import_paths(&dots_source)
        .map(ToString::to_string)
        .collect();

    let module_directory = current_path.parent().unwrap_or_else(|| Path::new("."));
    for import_path in import_paths {
        let import_target = match resolve_import(module_directory, &import_path) {
            Ok(target) => target,
            Err(error) => {
                active_stack.pop();
                return Err(LinkingErrorCollection::from_report(error));
            }
        };

        load_recursive(
            &import_target,
            seen_order,
            modules,
            active_stack,
            collected_errors,
        )?;
    }

    let module = match Parser::new(dots_source).run() {
        Ok(module) => module,
        Err(error) => {
            if let Some(parsing_errors) = error.downcast_ref::<ParsingErrorCollection>() {
                let report = miette::Report::new(parsing_errors.clone()).with_source_code(
                    NamedSource::new(current_path.display().to_string(), source_text.clone()),
                );
                collected_errors.push_error(report);
                active_stack.pop();
                return Ok(());
            }

            active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    modules.insert(current_path.to_path_buf(), module);
    seen_order.push(current_path.to_path_buf());

    active_stack.pop();
    Ok(())
}

fn iter_import_paths(source: &DotsSource) -> impl Iterator<Item = &str> {
    source
        .preamble()
        .iter()
        .filter_map(|statement| match statement {
            PreambleStatement::Import { path, .. } => Some(path.as_str()),
            _ => None,
        })
}

fn resolve_entry(path: &Path) -> miette::Result<PathBuf> {
    let direct = path.to_path_buf();
    if direct.exists() {
        return direct
            .canonicalize()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to resolve path '{}'", path.display()));
    }

    let with_extension = if path.extension().is_none() {
        path.with_extension("dots")
    } else {
        direct
    };

    with_extension
        .canonicalize()
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve path '{}'", with_extension.display()))
}

fn resolve_import(base_directory: &Path, import_path: &str) -> miette::Result<PathBuf> {
    let raw_import = Path::new(import_path);
    let direct = if raw_import.is_absolute() {
        raw_import.to_path_buf()
    } else {
        base_directory.join(raw_import)
    };

    if direct.exists() {
        return direct
            .canonicalize()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to resolve import '{}'", direct.display()));
    }

    let with_extension = if direct.extension().is_none() {
        direct.with_extension("dots")
    } else {
        direct
    };

    with_extension
        .canonicalize()
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve import '{}'", with_extension.display()))
}

fn merge_modules(modules: Vec<ParsedModule>) -> DotsModule {
    if modules.is_empty() {
        return DotsModule::new(AsciiField::new(0, 0), Vec::new());
    }

    let total_area: usize = modules
        .iter()
        .map(|module| module.field.width().max(1) * module.field.height().max(1))
        .sum();
    let max_width = modules
        .iter()
        .map(|module| module.field.width())
        .max()
        .unwrap_or_default();
    let target_width = (total_area as f64).sqrt().ceil() as usize;
    let shelf_width = target_width.max(max_width).max(1);

    let mut placements = Vec::with_capacity(modules.len());
    let mut cursor_x = 0usize;
    let mut cursor_y = 0usize;
    let mut shelf_height = 0usize;
    let mut final_width = 0usize;

    for module in &modules {
        let module_width = module.field.width();
        let module_height = module.field.height();

        if cursor_x > 0 && cursor_x + module_width > shelf_width {
            cursor_y += shelf_height;
            cursor_x = 0;
            shelf_height = 0;
        }

        placements.push((cursor_x, cursor_y));
        cursor_x += module_width;
        shelf_height = shelf_height.max(module_height);
        final_width = final_width.max(cursor_x);
    }

    let final_height = cursor_y + shelf_height;
    let mut merged_field = AsciiField::new(final_width, final_height);
    let mut merged_functions = Vec::new();

    for (module, (offset_x, offset_y)) in modules.into_iter().zip(placements.into_iter()) {
        for (x, y, token) in &module.field {
            let mut translated_token = token.clone();
            if let AsciiCell::Warp { other, .. } = translated_token.inner_mut() {
                other.x += offset_x as i32;
                other.y += offset_y as i32;
            }
            let target: Position = (x + offset_x, y + offset_y).into();
            merged_field.set_at(translated_token, &target);
        }

        for function in module.functions {
            let (name, position) = function.unpack();
            let global_pos = (
                position.x as usize + offset_x,
                position.y as usize + offset_y,
            );
            merged_functions.push(AsciiFunc::new(name, global_pos));
        }
    }

    DotsModule::new(merged_field, merged_functions)
}
