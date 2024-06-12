use crate::cli::Args;
use crate::errors::Errcode;
use crate::config::ContainerOpts;
use crate::child::generate_child_process;
use crate::mountpoint::clean_mounts;
// use crate::resources::{clean_cgroups, restrict_resources};

use scan_fmt::scan_fmt;
use nix::sys::stat::stat;
use nix::sys::utsname::uname;
use nix::sys::wait::waitpid;
use nix::unistd::{getuid, getgid, Pid};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use which::which;

const PROCFS_UNPRIVILEGED_NS: &str = "/proc/sys/kernel/unprivileged_userns_clone";

pub struct Container{
    pub config: ContainerOpts,
    pub child: Option<Pid>,
}

impl Container {
    pub fn new(args: Args) -> Result<Container, Errcode> {
        let mut addpaths = vec![];
        for ap_pair in args.addpaths.iter(){
            let mut pair = ap_pair.to_str().unwrap().split(":");
            let frompath = PathBuf::from(pair.next().unwrap())
                .canonicalize().expect("Cannot canonicalize path")
                .to_path_buf();
            let mntpath = PathBuf::from(pair.next().unwrap())
                .strip_prefix("/").expect("Cannot strip prefix from path")
                .to_path_buf();
            addpaths.push((frompath, mntpath));
        }

        // match default value for uid/gid
        let real_uid = match args.real_uid {
            0 => getuid().as_raw(),
            _        => args.real_uid,
        };

        let real_gid = match args.real_gid {
            0 => getgid().as_raw(),
            _        => args.real_uid,
        };

        let tor_path = check_binary(&args.tor, "tor")?;
        let slirp_path = check_binary(&args.slirp4netns, "slirp4netns")?;

        let mount_dir;
        if args.mount_dir.is_empty() {
            mount_dir = None;
        } else {
            mount_dir = Some(PathBuf::from(args.mount_dir));
        }

        let config = ContainerOpts::new(
            args.command,
            args.uid,
            real_uid,
            real_gid,
            mount_dir,
            args.namespace,
            addpaths,
            tor_path,
            slirp_path,
            args.disable_syscall,
            args.disable_capabilities)?;
        Ok(Container {
            config,
            child: None,
            })
        }

    pub fn create(&mut self) -> Result<(), Errcode> {
        let pid = generate_child_process(&mut self.config)?;
        // TODO investigate why cgroup constraints result in a deadlock
        // restrict_resources(&self.config.hostname, pid)?;
        self.child = Some(pid);

        log::debug!("Creation finished, PID: {:?} ", self.child.unwrap());
        Ok(())
    }

    pub fn clean_exit(&mut self) -> Result<(), Errcode>{
        log::debug!("Cleaning container");

        // Here we can not catch errors as its not returned

        clean_mounts(&self.config.mount_dir)?;

        // if let Err(e) = clean_cgroups(&self.config.hostname) {
        //     log::error!("Cgroups cleaning failed: {}", e);
        //     return Err(e);
        // }
        Ok(())
    }

}

pub fn start(args: Args) -> Result<(), Errcode> {
    check_compatibility()?;

    let mut container = Container::new(args)?;
    if let Err(e) = container.create(){
        container.clean_exit()?;
        log::error!("Error while creating container: {:?}", e);
        return Err(e);
    }
    // Set container cgroup constraints

    log::debug!("Container child PID: {:?}", container.child.unwrap());
    container.config.spawn_slirp(container.child.unwrap());

    wait_child(container.child)?;
    log::debug!("Finished, cleaning & exit");

    container.clean_exit()
}

pub fn wait_child(pid: Option<Pid>) -> Result<(), Errcode> {
    if let Some(child_pid) = pid {
        log::debug!("Waiting for child (pid {}) to finish", child_pid);
        if let Err(e) = waitpid(child_pid, None) {
            log::error!("Error while waiting for child to finish: {:?}", e);
            return Err(Errcode::ContainerError(format!("Error while waiting for child to finish: {:?}", e)));
        }
    }
    Ok(())
}

pub const MINIMAL_KERNEL_VERSION: f32 = 4.8;

pub fn check_compatibility() -> Result<(), Errcode> {
    let host = uname();
    log::debug!("Linux release: {}", host.unwrap().release().to_str().unwrap());

    if let Ok(version) = scan_fmt!(host.unwrap().release().to_str().unwrap(), "{f}.{}", f32) {
        if version < MINIMAL_KERNEL_VERSION {
            return Err(Errcode::NotSupported(0));
        }
    } else {
        return Err(Errcode::ContainerError("Can not parse kernel release version".to_string()));
    }

    if host.unwrap().machine() != "x86_64" {
        return Err(Errcode::NotSupported(1));
    }

    // Check that unprivileged namespaces are enabled
    let mut ns_fh = File::open(PROCFS_UNPRIVILEGED_NS)?;
    let mut byte_buf = vec![0; 2];
    ns_fh.read(&mut byte_buf)?;

    match byte_buf[0] {
        b'1' => Ok(()),
        b'0' => {
            log::error!("Unprivileged namespaces are not supported!");
            log::error!("Please check the value of {}, and set it to 1", PROCFS_UNPRIVILEGED_NS);
            log::error!("For example on Debian you can run as root: sysctl kernel.unprivileged_userns_clone=1");
            Err(Errcode::ContainerError("Unprivileged namespaces disabled".to_string()))
        }
        _ => {
            log::error!("Unexpected value read from {}", PROCFS_UNPRIVILEGED_NS);
            Err(Errcode::ContainerError("Unprivileged namespaces disabled".to_string()))
        }
    }


}

fn check_binary(arg: &String, name: &str) -> Result<PathBuf, Errcode> {
    if arg.is_empty() {
        match which(name) {
            Ok(path) => return Ok(path),
            Err(e) => {
                log::error!("Can not find {} in PATH, please be sure that is available or install it", name);
                return Err(Errcode::ContainerError(format!("Can not find {} in PATH: {}", name, e)));
            }
        };
    } else {
        let path = PathBuf::from(arg);
        if let Err(e) = stat(&path) {
            log::error!("Can not stat {} at {}: {}", name, arg, e);
            return Err(Errcode::ContainerError(format!("Can not find {} at path {}: {}", name, arg, e)));
        } else {
            return Ok(path);
        }
    }
}
