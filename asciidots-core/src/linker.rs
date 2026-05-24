use std::collections::BTreeMap;
use std::path::Path;

use log::trace;
use miette::NamedSource;
use rustc_hash::FxHashSet;

use crate::{
    filesystem::{Filesystem, NativeFilesystem},
    interpreter::AsciiField,
    parser::{
        AsciiCell, AsciiFunc, DotsModule, ImportSource, LexError, Lexer, LinkingErrorCollection,
        Parser, ParsingErrorCollection, PreambleStatement,
    },
    primitives::Position,
};

struct ParsedModule {
    key: String,
    field: AsciiField,
    functions: Vec<AsciiFunc>,
    native_imports: Vec<String>,
}

pub struct LinkOutput {
    pub module: DotsModule,
}

#[derive(Clone, Debug)]
struct NativeImportSpec {
    name: String,
}

struct RecursiveLoadCtx<'a> {
    seen_order: &'a mut Vec<String>,
    modules: &'a mut BTreeMap<String, DotsModule>,
    module_native_imports: &'a mut BTreeMap<String, Vec<NativeImportSpec>>,
    available_native_functions: &'a FxHashSet<String>,
    active_stack: &'a mut Vec<String>,
    collected_errors: &'a mut LinkingErrorCollection,
    fs: &'a dyn Filesystem,
}

pub fn link_file(
    path: &Path,
    available_native_functions: &FxHashSet<String>,
) -> Result<LinkOutput, LinkingErrorCollection> {
    let fs = NativeFilesystem;
    let path_str = path.to_string_lossy();
    link_with_fs(&path_str, &fs, available_native_functions)
}

pub fn link_with_fs(
    entry_path: &str,
    fs: &dyn Filesystem,
    available_native_functions: &FxHashSet<String>,
) -> Result<LinkOutput, LinkingErrorCollection> {
    let resolved = fs
        .resolve(None, entry_path)
        .map_err(LinkingErrorCollection::from_report)?;

    let mut seen_order = Vec::new();
    let mut modules = BTreeMap::new();
    let mut module_native_imports = BTreeMap::new();
    let mut active_stack = Vec::new();
    let mut collected_errors = LinkingErrorCollection::new();

    let mut ctx = RecursiveLoadCtx {
        seen_order: &mut seen_order,
        modules: &mut modules,
        module_native_imports: &mut module_native_imports,
        available_native_functions,
        active_stack: &mut active_stack,
        collected_errors: &mut collected_errors,
        fs,
    };

    load_recursive(&resolved, &mut ctx)?;

    if !collected_errors.is_empty() {
        return Err(collected_errors);
    }

    let parsed_modules: Vec<ParsedModule> = seen_order
        .into_iter()
        .filter_map(|key| {
            let module = modules.remove(&key)?;
            let (field, functions, _) = module.unpack();
            let native_imports = module_native_imports
                .remove(&key)
                .unwrap_or_default()
                .into_iter()
                .map(|spec| spec.name)
                .collect();
            Some(ParsedModule {
                key,
                field,
                functions,
                native_imports,
            })
        })
        .collect();

    Ok(LinkOutput {
        module: merge_modules(parsed_modules),
    })
}

fn load_recursive(
    current_key: &str,
    ctx: &mut RecursiveLoadCtx<'_>,
) -> Result<(), LinkingErrorCollection> {
    if ctx.modules.contains_key(current_key) {
        return Ok(());
    }

    if let Some(cycle_start) = ctx.active_stack.iter().position(|k| k == current_key) {
        let mut cycle = ctx.active_stack[cycle_start..].to_vec();
        cycle.push(current_key.to_string());
        let cycle_repr = cycle.join(" -> ");
        return Err(LinkingErrorCollection::from_report(miette::miette!(
            "cyclic import detected: {cycle_repr}"
        )));
    }

    ctx.active_stack.push(current_key.to_string());

    let source_text = match ctx.fs.read(current_key) {
        Ok(src) => src,
        Err(error) => {
            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let dots_source = match Lexer::new(&source_text).run() {
        Ok(source) => source,
        Err(error) => {
            if let Some(lex_error) = error.downcast_ref::<LexError>() {
                let report = miette::Report::from(lex_error.clone())
                    .with_source_code(NamedSource::new(current_key, source_text.clone()));
                ctx.collected_errors.push_error(report);
                ctx.active_stack.pop();
                return Ok(());
            }
            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    let mut native_imports = Vec::new();
    for statement in dots_source.preamble() {
        let PreambleStatement::Import {
            source,
            name,
            alias: _,
            span,
        } = statement
        else {
            continue;
        };

        match source {
            ImportSource::Path(import_path) => {
                let import_key = match ctx.fs.resolve(Some(current_key), import_path) {
                    Ok(key) => key,
                    Err(error) => {
                        ctx.active_stack.pop();
                        return Err(LinkingErrorCollection::from_report(error));
                    }
                };
                load_recursive(&import_key, ctx)?;
            }
            ImportSource::Native => {
                if !ctx.available_native_functions.contains(name) {
                    let report = miette::miette!(
                        labels = vec![miette::LabeledSpan::at(*span, "imported here")],
                        "unknown native function '{}': register it on DotsInterpreterBuilder before linking",
                        name
                    )
                    .with_source_code(NamedSource::new(current_key, source_text.clone()));
                    ctx.collected_errors.push_error(report);
                    continue;
                }
                native_imports.push(NativeImportSpec { name: name.clone() });
            }
        }
    }

    ctx.module_native_imports
        .insert(current_key.to_string(), native_imports);

    let module = match Parser::new(dots_source).run() {
        Ok(module) => module,
        Err(error) => {
            if let Some(parsing_errors) = error.downcast_ref::<ParsingErrorCollection>() {
                let report = miette::Report::from(parsing_errors.clone())
                    .with_source_code(NamedSource::new(current_key, source_text.clone()));
                ctx.collected_errors.push_error(report);
                ctx.active_stack.pop();
                return Ok(());
            }
            ctx.active_stack.pop();
            return Err(LinkingErrorCollection::from_report(error));
        }
    };

    ctx.modules.insert(current_key.to_string(), module);
    ctx.seen_order.push(current_key.to_string());
    ctx.active_stack.pop();
    Ok(())
}

fn merge_modules(modules: Vec<ParsedModule>) -> DotsModule {
    if modules.is_empty() {
        return DotsModule::new(AsciiField::new(0, 0), Vec::new(), Vec::new());
    }

    let total_area: usize = modules
        .iter()
        .map(|m| m.field.width().max(1) * m.field.height().max(1))
        .sum();
    let max_width = modules.iter().map(|m| m.field.width()).max().unwrap_or(0);
    let target_width = (total_area as f64).sqrt().ceil() as usize;
    let shelf_width = target_width.max(max_width).max(1);

    let mut placements = Vec::with_capacity(modules.len());
    let mut cursor_x = 0usize;
    let mut cursor_y = 0usize;
    let mut shelf_height = 0usize;
    let mut final_width = 0usize;

    for module in &modules {
        let w = module.field.width();
        let h = module.field.height();
        if cursor_x > 0 && cursor_x + w > shelf_width {
            cursor_y += shelf_height;
            cursor_x = 0;
            shelf_height = 0;
        }
        placements.push((cursor_x, cursor_y));
        cursor_x += w;
        shelf_height = shelf_height.max(h);
        final_width = final_width.max(cursor_x);
    }

    let final_height = cursor_y + shelf_height;
    let mut merged_field = AsciiField::new(final_width, final_height);
    let mut merged_functions = Vec::new();
    let mut merged_native_names = Vec::new();

    for (mut module, (offset_x, offset_y)) in modules.into_iter().zip(placements) {
        shift_module_in_place(&mut module, offset_x, offset_y);
        let module_width = module.field.width();
        for (index, token) in module.field.into_tokens().into_iter().enumerate() {
            let target_x = offset_x + (index % module_width);
            let target_y = offset_y + (index / module_width);
            merged_field.set_at(token, &(target_x, target_y).into());
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
