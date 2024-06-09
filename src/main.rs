mod capabilities;
mod child;
mod cli;
mod config;
mod container;
mod errors;
mod hostname;
mod ipc;
mod mountpoint;
mod namespaces;
mod net;
mod nftables;
mod resources;
mod slirp;
mod syscalls;
mod tor;
mod utils;

use errors::exit_with_retcode;
use std::process::exit;

fn main() {
    match cli::parse_args(){
        Ok(args) => {
            log::info!("{:?}", args);
            exit_with_retcode(container::start(args));
        },
        Err(e) => {
            log::error!("Error while parsing arguments:\n\t{}", e);
            exit(e.get_retcode());
        }
    };
}
