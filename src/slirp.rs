use crate::errors::Errcode;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use nix::unistd::Pid;

pub struct SlirpProcess {
    pub process: Child,
}

pub type SlirpWrapper = Arc<Mutex<SlirpProcess>>;

impl SlirpProcess {
    // TODO continue from here to copy tor impl.
    pub fn new(pid: Pid, slirp_path: &PathBuf) -> Result<SlirpProcess, Errcode> {
        let pid_str = format!("{}", pid.as_raw());
        // TODO catch error when spawning slirp4netns
        let slirp_process = Command::new(slirp_path.as_os_str())
                        .args(["--configure", "--mtu=65520", "--disable-host-loopback", &pid_str, "tap0"])
                        .stdout(Stdio::null())
                        .spawn();

        match slirp_process {
            Ok(child) => Ok(SlirpProcess { process: child }),
            Err(e) => {
                log::error!("Error while spawning slirp: {e}");
                return Err(Errcode::SlirpError(format!("Error while spawning slirp: {}", e)));
            }
        }
    }
}

impl Drop for SlirpProcess {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}


