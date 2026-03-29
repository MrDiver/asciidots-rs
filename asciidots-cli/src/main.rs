use asciidots_cli::{check_file, graph_file, inspect_file, run_file};
use miette::IntoDiagnostic;
use asciidots_tui::TuiDisplayMode;
use clap::{
    Parser, Subcommand, ValueEnum,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Blue.on_default() | Effects::BOLD)
        .usage(AnsiColor::White.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Green.on_default() | Effects::BOLD)
        .invalid(AnsiColor::Red.on_default() | Effects::BOLD)
}

#[derive(ValueEnum, Clone, Default)]
enum DisplayBackend {
    #[default]
    Minifb,
    Ratatui,
    None,
}

#[derive(Parser)]
#[command(version = "0.1", about = "CLI for asccidots interpreter and compiler and lsp", long_about=None, styles=styles())]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Checks the file for errors")]
    Check { path: String },
    #[command(about = "Runs the file with the interpreter")]
    Run {
        path: String,
        #[arg(long)]
        tui: bool,
        #[arg(long, default_value = "minifb")]
        display: DisplayBackend,
    },
    #[command(about = "Compiles the file (for the asciidots vm)")]
    Build { path: String },
    #[command(about = "Outputs a GraphViz DOT graph of the program's dataflow")]
    Graph {
        path: String,
        #[arg(long, short)]
        output: Option<String>,
    },
    #[command(about = "Runs and prints runtime metrics periodically")]
    Inspect {
        path: String,
        #[arg(long, default_value_t = 250)]
        every: usize,
        #[arg(long)]
        max_steps: Option<usize>,
    },
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { path } => {
            check_file(&path)?;
            println!("No errors found");
        }
        Commands::Run { path, tui, display } => {
            if tui {
                let display_mode = match display {
                    DisplayBackend::Minifb => TuiDisplayMode::Minifb,
                    DisplayBackend::Ratatui | DisplayBackend::None => TuiDisplayMode::Ratatui,
                };
                asciidots_tui::run_file(&path, display_mode)?;
            } else {
                env_logger::init();
                run_file(&path, matches!(display, DisplayBackend::Minifb))?;
            }
        }
        Commands::Inspect {
            path,
            every,
            max_steps,
        } => {
            inspect_file(&path, every, max_steps)?;
        }
        Commands::Graph { path, output } => {
            let dot = graph_file(&path)?;
            match output {
                Some(out_path) => std::fs::write(&out_path, &dot).into_diagnostic()?,
                None => print!("{}", dot),
            }
        }
        _ => todo!(),
    }
    Ok(())
}