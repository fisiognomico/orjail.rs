use crate::errors::Errcode;
// TODO clean IPC
use crate::ipc::{recv_boolean, send_boolean};

use std::os::unix::io::RawFd;
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{getuid, getgid, Uid, Pid, Gid};
use nix::unistd::{setgroups, setresuid, setresgid};
use std::fs::{File, OpenOptions};
use std::io::Write;

// This function will be called by the child during its configuration
// to create its namespace. 
const UID_COUNT: u64 = 1;
const GID_COUNT: u64 = 1;

pub fn userns(real_uid: u32, real_gid: u32, target_uid: u32) -> Result<(), Errcode> {
    log::debug!("Switching to uid {} / gid {}...", target_uid, target_uid);

    if let Ok(mut uid_map) = File::create("/proc/self/uid_map") {
        if let Err(e) = uid_map.write_all(format!("{} {} {}", target_uid, real_uid, UID_COUNT).as_bytes()) {
            log::error!("Unable to open UID map: {:?}", e);
            return Err(Errcode::NamespacesError(4));
        }
    } else {
        log::error!("Unable to create the UID MAP");
        return Err(Errcode::NamespacesError(5));
    }

    // TODO is this step really needed? Is there a better way to handle GUID map?
    if let Ok(mut setgroups) = OpenOptions::new().write(true).open("/proc/self/setgroups") {
        if let Err(e) = setgroups.write_all("deny".as_bytes()) {
            log::error!("Unable to write to setgroups: {:?}", e);
            return Err(Errcode::NamespacesError(6));
        }
    }

    if let Ok(mut gid_map) = File::create("/proc/self/gid_map") {
        if let Err(e) = gid_map.write_all(format!("{} {} {}", target_uid, real_gid, GID_COUNT).as_bytes()) {
            log::error!("Unable to open GID map: {:?}", e);
            return Err(Errcode::NamespacesError(8));
        }
    } else {
        log::error!("Unable to create the GID MAP");
        return Err(Errcode::NamespacesError(7));
    }

    Ok(())
}
