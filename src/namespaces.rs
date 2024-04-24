use crate::errors::Errcode;
use crate::ipc::{recv_boolean, send_boolean};

use std::os::unix::io::RawFd;
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{Uid, Pid, Gid};
use nix::unistd::{setgroups, setresuid, setresgid};
use std::fs::{File, OpenOptions};
use std::io::Write;

// This function will be called by the child during its configuration
// to create its namespace. 
// TODO set the capability to 
// const USERNS_OFFSET: u64 = 10000;
// const USERNS_COUNT: u64 = 2000;
// the values above seem to break substially the capability of the container
const UID_OFFSET: u64 = 1000;
const UID_COUNT: u64 = 1;
const GID_OFFSET: u64 = 1001;
const GID_COUNT: u64 = 1;

pub fn userns(fd: RawFd, uid: u32) -> Result<(), Errcode> {
    log::debug!("Switching to uid {} / gid {}...", uid, uid);
    let gid = Gid::from_raw(uid);
    let uid = Uid::from_raw(uid);

    // if let Err(_) = setgroups(&[gid]) {
    //     return Err(Errcode::NamespacesError(1));
    // }

    // if let Err(_) = setresgid(gid, gid, gid) {
    //     return Err(Errcode::NamespacesError(2));
    // }

    // if let Err(_) = setresuid(uid, uid, uid) {
    //     return Err(Errcode::NamespacesError(3));
    // }

    log::debug!("Creating namespace with UID: {}", uid);
    // // The call to unshare with CLONE_NEWUSER perform the following:
    // // "Unshare the user namespace, so that the calling process is moved
    // // into a new user namespace which is not shared with any previously existing process."
    // let has_userns = match unshare(CloneFlags::CLONE_NEWUSER) {
    //     Ok(_) => true,
    //     Err(_) => false,
    // };
    // send_boolean(fd, true)?;
    // // Check handle_child_uid_map implementation
    // if recv_boolean(fd)? {
    //     return Err(Errcode::NamespacesError(0));
    // }

    // if has_userns {
    //     log::info!("User namespace setup");
    // } else {
    //     log::info!("User namespaces not supported, continuing...");
    // }
    // TODO send/recv mistery: if I left the send_boolean above the handle_child_uid_map executes
    // as expected, without it does NOT!
    if let Ok(mut uid_map) = File::create("/proc/self/uid_map") {
        if let Err(e) = uid_map.write_all(format!("0 {} {}", UID_OFFSET, UID_COUNT).as_bytes()) {
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
        if let Err(e) = gid_map.write_all(format!("0 {} {}", GID_OFFSET, GID_COUNT).as_bytes()) {
            log::error!("Unable to open GID map: {:?}", e);
            return Err(Errcode::NamespacesError(8));
        }
    } else {
        log::error!("Unable to create the GID MAP");
        return Err(Errcode::NamespacesError(7));
    }

    // Switch UID/GID with the one provided by the User
    Ok(())
}

pub fn handle_child_uid_map(pid: Pid, fd: RawFd) -> Result<(), Errcode> {
    // if recv_boolean(fd)? {
        if let Ok(mut uid_map) = File::create(format!("/proc/{}/{}", pid.as_raw(), "uid_map")) {
            if let Err(e) = uid_map.write_all(format!("0 {} {}", UID_OFFSET, UID_COUNT).as_bytes()) {
                log::error!("Unable to open UID map: {:?}", e);
                return Err(Errcode::NamespacesError(4));
            }
        } else {
            log::error!("Unable to create the UID MAP");
            return Err(Errcode::NamespacesError(5));
        }

        // TODO is this step really needed? Is there a better way to handle GUID map?
        if let Ok(mut setgroups) = OpenOptions::new().write(true)
                                .read(false).create(false).truncate(false)
                                .open(format!("/proc/{}/{}", pid.as_raw(), "setgroups"))
            {
            if let Err(e) = setgroups.write_all("deny".as_bytes()) {
                log::error!("Unable to write to setgroups: {:?}", e);
                return Err(Errcode::NamespacesError(6));
            }
        }

        if let Ok(mut gid_map) = File::create(format!("/proc/{}/{}", pid.as_raw(), "gid_map")) {
            if let Err(e) = gid_map.write_all(format!("0 {} {}", GID_OFFSET, GID_COUNT).as_bytes()) {
                log::error!("Unable to open GID map: {:?}", e);
                return Err(Errcode::NamespacesError(6));
            }
        } else {
            log::error!("Unable to create the GID MAP");
            return Err(Errcode::NamespacesError(7));
        }
    // } else {
    //     log::info!("No user namespace set up from child process");
    // }

    // log::info!("Child UID/GID map done, sending signal to child to continue...");
    // send_boolean(fd, false)
    Ok(())
}
