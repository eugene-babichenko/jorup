use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
pub struct JorupConfig {
    home_dir: PathBuf,
    settings: JorupSettings,

    jor_file: Option<PathBuf>,
    offline: bool,
}

#[derive(Debug, Clone)]
pub enum Channel {
    Stable,
    Nightly,
    Specific { channel: jorup_lib::Channel },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JorupSettings {
    default: Channel,
}

error_chain! {
    errors {
        NoHOMEDir {
            description("No $HOME environment variable, can not set JORUP_HOME value.")
        }
        CannotCreateHomeDir(home_dir: PathBuf) {
            description("Cannot create the JORUP_HOME directory"),
            display("Cannot create JORUP_HOME [={}]", home_dir.display()),
        }
        CannotCreateInitDir(init_dir: PathBuf) {
            description("Cannot create one of the main HOME directory"),
            display("Cannot create directory [={}]", init_dir.display()),
        }
        CannotSaveSettings(file: PathBuf) {
            description("Cannot save the setting file"),
            display("Cannot save settings [={}]", file.display()),
        }
    }
}

impl Channel {
    pub fn is_nightly(&self) -> bool {
        match self {
            Self::Nightly => true,
            Self::Stable => false,
            Self::Specific { channel } => channel.is_nightly(),
        }
    }
}

impl JorupConfig {
    pub fn new<'a>(args: &ArgMatches<'a>) -> Result<Self> {
        let home_dir = value_t!(args, arg::name::JORUP_HOME, PathBuf).unwrap();
        std::fs::create_dir_all(&home_dir)
            .chain_err(|| ErrorKind::CannotCreateHomeDir(home_dir.clone()))?;

        let jor_file = if let Some(jor_file) = args.value_of(arg::name::JOR_FILE) {
            Some(jor_file.into())
        } else {
            None
        };
        let mut cfg = JorupConfig {
            home_dir,
            settings: JorupSettings::default(),
            jor_file,
            offline: args.is_present(arg::name::OFFLINE),
        };

        cfg.init()?;
        cfg.load_settings()?;
        cfg.detect_installed_path();

        Ok(cfg)
    }

    fn init(&self) -> Result<()> {
        std::fs::create_dir_all(self.bin_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.bin_dir()))?;
        std::fs::create_dir_all(self.channel_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.channel_dir()))?;
        std::fs::create_dir_all(self.release_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.release_dir()))?;

        if !self.jorup_settings_file().is_file() {
            self.save_settings()?;
        }

        Ok(())
    }

    fn detect_installed_path(&self) {
        let bin_dir = if self.bin_dir().is_absolute() {
            self.bin_dir()
        } else {
            std::env::current_dir().unwrap().join(self.bin_dir())
        };
        match std::env::var_os("PATH") {
            Some(paths) => {
                let present = std::env::split_paths(&paths).any(|path| path == bin_dir);
                if !present {
                    eprintln!(
                        "WARN: environment PATH does not contain bin dir: {}",
                        bin_dir.display()
                    );
                }

                let others: BTreeSet<_> = std::env::split_paths(&paths)
                    .filter(|path| path != &bin_dir)
                    .filter(|path| path.join("jormungandr").is_file())
                    .collect();
                for other in others {
                    eprintln!("WARN: found competing installation in {}", other.display());
                }
            }
            None => {
                eprintln!("WARN: no environment PATH recognized on this system");
            }
        }
    }

    pub fn settings(&self) -> &JorupSettings {
        &self.settings
    }

    pub fn current_channel(&self) -> &Channel {
        &self.settings().default
    }

    pub fn current_entry<'a>(&self, jor: &'a jorup_lib::Jor) -> Option<&'a jorup_lib::Entry> {
        let current_default = &self.settings.default;

        jor.entries()
            .values()
            .filter(|entry| current_default.is_nightly() == entry.channel().is_nightly())
            .next()
    }

    pub fn set_default_channel(&mut self, new_default: Channel) -> Result<()> {
        self.settings.default = new_default;
        self.save_settings()
    }

    fn load_settings(&mut self) -> Result<()> {
        let toml = std::fs::read_to_string(self.jorup_settings_file()).chain_err(|| {
            format!(
                "Cannot open settings file: {}",
                self.jorup_settings_file().display()
            )
        })?;

        self.settings = toml::from_str(&toml).chain_err(|| {
            format!(
                "Cannot parse settings file: {}",
                self.jorup_settings_file().display()
            )
        })?;
        Ok(())
    }

    fn save_settings(&self) -> Result<()> {
        std::fs::write(
            self.jorup_settings_file(),
            toml::to_vec(&self.settings).chain_err(|| "Cannot encode settings in Toml")?,
        )
        .chain_err(|| ErrorKind::CannotSaveSettings(self.jorup_settings_file()))
    }

    pub fn jorfile(&self) -> PathBuf {
        self.jor_file
            .clone()
            .unwrap_or(self.home_dir.join("jorfile.json"))
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.home_dir.join("bin")
    }
    pub fn channel_dir(&self) -> PathBuf {
        self.home_dir.join("channel")
    }
    pub fn release_dir(&self) -> PathBuf {
        self.home_dir.join("release")
    }
    pub fn jorup_settings_file(&self) -> PathBuf {
        self.home_dir.join("settings.toml")
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn sync_jorfile(&self) -> Result<()> {
        // do not sync if the jorfile was given as parameter of the
        // command line or if `--offline`
        if self.jor_file.is_some() || self.offline {
            return Ok(());
        }

        unimplemented!("fetching jor file from the network is not supported yet")
    }

    pub fn load_jor(&self) -> Result<jorup_lib::Jor> {
        let file = std::fs::File::open(self.jorfile())
            .chain_err(|| format!("Cannot open file {}", self.jorfile().display()))?;

        serde_json::from_reader(file)
            .chain_err(|| format!("cannot parse file {}", self.jorfile().display()))
    }
}

impl Default for JorupSettings {
    fn default() -> Self {
        JorupSettings {
            default: Channel::Stable,
        }
    }
}

impl std::str::FromStr for Channel {
    type Err = <jorup_lib::Channel as std::str::FromStr>::Err;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "stable" => Ok(Channel::Stable),
            "nightly" => Ok(Channel::Nightly),
            s => Ok(Channel::Specific {
                channel: s.parse()?,
            }),
        }
    }
}
impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Channel::Stable => "stable".fmt(f),
            Channel::Nightly => "nightly".fmt(f),
            Channel::Specific { channel } => channel.fmt(f),
        }
    }
}

impl Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

pub mod arg {
    use super::Result;
    use clap::Arg;

    pub mod name {
        pub const GENERATE_AUTOCOMPLETION: &str = "GENERATE_AUTOCOMPLETION";
        pub const JORUP_HOME: &str = "JORUP_HOME";
        pub const JOR_FILE: &str = "JOR_FILE";
        pub const OFFLINE: &str = "JORUP_OFFLINE";
    }

    pub fn jorup_home<'a, 'b>() -> Result<Arg<'a, 'b>> {
        let arg = Arg::with_name(name::JORUP_HOME)
            .long("jorup-home")
            .help("Set the directory home for jorup")
            .long_help(
                "Set the directory path where jorup will install the different
releases or different channels. Mainly remember to set `$JORUP_HOME/bin` value to your
$PATH for easy access to the default release's tools
",
            )
            .takes_value(true)
            .env(name::JORUP_HOME)
            .value_name(name::JORUP_HOME)
            .default_value_os(super::JORUP_HOME.as_os_str())
            .multiple(false)
            .global(true);
        Ok(arg)
    }

    pub fn jor_file<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::JOR_FILE)
            .long("jorfile")
            .help("don't use the jor file from from local setting but use given one")
            .long_help(
                "This is not to be used lightly as it may put your local jor in an invalid
state. Instead of fetching the jorfile from the network and/or to use the local one, use
a specific file. This is useful only for testing. This option does not imply offline.",
            )
            .takes_value(true)
            .value_name(name::JOR_FILE)
            .multiple(false)
            .hidden_short_help(true)
            .global(true)
    }

    pub fn offline<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::OFFLINE)
            .long("offline")
            .help("don't query the release server to update the index")
            .long_help(
                "Try only to work with the current states and values. Do not attempt to
update the known releases and testnets. This may make your system to fail to install specific
releases if they are not already cached locally.",
            )
            .multiple(false)
            .global(true)
    }

    pub fn generate_autocompletion<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::GENERATE_AUTOCOMPLETION)
            .long("generate-auto-completion")
            .help("generate autocompletion scripts for the given <SHELL>")
            .long_help(
                "Generate the autocompletion scripts for the given shell,
Autocompletion will be written in the standard output and can then be pasted
by the user to the appropriate place",
            )
            .takes_value(true)
            .possible_values(&clap::Shell::variants())
            .value_name("SHELL")
            .multiple(false)
            .global(true)
    }
}

lazy_static! {
    static ref JORUP_HOME: PathBuf = { jorup_home().unwrap() };
}

fn jorup_home() -> Result<PathBuf> {
    home::home_dir()
        .map(|d| d.join(".jorup"))
        .ok_or_else(|| ErrorKind::NoHOMEDir.into())
}
