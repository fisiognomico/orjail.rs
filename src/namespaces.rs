use crate::errors::Errcode;
use crate::mountpoint::{bind_mount_namespace, create_directory};

use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use rtnetlink::NetworkNamespace;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::io::Write;
use std::os::unix::io::RawFd;

// This function will be called by the child during its configuration
// to create its namespace. 
const UID_COUNT: u64 = 1;
const GID_COUNT: u64 = 1;
static NETNS: &str = "/var/run/netns/";

pub fn userns(real_uid: u32, real_gid: u32, target_uid: u32) -> Result<(), Errcode> {
    log::debug!("Switching to uid {} / gid {}...", target_uid, target_uid);

    if let Ok(mut uid_map) = File::create("/proc/self/uid_map") {
        if let Err(e) = uid_map.write_all(format!("{} {} {}", target_uid, real_uid, UID_COUNT).as_bytes()) {
            log::error!("Unable to open UID map: {:?}", e);
            return Err(Errcode::NamespacesError(format!("Unable to open UID Map: {}", e)));
        }
    } else {
        log::error!("Unable to create the UID MAP");
        return Err(Errcode::NamespacesError("Unable to create UID Map".to_string()));
    }

    if let Ok(mut setgroups) = OpenOptions::new().write(true).open("/proc/self/setgroups") {
        if let Err(e) = setgroups.write_all("deny".as_bytes()) {
            log::error!("Unable to write to setgroups: {:?}", e);
            return Err(Errcode::NamespacesError(format!("Unable to block setgroups: {}",e )));
        }
    }

    if let Ok(mut gid_map) = File::create("/proc/self/gid_map") {
        if let Err(e) = gid_map.write_all(format!("{} {} {}", target_uid, real_gid, GID_COUNT).as_bytes()) {
            log::error!("Unable to open GID map: {:?}", e);
            return Err(Errcode::NamespacesError(format!("Unable to open GID Map: {}", e)));
        }
    } else {
        log::error!("Unable to create the GID MAP");
        return Err(Errcode::NamespacesError("Unable to create GID map".to_string()));
    }

    Ok(())
}

pub async fn open_namespace(ns_name: &String) -> Result<RawFd, Errcode> {

    let ns_path = PathBuf::from(format!("{}{}", NETNS, ns_name));

    // Use rnetlink to create namespace
    NetworkNamespace::add(ns_name.to_string()).await.map_err(|e| {
        Errcode::NamespacesError(format!{"Can not create network namespace {}: {}", ns_name, e})
    })?;


    match open(&ns_path, OFlag::empty(), Mode::empty()) {
        Ok(fd) => return Ok(fd),
        Err(e) => {
            log::error!("Can not create network namespace {}: {}", ns_name, e);
            return Err(Errcode::NamespacesError(format!("Can not create network namespace {}: {}", ns_name, e)));
        }
    }
}

pub fn mount_netns(hostname: &String) -> Result<(), Errcode> {
    let netns_mount = PathBuf::from(format!("/tmp/{}", hostname));
    create_directory(&netns_mount)?;
    let netns_dir = PathBuf::from(NETNS);
    // It's not mount(2) that I need to use
    if let Err(e) = bind_mount_namespace(&netns_mount, &netns_dir) {
        log::error!("Can not remount network namespace inside the container: {:?}", e);
        return Err(Errcode::NamespacesError(format!("Can not remount network namespace inside the container: {:?}", e)));
    }

    Ok(())

}

