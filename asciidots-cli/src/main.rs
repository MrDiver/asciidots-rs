use asciidots_cli::{check, run};
use clap::{
    Parser, Subcommand,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use miette::IntoDiagnostic;

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
    },
    #[command(about = "Compiles the file (for the asciidots vm)")]
    Build { path: String },
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { path } => {
            let content = std::fs::read_to_string(path).into_diagnostic()?;
            check(&content)?;
            println!("No errors found");
        }
        Commands::Run { path, tui } => {
            let content = std::fs::read_to_string(path).into_diagnostic()?;
            if tui {
                asciidots_tui::run(&content);
            } else {
                env_logger::init();
                run(&content)?;
            }
        }
        _ => todo!(),
    }
    Ok(())
}
