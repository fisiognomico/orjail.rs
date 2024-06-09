use crate::errors::Errcode;
use crate::ipc::generate_socketpair;
use crate::hostname::generate_hostname;
use crate::slirp::{SlirpProcess, SlirpWrapper};
use crate::tor::{TorProcess, TorWrapper};

use nix::unistd::Pid;
use std::os::unix::io::RawFd;
use std::ffi::CString;
use std::path::PathBuf;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ContainerOpts{
    pub path:       CString,
    pub argv:       Vec<CString>,

    pub uid:        u32,
    pub real_uid:   u32,
    pub real_gid:   u32,
    pub mount_dir:  PathBuf,
    pub hostname: String,
    pub addpaths: Vec<(PathBuf, PathBuf)>,
    pub slirp_process: Option<SlirpWrapper>,
    pub tor_process: Option<TorWrapper>,
}

impl ContainerOpts{
    pub fn new(command: String, uid: u32, real_uid: u32, real_gid: u32, mount_dir: PathBuf, addpaths: Vec<(PathBuf, PathBuf)>) -> Result<ContainerOpts, Errcode> {
        let argv: Vec<CString> = command.split_ascii_whitespace()
            .map(|s| CString::new(s).expect("Cannot read arg")).collect();
        let path = argv[0].clone();

        // TODO clean socket conf
        let sockets = generate_socketpair()?;

        Ok( ContainerOpts {
                    path,
                    argv,
                    uid,
                    real_uid,
                    real_gid,
                    mount_dir,
                    hostname: generate_hostname()?,
                    addpaths,
                    slirp_process: None,
                    tor_process: None,
        })
    }

    pub fn spawn_slirp(&mut self, pid: Pid) {
        self.slirp_process = Some(Arc::new(Mutex::new(SlirpProcess::new(pid).unwrap())));
    }

    pub fn spawn_tor(&mut self) {
        // TODO it should be in the configuration
        let tor_path = Path::new("/tmp/tor");
        self.tor_process = Some(Arc::new(Mutex::new(TorProcess::new(tor_path).unwrap())));
    }

}
