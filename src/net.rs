use crate::errors::Errcode;
use crate::mountpoint::{create_directory, bind_mount_namespace};
use crate::utils::generate_random_str;

use nix::unistd::Pid;
use rtnetlink::{new_connection, AddressHandle, Handle};
use std::path::PathBuf;
use std::net;
use std::process::{Command, Stdio};


static NETNS: &str = "/var/run/netns/";

pub fn slirp(pid: Pid) -> Result<(), Errcode> {
    let pid_str = format!("{}", pid.as_raw());
    let run_slirp = Command::new("slirp4netns")
                    .args(["--configure", "--mtu=65520", "--disable-host-loopback", &pid_str, "tap0"])
                    .stdout(Stdio::null())
                    .spawn();
    Ok(())
}

pub fn mount_netns(hostname: &String) -> Result<(), Errcode> {
    let netns_mount = PathBuf::from(format!("/tmp/{}", hostname));
    create_directory(&netns_mount)?;
    let netns_dir = PathBuf::from(NETNS);
    // It's not mount(2) that I need to use
    if let Err(e) = bind_mount_namespace(&netns_mount, &netns_dir) {
        log::error!("Can not remount network namespace inside the container: {:?}", e);
        return Err(Errcode::NetworkError(2));
    }
    
    Ok(())

}

// pub fn setup_veth_peer(veth_idx: u32, ns_ip: &String, subnet: u8) -> Result<(), Errcode> {
//     let (connection, handle, _) = new_connection
// }
