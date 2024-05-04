use crate::capabilities::setcapabilities;
use crate::config::ContainerOpts;
use crate::errors::Errcode;
use crate::hostname::set_container_hostname;
use crate::mountpoint::remount_root;
use crate::namespaces::userns;
use crate::net::mount_netns;
use crate::syscalls::setsyscalls;

use nix::unistd::{Pid, close, execve};
use nix::sched::{unshare, clone};
use nix::sys::signal::Signal;
use nix::sched::CloneFlags;
use std::ffi::CString;

const STACK_SIZE: usize = 1024 * 1024;

pub fn generate_child_process(config: ContainerOpts) -> Result<Pid, Errcode> {
    let mut tmp_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWUSER);
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    // FIX with the NEWPID namespace slirp can not find the process id!
    // flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWUTS);
    // match unshare(flags) {
    //     Ok(_) => log::info!("Unshared namespace successfully!"),
    //     Err(e) => log::info!("Unable to unshare: {:?}", e),
    // }

    // flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_CHILD_SETTID);
    flags.insert(CloneFlags::CLONE_CHILD_CLEARTID);
    // TODO upgade to nix latest and investigate the feasibility of passing
    // NULL as the child stack.
    match clone(
        Box::new(|| child(config.clone())),
        &mut tmp_stack,
        flags,
        Some(Signal::SIGCHLD as i32)
        ) {
        Ok(pid) => Ok(pid),
        Err(_) => Err(Errcode::ChildProcessError(0)),
    }
}

fn child(config: ContainerOpts) -> isize {
    match setup_container_configurations(&config) {
        Ok(()) => log::info!("Container setup successfully!"),
        Err(e) => {
            log::error!("Error while configuring container: {:?}", e);
            return -1;
        }
    }

    // TODO clean socket conf
    if let Err(_) = close(config.fd) {
        log::error!("Error while closing socket...");
        return -1;
    }

    log::info!("Starting container with command: {} and args: {:?}", config.path.to_str().unwrap(), config.argv);
    let retcode = match execve::<CString, CString>(&config.path, &config.argv, &[]) {
        Ok(_) => 0,
        Err(e) => {
            log::error!("Error while trying to perform execve {:?}", e);
            return -1;
        }
    };
    retcode
}

fn setup_container_configurations(config: &ContainerOpts) -> Result<(), Errcode> {
    set_container_hostname(&config.hostname)?;
    mount_netns(&config.hostname)?;
    if let Err(e) = userns(config.real_uid, config.real_gid, config.uid) {
        log::error!("Error in namespace configuration: {:?}", e);
    }

    remount_root()?;
    setcapabilities()?;
    // TODO namespace configuration and clean!
    // setsyscalls()?;
    Ok(())
}
