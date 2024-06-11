use crate::capabilities::setcapabilities;
use crate::config::ContainerOpts;
use crate::errors::{Errcode, exit_with_errcode};
use crate::hostname::set_container_hostname;
use crate::mountpoint::remount_root;
use crate::namespaces::{mount_netns, split_namespace, userns};
use crate::net::prepare_net;
use crate::nftables::test_apply_ruleset;
use crate::syscalls::setsyscalls;

use nix::unistd::{Pid, execve};
use nix::sched::clone;
use nix::sys::signal::Signal;
use nix::sched::CloneFlags;
use std::ffi::CString;

const STACK_SIZE: usize = 1024 * 1024;

pub fn generate_child_process(config: &mut ContainerOpts) -> Result<Pid, Errcode> {
    let mut tmp_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWUSER);
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWUTS);

    // TODO upgade to nix latest and investigate the feasibility of passing
    // NULL as the child stack.
    unsafe {
        match clone(
            Box::new(|| child(config)),
            &mut tmp_stack,
            flags,
            Some(Signal::SIGCHLD as i32)
            ) {
            Ok(pid) => Ok(pid),
            Err(_) => Err(Errcode::ChildProcessError(0)),
        }
    }
}

fn child(config: &mut ContainerOpts) -> isize {
    match setup_container_configurations(config) {
        Ok(()) => log::info!("Container setup successfully!"),
        Err(e) => {
            log::error!("Error while configuring container: {:?}", e);
            return -1;
        }
    }

    // TODO clean socket conf
    log::info!("Starting container with command: {} and args: {:?}", config.path.to_str().unwrap(), config.argv);
    // Switch to target network namespace afteer the configuration is done
    if let Err(e) = split_namespace(&config.namespace) {
        exit_with_errcode(e);
    }

    let retcode = match execve::<CString, CString>(&config.path, &config.argv, &[]) {
        Ok(_) => 0,
        Err(e) => {
            log::error!("Error while trying to perform execve {:?}", e);
            return -1;
        }
    };
    retcode
}

fn setup_container_configurations(config: &mut ContainerOpts) -> Result<(), Errcode> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    set_container_hostname(&config.hostname)?;
    // TODO by looking at the rnetlink implementation maybe we do not need this step
    if let Err(e) = mount_netns(&config.hostname) {
        log::error!("{:?}", e);
        // TODO return error and check status for each call
    };
    if let Err(e) = userns(config.real_uid, config.real_gid, config.uid) {
        log::error!("Error in namespace configuration: {:?}", e);
    }

    remount_root()?;
    // TODO maybe change name to the network namespace and make these parameters configurable
    let (_veth_idx, _veth_2_idx) = rt.block_on(
        prepare_net(&config.namespace, "10.40.50.10", "10.40.50.20", 24)).expect("Failed to prepare network");
    // TODO namespace configuration and clean!
    // TODO all this should be configurable
    test_apply_ruleset();

    // Last step drop capabilities and limit syscalls
    setcapabilities()?;
    setsyscalls()?;


    // Last step run TOR from the container
    config.spawn_tor();

    Ok(())
}

