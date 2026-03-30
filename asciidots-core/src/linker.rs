use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use log::trace;
use miette::{Context, IntoDiagnostic, NamedSource};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::native;
use crate::{
    interpreter::AsciiField,
    parser::{
        AsciiCell, AsciiFunc, DotsModule, ImportSource, LexError, Lexer,
        LinkingErrorCollection, Parser, ParsingErrorCollection, PreambleStatement,
    },
    primitives::Position,
};

struct ParsedModule {
    path: PathBuf,
    field: AsciiField,
    functions: Vec<AsciiFunc>,
    native_imports: Vec<String>,
}

pub struct LinkOutput {
    pub module: DotsModule,
}

#[derive(Clone, Debug)]
struct PathImportSpec {
    source_module: PathBuf,
    function_name: String,
}

#[derive(Clone, Debug)]
struct NativeImportSpec {
    name: String,
}

struct RecursiveLoadCtx<'a> {
    seen_order: &'a mut Vec<PathBuf>,
    modules: &'a mut BTreeMap<PathBuf, DotsModule>,
    module_native_imports: &'a mut BTreeMap<PathBuf, Vec<NativeImportSpec>>,
    available_native_functions: &'a FxHashSet<String>,
    active_stack: &'a mut Vec<PathBuf>,
    collected_errors: &'a mut LinkingErrorCollection,
}

pub fn link_file(
    path: &Path,
    available_native_functions: &FxHashSet<String>,
) -> Result<LinkOutput, LinkingErrorCollection> {
    let resolved_path = resolve_entry(path).map_err(LinkingErrorCollection::from_report)?;

    let mut seen_order = Vec::new();
    let mut modules = BTreeMap::new();
    let mut module_native_imports = BTreeMap::new();
    let mut active_stack = Vec::new();
    let mut collected_errors = LinkingErrorCollection::new();

    let mut load_ctx = RecursiveLoadCtx {
        seen_order: &mut seen_order,
        modules: &mut modules,
        module_native_imports: &mut module_native_imports,
        available_native_functions,
        active_stack: &mut active_stack,
        collected_errors: &mut collected_errors,
    };

    load_recursive(&resolved_path, &mut load_ctx)?;

    if !collected_errors.is_empty() {
        return Err(collected_errors);
    }

    let parsed_modules: Vec<ParsedModule> = seen_order
        .into_iter()
        .map(|module_path| {
            let module = modules
                .remove(&module_path)
                .expect("module order and module map must stay in sync");
            let (field, functions, _) = module.unpack();
            let native_imports = module_native_imports.remove(&module_path).unwrap_or(Vec::new()).iter().map(|spec|
                {spec.name.clone()}
            ).collect();
            ParsedModule {
                path: module_path,
                field,
                functions,
                native_imports,
            }
        })
        .collect();

    Ok(LinkOutput {
        module: merge_modules(parsed_modules),
    })
}

fn load_recursive(
    current_path: &Path,
    ctx: &mut RecursiveLoadCtx<'_>,
) -> Result<(), LinkingErrorCollection> {
    if ctx.modules.contains_key(current_path) {
        return Ok(());
    }

    if let Some(cycle_start) = ctx
        .active_stack
        .iter()
        .position(|path| path == current_path)
    {
        let mut cycle = ctx.active_stack[cycle_start..].to_vec();
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

    ctx.active_stack.push(current_path.to_path_buf());

    let source_text = match fs::read_to_string(current_path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read module '{}'", current_path.display()))
    {
        Ok(source) => source,
        Err(error) => {
            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let dots_source = match Lexer::new(&source_text).run() {
        Ok(source) => source,
        Err(error) => {
            if let Some(lex_error) = error.downcast_ref::<LexError>() {
                let report = miette::Report::from(lex_error.clone()).with_source_code(
                    NamedSource::new(current_path.display().to_string(), source_text.clone()),
                );
                ctx.collected_errors.push_error(report);
                ctx.active_stack.pop();
                return Ok(());
            }

            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let module_directory = current_path.parent().unwrap_or_else(|| Path::new("."));
    let mut native_imports = Vec::new();
    for statement in dots_source.preamble() {
        let PreambleStatement::Import {
            source,
            name,
            alias:_,
            span,
        } = statement
        else {
            continue;
        };

        match source {
            ImportSource::Path(path) => {
                let import_target = match resolve_import(module_directory, path) {
                    Ok(target) => target,
                    Err(error) => {
                        ctx.active_stack.pop();
                        return Err(LinkingErrorCollection::from_report(error));
                    }
                };

                load_recursive(
                    &import_target,
                    ctx,
                )?;
            }
            ImportSource::Native => {
                if !ctx.available_native_functions.contains(name) {
                    let report = miette::miette!(
                        labels = vec![miette::LabeledSpan::at(*span, "imported here")],
                        "unknown native function '{}': register it on DotsInterpreterBuilder before linking",
                        name
                    )
                    .with_source_code(NamedSource::new(
                        current_path.display().to_string(),
                        source_text.clone(),
                    ));
                    ctx.collected_errors.push_error(report);
                    continue;
                }

                native_imports.push(NativeImportSpec {
                    name: name.clone(),
                });
            }
        }
    }

    ctx.module_native_imports
        .insert(current_path.to_path_buf(), native_imports);

    let module = match Parser::new(dots_source).run() {
        Ok(module) => module,
        Err(error) => {
            if let Some(parsing_errors) = error.downcast_ref::<ParsingErrorCollection>() {
                let report = miette::Report::from(parsing_errors.clone()).with_source_code(
                    NamedSource::new(current_path.display().to_string(), source_text.clone()),
                );
                ctx.collected_errors.push_error(report);
                ctx.active_stack.pop();
                return Ok(());
            }

            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    ctx.modules.insert(current_path.to_path_buf(), module);
    ctx.seen_order.push(current_path.to_path_buf());

    ctx.active_stack.pop();
    Ok(())
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

fn merge_modules(
    modules: Vec<ParsedModule>,
) -> DotsModule {
    if modules.is_empty() {
        return DotsModule::new(AsciiField::new(0, 0), Vec::new(), Vec::new());
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
    let mut merged_native_names = Vec::new();

    for (mut module, (offset_x, offset_y)) in modules.into_iter().zip(placements.into_iter()) {
        shift_module_in_place(&mut module, offset_x, offset_y);

        let module_width = module.field.width();
        for (index, token) in module.field.into_tokens().into_iter().enumerate() {
            let target_x = offset_x + (index % module_width);
            let target_y = offset_y + (index / module_width);
            let target: Position = (target_x, target_y).into();
            merged_field.set_at(token, &target);
        }
        merged_functions.extend(module.functions);
        merged_native_names.extend(module.native_imports);
    }

    trace!("Merged native imports: {:?}", merged_native_names);
    DotsModule::new(merged_field, merged_functions, merged_native_names)
}

fn shift_module_in_place(module: &mut ParsedModule, offset_x: usize, offset_y: usize) {
    let width = module.field.width();
    let height = module.field.height();

    for y in 0..height {
        for x in 0..width {
            let position: Position = (x, y).into();
            if let Some(token) = module.field.at_spanned_mut(&position)
                && let AsciiCell::Warp { other, .. } = token.inner_mut()
            {
                other.x += offset_x as i32;
                other.y += offset_y as i32;
            }
        }
    }

    module.functions = module
        .functions
        .drain(..)
        .map(|function| {
            let (name, position) = function.unpack();
            AsciiFunc::new(
                name,
                (
                    position.x as usize + offset_x,
                    position.y as usize + offset_y,
                ),
            )
        })
        .collect();
}
