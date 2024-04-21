use crate::errors::Errcode;
use crate::ipc::{recv_boolean, send_boolean};

use std::os::unix::io::RawFd;
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{Uid, Pid, Gid};
use nix::unistd::{setgroups, setresuid, setresgid};
use std::fs::File;
use std::io::Write;

// This function will be called by the child during its configuration
// to create its namespace. 
pub fn userns(fd: RawFd, uid: u32) -> Result<(), Errcode> {
    log::debug!("Creating namespace with UID: {}", uid);
    // The call to unshare with CLONE_NEWUSER perform the following:
    // "Unshare the user namespace, so that the calling process is moved
    // into a new user namespace which is not shared with any previously existing process."
    let has_userns = match unshare(CloneFlags::CLONE_NEWUSER) {
        Ok(_) => true,
        Err(_) => false,
    };
    send_boolean(fd, has_userns)?;
    // Check handle_child_uid_map implementation
    if recv_boolean(fd)? {
        return Err(Errcode::NamespacesError(0));
    }

    if has_userns {
        log::info!("User namespace setup");
    } else {
        log::info!("User namespaces not supported, continuing...");
    }

    // Switch UID/GID with the one provided by the User
    log::debug!("Switching to uid {} / gid {}...", uid, uid);
    let gid = Gid::from_raw(uid);
    let uid = Uid::from_raw(uid);

    if let Err(_) = setgroups(&[gid]) {
        return Err(Errcode::NamespacesError(1));
    }

    if let Err(_) = setresgid(gid, gid, gid) {
        return Err(Errcode::NamespacesError(2));
    }

    if let Err(_) = setresuid(uid, uid, uid) {
        return Err(Errcode::NamespacesError(3));
    }

    Ok(())
}

const USERNS_OFFSET: u64 = 10000;
const USERNS_COUNT: u64 = 2000;

pub fn handle_child_uid_map(pid: Pid, fd: RawFd) -> Result<(), Errcode> {
    if recv_boolean(fd)? {
        if let Ok(mut uid_map) = File::create(format!("/proc/{}/{}", pid.as_raw(), "uid_map")) {
            if let Err(_) = uid_map.write_all(format!("0 {} {}", USERNS_OFFSET, USERNS_COUNT).as_bytes()) {
                return Err(Errcode::NamespacesError(4));
            }
        } else {
            return Err(Errcode::NamespacesError(5));
        }

        if let Ok(mut gid_map) = File::create(format!("/proc/{}/{}", pid.as_raw(), "gid_map")) {
            if let Err(_) = gid_map.write_all(format!("0 {} {}", USERNS_OFFSET, USERNS_COUNT).as_bytes()) {
                return Err(Errcode::NamespacesError(6));
            }
        } else {
            return Err(Errcode::NamespacesError(7));
        }
    } else {
        log::info!("No user namespace setup from here!");
    }
    log::debug!("UID/GID map done, sending a signal to child to continue");
    send_boolean(fd, false)
}
