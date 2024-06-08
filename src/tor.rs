use crate::errors::Errcode;

use std::process::{Child, Command, Stdio};
use std::fs::File;
use std::io::Write;
use std::ops::Drop;
use std::path::Path;
use std::sync::{Arc, Mutex};

use which::which;

pub struct TorProcess {
    process: Child,
}

pub type TorWrapper = Arc<Mutex<TorProcess>>;

impl TorProcess {
    pub fn new(data_directory: &Path) -> Result<TorProcess, Errcode> {

        let tor_bin_path = which("tor").unwrap();
        if data_directory.is_relative() {
            return Err(Errcode::TorError(format!("Data directory not absolute: {:?}", data_directory)));
        }

        if !data_directory.exists() {
            std::fs::create_dir_all(data_directory).map_err(|e| {
                log::error!("Can not create tor data directory: {e}");
                // return Err(Errcode::TorError("porocido".to_string()));
            });
        } else if data_directory.is_file() {
            return Err(Errcode::TorError(format!("Tor data dir {:?} exists as file", data_directory)));
        }

        let torrc_contents = "VirtualAddrNetwork 10.40.0.0/16\n
                            AutomapHostsOnResolve 1\n
                            TransPort 10.40.50.10:9050\n
                            DNSPort 10.40.50.10:5353\n
                            SocksPort 9040\n
                            RunAsDaemon 1\n
                            DataDirectory /var/lib/tor\n";

        // Write torrc to file 
        let torrc = data_directory.join("torrc");
        if !torrc.exists() {
            let mut default_torrc = File::create(&torrc).unwrap();
            default_torrc.write_all(torrc_contents.as_bytes()).unwrap();
        }

        let mut process = Command::new(tor_bin_path.as_os_str())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .arg("-f")
            .arg(torrc)
            .spawn()
            .unwrap();

        Ok(  TorProcess {
            process,
        })
    }

}

impl Drop for TorProcess {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
