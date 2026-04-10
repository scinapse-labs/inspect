use clap::Args;
use colored::Colorize;

use crate::config;

#[derive(Args)]
pub struct LogoutArgs {}

pub fn run(_args: LogoutArgs) {
    match config::remove_credentials() {
        Ok(()) => println!("{}", "Logged out.".green()),
        Err(e) => {
            eprintln!("{}", format!("error: {e}").red());
            std::process::exit(1);
        }
    }
}
