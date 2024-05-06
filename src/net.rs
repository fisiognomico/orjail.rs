use crate::container::Container;
use crate::errors::Errcode;
use crate::ipc::send_u32;
use crate::mountpoint::{create_directory, bind_mount_namespace};
use crate::utils::generate_random_str;

use nix::unistd::Pid;
use rtnetlink::{new_connection, AddressHandle, Handle};
use std::net::{IpAddr, Ipv4Addr};
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;


static NETNS: &str = "/var/run/netns/";

pub fn slirp(pid: Pid) -> isize {
    let pid_str = format!("{}", pid.as_raw());
    // TODO catch error when spawning slirp4netns
    let slirp_process = Command::new("slirp4netns")
                    .args(["--configure", "--mtu=65520", "--disable-host-loopback", &pid_str, "tap0"])
                    .stdout(Stdio::null())
                    .spawn();
    slirp_process.unwrap().id() as isize

}

pub fn mount_netns(hostname: &String) -> Result<(), Errcode> {
    let netns_mount = PathBuf::from(format!("/tmp/{}", hostname));
    create_directory(&netns_mount)?;
    let netns_dir = PathBuf::from(NETNS);
    // It's not mount(2) that I need to use
    if let Err(e) = bind_mount_namespace(&netns_mount, &netns_dir) {
        log::error!("Can not remount network namespace inside the container: {:?}", e);
        return Err(Errcode::NetworkError(format!("Can not remount network namespace inside the container: {:?}", e)));
    }

    Ok(())

}

// TODO continue configure address interface definition
pub async fn setup_veth_peer(veth_idx: u32, ns_ip: &String, subnet: u8) -> Result<(), Errcode> {
    let (connection, handle, _) = new_connection()?;

    let veth2_addr = IpAddr::V4(Ipv4Addr::from_str(ns_ip)?);

    // Setup veth peer interface address
    AddressHandle::new(handle.clone()).add(veth_idx, veth2_addr, subnet).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting addr {} to veth with index {} failed: {}", ns_ip, veth_idx, e));
        });
    Ok(())
}
