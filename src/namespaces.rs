use crate::errors::Errcode;
use crate::mountpoint::{bind_mount_namespace, create_directory, mount_directory};
// use crate::net::set_veth_up;

use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::mount::{mount, MsFlags};
use nix::sched::{CloneFlags, unshare, setns};
use nix::unistd::{fork, ForkResult, Pid};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::stat::{stat, Mode};
use nix::sys::statvfs::{statvfs, FsFlags};
use rtnetlink::{new_connection, NetworkNamespace};
use futures::TryStreamExt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::io::Write;
use std::os::unix::io::{FromRawFd ,RawFd};

// This function will be called by the child during its configuration
// to create its namespace.
const UID_COUNT: u64 = 1;
const GID_COUNT: u64 = 1;
static RUN: &str = "/run/";
static NETNS: &str = "/run/netns/";
static VAR_LIB: &str = "/var/lib";

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
            return Err(Errcode::NamespacesError(format!("Unable to block setgroups: {}",e ))); }
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
    let mount_dir_name = format!("/tmp/{}", hostname);
    let netns_dir_name = format!("{}/netns", mount_dir_name);
    let lib_dir_name = format!("{}/lib", mount_dir_name);

    let netns_mount = PathBuf::from(netns_dir_name);
    let lib_mount = PathBuf::from(lib_dir_name);


    create_directory(&netns_mount)?;
    let netns_dir = PathBuf::from(NETNS);
    // It's not mount(2) that I need to use
    // First check if netns exists in run
    match stat(&netns_dir) {
        Ok(_stat) => {
            if let Err(e) = bind_mount_namespace(&netns_mount, &netns_dir) {
                log::error!("Can not remount network namespace inside the container: {:?}", e);
                return Err(Errcode::NamespacesError(format!("Can not remount network namespace inside the container: {:?}", e)));
            }
        }

        // If /run/netns does not exist bind mount only /run and create it
        Err(Errno::ENOENT) => {
            let run_path = PathBuf::from(RUN);
            if let Err(e) = bind_mount_namespace(&netns_mount, &run_path) {
                log::error!("Can not remount network namespace inside the container: {:?}", e);
                return Err(Errcode::NamespacesError(format!("Can not remount network namespace inside the container: {:?}", e)));
            }
            create_directory(&netns_mount)?;
        }

        // What else can go wrong?
        Err(e) => {
            log::error!("Unknown error during stat of {}: {}", NETNS, e);
            return Err(Errcode::NamespacesError(format!("Error stat of {}: {}", NETNS, e)));
        }
    }

    create_directory(&lib_mount)?;
    let lib_dir = PathBuf::from(VAR_LIB);
    if let Err(e) = bind_mount_namespace(&lib_mount, &lib_dir) {
        log::error!("Can not remount network namespace inside the container: {:?}", e);
        return Err(Errcode::NamespacesError(format!("Can not remount network namespace inside the container: {:?}", e)));
    }

    Ok(())

}

pub async fn run_in_namespace(ns_name: &String, veth_ip: &str, veth_2_ip: &str) -> Result<(), Errcode> {
    prep_for_fork()?;
    // Configure networking in the child namespace:
    // Fork a process that is set to the newly created namespace
    // Here set the veth ip addr, routing tables etc.
    // Unfortunately the NetworkNamespace interface of rtnetlink does
    // not offer these functionalities
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            // Parent process
            log::debug!("Net configuration PID: {}", child.as_raw());
            run_parent(child)
        }
        Ok(ForkResult::Child) => {
            // Child process
            // Move the child to the target namespace
            run_child(ns_name, veth_ip, veth_2_ip).await
        }
        Err(e) => {
            log::error!("Can not fork() for ns creation: {}", e);
            return Err(Errcode::NamespacesError(format!("Error fork(): {}",e)));
        }
    }

}

fn run_parent(child: Pid) -> Result<(), Errcode> {
    log::trace!("[Parent] Child PID: {}", child);
    match waitpid(child, None) {
        Ok(wait_status) => match wait_status {
            WaitStatus::Exited(_, res) => {
                log::trace!("Child exited with: {}", res);
                if res == 0 {
                    return Ok(());
                } else {
                    log::error!("Child exited with status {}", res);
                    return Err(Errcode::NamespacesError(format!("Namespace conf error: child exited with {}", res)));
                }
            }
            WaitStatus::Signaled(_, signal, coredump) => {
                log::error!("Child process killed by signal {signal} with core dump {coredump}");
                return Err(Errcode::NamespacesError(format!("Child process killed by signal {:?}", signal)));
            }
            _ => {
                log::error!("Unknown child process status: {:?}", wait_status);
                return Err(Errcode::NamespacesError(format!("Unknown child process status {:?}", wait_status)));
            }
        }
        Err(e) => {
            log::error!("wait error : {}", e);
            return Err(Errcode::NamespacesError(format!("Error during wait: {}", e)));
        }
    }

}

async fn run_child(ns_name: &String, veth_ip: &str, veth_2_ip: &str) -> Result<(), Errcode> {
    let res = conf_netns_ifaces(ns_name, veth_ip, veth_2_ip).await;

    match res {
        Err(_) => {
            log::error!("Child process crashed");
            std::process::abort()
        }
        Ok(()) => {
            log::debug!("Child exited normally");
            exit(0)
        }
    }
}


async fn conf_netns_ifaces(ns_name: &String, veth_ip: &str, veth_2_ip: &str) -> Result<(), Errcode> {
    split_namespace(ns_name)?;
    net_conf(ns_name, veth_ip, veth_2_ip).await?;
    // ISSUE Unfortunately configuring the interfaces properly inside
    // a network namespace with rtnetlink results in a deadlock
    // set_lo_up().await?;
    Ok(())
}

pub fn split_namespace(ns_name: &String) -> Result<(), Errcode> {
    // Open NS path
    let ns_path = format!("{}{}", NETNS, ns_name);

    let mut open_flags = OFlag::empty();
    open_flags.insert(OFlag::O_RDONLY);
    open_flags.insert(OFlag::O_CLOEXEC);

    let fd = match open(Path::new(&ns_path), open_flags, Mode::empty()) {
        Ok(raw_fd) => unsafe {
            File::from_raw_fd(raw_fd)
        }
        Err(e) => {
            log::error!("Can not open network namespace: {}", e);
            return Err(Errcode::NamespacesError(format!("Can not open network namespace: {}", e)));
        }
    };
    // Switch to network namespace with CLONE_NEWNET
    if let Err(e) = setns(fd, CloneFlags::CLONE_NEWNET) {
        log::error!("Can not set namespace to target {}: {}", ns_name, e);
        return Err(Errcode::NamespacesError(format!("Unable to set target namespace: {}", e)));
    }
    // unshare with CLONE_NEWNS
    if let Err(e) = unshare(CloneFlags::CLONE_NEWNS) {
        log::error!("Can not unshare: {}", e);
        return Err(Errcode::NamespacesError(format!("Can not unshare: {}", e)));
    }
    // mount blind the fs
    // let's avoid that any mount propagates to the parent process
    mount_directory(None, &PathBuf::from("/"), vec![MsFlags::MS_REC, MsFlags::MS_PRIVATE])?;

    // Now unmount /sys
    let sys_path = PathBuf::from("/sys");
    let mut mount_flags = MsFlags::empty();
    // Needed to respect the trait for NixPath
    let ns_name_path = PathBuf::from(ns_name);

    // TODO do not exit for EINVAL error
    // unmount_path(&sys_path)?;
    // consider the case that a sysfs is not present
    let stat_sys = statvfs(&sys_path)
        .map_err(|e| {
            log::error!("Can not stat sys: {}", e);
    }).unwrap();
    if stat_sys.flags().contains(FsFlags::ST_RDONLY) {
        mount_flags.insert(MsFlags::MS_RDONLY);
    }

    // and remount a version of /sys that describes the network namespace
    if let Err(e) = mount::<PathBuf, PathBuf, str, PathBuf>(Some(&ns_name_path), &sys_path, Some("sysfs"), mount_flags, None) {
        log::error!("Can not remount /sys to namespace: {}", e);
        return Err(Errcode::NamespacesError(format!("Can not remount /sys to namespace: {}", e)));
    }

    // call net_conf

    Ok(())
}


// TODO need to open an issue to rtnetlink to find the proper way to configure an interface inside
// the created network namespace
// UPDATE Unfortunately it seems that the issue is not due to rtnetlink per se, but in an
// unexpected behaviour that emerges when an unshare is done after a tokio runtime has been spawned
// This would require to reimplement this block using the netlink API directly.
async fn net_conf(ns_name: &String, veth_ip: &str, veth_2_ip: &str) -> Result<(), Errcode> {
    let _lo_process = std::process::Command::new("ip")
        .args(["link", "set", "lo", "up"])
        .stdout(std::process::Stdio::null())
        .spawn()?;
    let veth_2 = format!("{}_peer", ns_name);
    let _up_process = std::process::Command::new("ip")
        .args(["link", "set", veth_2.as_str(), "up"])
        .stdout(std::process::Stdio::null())
        .spawn()?;
    // set_veth_up().await?;
    let addr_subnet = format!("{}/24", veth_2_ip);
    let _addr_process = std::process::Command::new("ip")
        .args(["addr", "add", addr_subnet.as_str(), "dev", veth_2.as_str()])
        .stdout(std::process::Stdio::null())
        .spawn()?;
    //
    let _route_process = std::process::Command::new("ip")
        .args(["route", "add", "default", "via", veth_ip, "dev", veth_2.as_str()])
        .stdout(std::process::Stdio::null())
        .spawn()?;


    Ok(())
}

// TODO Unfortunately it seems that using rtnetlink inside the forked process that has been moved
// to the target network namespace hangs undefinitively.
#[allow(dead_code)]
async fn set_veth_up() -> Result<(), Errcode> {
    let (_connection, handle, _) = new_connection()?;
    let mut links = handle.link().get().execute();
    'outer: while let Some(msg) = links.try_next().await? {
        for _nla in msg.attributes.into_iter() {
            log::debug!("found link {}", msg.header.index);
            continue 'outer;
        }
    }
    let veth_idx = handle.link().get().match_name("test_veth".to_string()).execute().try_next().await?
                .ok_or_else(|| Errcode::NetworkError(format!("Can not find lo interface ")))?
                .header.index;
    log::debug!("LO INTERFACE INDEX: {}", veth_idx);
    handle.link().set(veth_idx).up().execute().await
         .map_err(|e| {Errcode::NetworkError(format!("Can not set lo interface up: {}", e))
     })?;
     Ok(())
}

#[allow(dead_code)]
async fn set_lo_up() -> Result<(), Errcode> {
    let (_connection, handle, _) = new_connection()?;
    let veth_idx = handle.link().get().match_name("lo".to_string()).execute().try_next().await?
                .ok_or_else(|| Errcode::NetworkError(format!("Can not find lo interface ")))?
                .header.index;
    handle.link().set(veth_idx).up().execute().await
         .map_err(|e| {Errcode::NetworkError(format!("Can not set lo interface up: {}", e))
     })?;
     Ok(())
}


// Cargo cult from the definition in rtnetlink
fn prep_for_fork() -> Result<(), Errcode> {
    Ok(())
}
