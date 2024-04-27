use crate::errors::Errcode;
use crate::ipc::generate_socketpair;
use crate::hostname::generate_hostname;

use std::os::unix::io::RawFd;
use std::ffi::CString;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ContainerOpts{
    pub path:       CString,
    pub argv:       Vec<CString>,

    pub uid:        u32,
    pub real_uid:   u32,
    pub real_gid:   u32,
    pub mount_dir:  PathBuf,
    pub fd:         RawFd,
    pub hostname: String,
    pub addpaths: Vec<(PathBuf, PathBuf)>,
}

impl ContainerOpts{
    pub fn new(command: String, uid: u32, real_uid: u32, real_gid: u32, mount_dir: PathBuf, addpaths: Vec<(PathBuf, PathBuf)>) -> Result<(ContainerOpts, (RawFd, RawFd)), Errcode> {
        let argv: Vec<CString> = command.split_ascii_whitespace()
            .map(|s| CString::new(s).expect("Cannot read arg")).collect();
        let path = argv[0].clone();

        // TODO clean socket conf
        let sockets = generate_socketpair()?;

        Ok(
            (
                ContainerOpts {
                    path,
                    argv,
                    uid,
                    real_uid,
                    real_gid,
                    mount_dir,
                    fd: sockets.1.clone(),
                    hostname: generate_hostname()?,
                    addpaths,
                },
            sockets)
        )
    }
}
