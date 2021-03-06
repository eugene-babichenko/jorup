mod commands;
mod common;
mod config;
mod jormungandr_config;
mod utils;

use commands::Cmd;
use std::{
    env::{self, consts::EXE_SUFFIX},
    error::Error,
    ffi::OsStr,
};
use structopt::StructOpt;

fn main() {
    let current_executable = env::current_exe().expect("Failed to get current executable name");
    let current_executable = current_executable.file_name().unwrap();
    let init_name = format!("jorup-init{}", EXE_SUFFIX);
    if current_executable == OsStr::new(&init_name) {
        run(commands::Install::from_args())
    } else {
        run(commands::RootCmd::from_args())
    }
}

fn run(app: impl Cmd) {
    if let Err(error) = app.run() {
        eprintln!("{}", error);
        let mut source = error.source();
        while let Some(err) = source {
            eprintln!(" |-> {}", err);
            source = err.source();
        }

        // TODO: https://github.com/rust-lang/rust/issues/43301
        //
        // as soon as #43301 is stabilized it would be nice to no use
        // `exit` but the more appropriate:
        // https://doc.rust-lang.org/stable/std/process/trait.Termination.html
        std::process::exit(1);
    }
}
