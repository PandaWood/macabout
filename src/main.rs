//! macabout: an "About This Computer" dialog for Linux, or with `--text` the same summary on
//! stdout.

mod format;
mod parse;
mod system;
mod ui;

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

/// About This Computer: a system information dialog for Linux
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Show sample data instead of this computer's
    #[arg(long)]
    mock: bool,
    /// Print the summary to stdout instead of opening the window
    #[arg(long)]
    text: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let summary = format::Summary::new(system::collect(args.mock));
    if args.text {
        let mut out = std::io::stdout().lock();
        return match writeln!(out, "{}", summary.copy_text()).and_then(|()| out.flush()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    match ui::show_dialog(summary) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("macabout: {e}");
            ExitCode::FAILURE
        }
    }
}
