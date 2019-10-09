#[macro_use(error_chain, bail, quick_main)]
extern crate error_chain;
#[macro_use(crate_name, crate_version, crate_authors, crate_description, value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;

mod common;
mod info;
mod run;
mod shutdown;
mod update;
mod utils;
mod wallet;

use clap::{App, AppSettings};

quick_main!(run_main);

error_chain! {
    links {
        Common(common::Error, common::ErrorKind);
        Update(update::Error, update::ErrorKind);
        Run(run::Error, run::ErrorKind);
        Shutdown(shutdown::Error, shutdown::ErrorKind);
        Info(info::Error, info::ErrorKind);
        Wallet(wallet::Error, wallet::ErrorKind);
    }

    errors {
        NoCommand {
            description("No commands, try '--help' for more information")
        }

        UnknownCommand (cmd: String) {
            description("Unknown command"),
            display("Unknown command '{}', try '--help' to see full list of commands", cmd),
        }
    }
}

fn run_main() -> Result<()> {
    let mut app = App::new(crate_name!())
        .settings(&[AppSettings::ColorAuto, AppSettings::VersionlessSubcommands])
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .arg(common::arg::jorup_home()?)
        .arg(common::arg::generate_autocompletion())
        .arg(common::arg::jor_file())
        .arg(common::arg::offline())
        .subcommand(run::arg::command())
        .subcommand(shutdown::arg::command())
        .subcommand(info::arg::command())
        .subcommand(wallet::arg::command())
        .subcommand(update::arg::command());

    let matches = app.clone().get_matches();

    if let Some(shell) = matches.value_of(common::arg::name::GENERATE_AUTOCOMPLETION) {
        // safe to unwrap as possible values have been validated first
        let shell = shell.parse().unwrap();

        app.gen_completions_to(crate_name!(), shell, &mut std::io::stdout());
        return Ok(());
    }

    let cfg = common::JorupConfig::new(&matches)?;

    match matches.subcommand() {
        (update::arg::name::COMMAND, matches) => update::run(cfg, matches.unwrap())?,
        (run::arg::name::COMMAND, matches) => run::run(cfg, matches.unwrap())?,
        (shutdown::arg::name::COMMAND, matches) => shutdown::run(cfg, matches.unwrap())?,
        (info::arg::name::COMMAND, matches) => info::run(cfg, matches.unwrap())?,
        (wallet::arg::name::COMMAND, matches) => wallet::run(cfg, matches.unwrap())?,
        (cmd, _) => {
            if cmd.is_empty() {
                bail!(ErrorKind::NoCommand)
            }
            bail!(ErrorKind::UnknownCommand(cmd.to_owned()))
        }
    }

    Ok(())
}
