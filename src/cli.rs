use std::path::PathBuf;
use structopt::StructOpt;
use crate::errors::Errcode;

#[derive(Debug, StructOpt)]
#[structopt(name = "orjail", about = "Container runtime that strictly forces traffic through TOR.")]
pub struct Args {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Command to execute inside the container
    #[structopt(short, long)]
    pub command: String,

    /// User ID to create inside the container
    #[structopt(default_value = "0", short = "u", long = "uid")]
    pub uid: u32,

    /// User ID to map inside the container
    #[structopt(default_value = "4294967295", long = "real-uid")]
    pub real_uid: u32,

    /// Group ID to map inside the container
    #[structopt(default_value = "4294967295", long = "real-gid")]
    pub real_gid: u32,

    #[structopt(parse(from_os_str), short = "a", long = "add")]
    pub addpaths: Vec<PathBuf>,

    /// Directory to mount as root of the container
    #[structopt(default_value = "", short = "m", long = "mount")]
    pub mount_dir: String,

    /// Name of the newtork namespace to create
    #[structopt(default_value = "test", short, long)]
    pub namespace: String,

    /// Set custom TOR binary
    #[structopt(default_value = "", short, long)]
    pub tor: String,

    /// Set custom slirp4netns binary
    #[structopt(default_value = "", short, long)]
    pub slirp4netns: String,

    /// Disable syscall filtering
    #[structopt(long)]
    pub disable_syscall: bool,

    /// Disable capabilities drop
    #[structopt(long)]
    pub disable_capabilities: bool
}

pub fn parse_args() -> Result<Args, Errcode> {
    let args = Args::from_args();

    // If args.debug: Setup log at debug level
    // Else: Setup log at info level
    if args.debug{
        setup_log(log::LevelFilter::Debug);
    } else {
        setup_log(log::LevelFilter::Info);
    }

    // Validate arguments
    // TODO this will be performed after a recheck of set_container_mountpoint
    // if !args.mount_dir.exists() || !args.mount_dir.is_dir(){
    //     return Err(Errcode::ArgumentInvalid("mount"));
    // }

    if args.command.is_empty() {
        return Err(Errcode::ArgumentInvalid("command"));
    }

    Ok(args)
}

pub fn setup_log(level: log::LevelFilter){
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .filter(None, level)
        .init();
}
