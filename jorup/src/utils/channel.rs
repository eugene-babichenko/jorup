use crate::common::JorupConfig;
use error_chain::ChainedError as _;
use jorup_lib::{PartialChannelDesc, VersionReq};
use std::path::{Path, PathBuf};

error_chain! {}

pub struct Channel {
    entry: jorup_lib::Entry,
    version: PartialChannelDesc,

    path: PathBuf,
}

const CHANNEL_NAME: &str = "CHANNEL_NAME";

impl Channel {
    pub fn arg<'a, 'b>() -> clap::Arg<'a, 'b>
    where
        'a: 'b,
    {
        clap::Arg::with_name(CHANNEL_NAME)
            .value_name("CHANNEL")
            .help("The channel to run jormungandr for, jorup uses the default channel otherwise")
            .validator(|s: String| {
                s.parse::<PartialChannelDesc>()
                    .map(|_channel| ())
                    .map_err(|err| err.display_chain().to_string())
            })
    }

    pub fn load<'a, 'b>(cfg: &'b mut JorupConfig, args: &clap::ArgMatches<'a>) -> Result<Self> {
        let mut channel_entered = cfg.current_channel().clone();

        let entry = if let Some(channel) = args.value_of(CHANNEL_NAME) {
            let jor = cfg
                .load_jor()
                .chain_err(|| "No jorfile... cannot operate")?;

            // should be save to unwrap as we have set a validator in the Argument
            // for the CLI to check it is valid
            channel_entered = channel.parse().unwrap();

            jor.entries()
                .values()
                .filter(|entry| channel_entered.matches(entry.channel()))
                .last()
                .cloned()
        } else {
            cfg.current_entry()
                .chain_err(|| "No jorfile... cannot operate")?
                .cloned()
        };

        if let Some(entry) = entry {
            Self::new(cfg, entry.clone(), channel_entered)
        } else {
            bail!("No entry available for the given version")
        }
    }

    fn new(
        cfg: &JorupConfig,
        entry: jorup_lib::Entry,
        channel_version: PartialChannelDesc,
    ) -> Result<Self> {
        let path = cfg
            .channel_dir()
            .join(entry.channel().channel().to_string())
            .join(entry.channel().date().to_string());
        std::fs::create_dir_all(&path)
            .chain_err(|| format!("Error while creating directory '{}'", path.display()))?;
        Ok(Channel {
            entry,
            version: channel_version,
            path,
        })
    }

    pub fn channel_version(&self) -> &PartialChannelDesc {
        &self.version
    }

    pub fn prepare(&self) -> Result<()> {
        self.install_block0_hash()
    }

    fn install_block0_hash(&self) -> Result<()> {
        let path = self.get_genesis_block_hash();
        let content = self.entry().genesis().block0_hash();

        write_all_to(&path, content).chain_err(|| format!("with file {}", path.display()))
    }

    pub fn jormungandr_version_req(&self) -> &VersionReq {
        self.entry().jormungandr_versions()
    }

    pub fn entry(&self) -> &jorup_lib::Entry {
        &self.entry
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.dir().join("NODE.logs")
    }

    pub fn get_runner_file(&self) -> PathBuf {
        self.dir().join("running_config.toml")
    }

    pub fn get_genesis_block_hash(&self) -> PathBuf {
        self.dir().join("genesis.block.hash")
    }

    pub fn get_node_storage(&self) -> PathBuf {
        self.dir().join("node-storage")
    }

    pub fn get_node_config(&self) -> PathBuf {
        self.dir().join("node-config.yaml")
    }

    pub fn get_node_secret(&self) -> PathBuf {
        self.dir().join("node-secret.yaml")
    }

    pub fn get_wallet_secret(&self) -> PathBuf {
        self.dir().join("wallet.secret.key")
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}

fn write_all_to<P, C>(path: P, content: C) -> std::io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    if path.as_ref().is_file() {
        return Ok(());
    }

    std::fs::write(path, content)
}
