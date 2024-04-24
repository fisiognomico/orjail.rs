use crate::capabilities::setcapabilities;
use crate::config::ContainerOpts;
use crate::errors::Errcode;
use crate::hostname::set_container_hostname;
use crate::mountpoint::set_container_mountpoint;
use crate::namespaces::userns;
use crate::syscalls::setsyscalls;

use nix::unistd::{Pid, close, execve};
use nix::sched::{unshare, clone};
use nix::sys::signal::Signal;
use nix::sched::CloneFlags;
use std::ffi::CString;
use std::iter::Cloned;

const STACK_SIZE: usize = 1024 * 1024;

pub fn generate_child_process(config: ContainerOpts) -> Result<Pid, Errcode> {
    let mut tmp_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    // TODO here we perfom a root only action, which one?
    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWUSER);
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWUTS);
    match unshare(flags) {
        Ok(_) => log::info!("Unshared namespace successfully!"),
        Err(e) => log::info!("Unable to unshare: {:?}", e),
    }

    flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_CHILD_SETTID);
    flags.insert(CloneFlags::CLONE_CHILD_CLEARTID);
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

    if let Err(e) = close(config.fd) {
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
    // TODO at the moment I do not need to change the mount point
    // as it will be carried out by bubblewrap
    // set_container_mountpoint(&config.mount_dir, &config.addpaths)?;
    if let Err(e) = userns(config.fd, config.uid) {
        log::error!("Error in namespace configuration: {:?}", e);
    }
    setcapabilities()?;
    // setsyscalls()?;
    Ok(())
}
