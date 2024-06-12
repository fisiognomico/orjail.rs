use std::process::exit;
use thiserror::Error;

// Allows to display a variant with the format {:?}
#[derive(Debug, Error)]
pub enum Errcode{
    #[error("argument {0} is not valid")]
    ArgumentInvalid(&'static str),
    #[error("Error in setting capabilities {0}")]
    Capabilities(String),
    #[error("Error in container creation {0}")]
    ContainerError(String),
    #[error("Error in child creation {0}")]
    ChildProcessError(u8),
    #[error("Error while setting hostname {0}")]
    HostnameError(u8),
    #[error("Error while mounting container fs {0}")]
    MountsError(String),
    #[error("Error in namespace creation {0}")]
    NamespacesError(String),
    #[error("Error in network creation: {0}")]
    NetworkError(String),
    #[error("Functionality not supported")]
    NotSupported(u8),
    #[error("Not possible to define cgroups: {0}")]
    ResourcesError(String),
    #[error("Error in Slirp Process creation: {0}")]
    SlirpError(String),
    #[error("Error in IPC socket communication: {0}")]
    SocketError(String),
    #[error("Unable to define container syscalls: {0}")]
    SyscallsError(String),
    #[error("Error with tor instance {0}")]
    TorError(String),
}

impl Errcode{
    // Translate an Errcode::X into a number to return (the Unix way)
    pub fn get_retcode(&self) -> i32 {
        1 // Everything != 0 will be treated as an error
    }
}

impl From<rtnetlink::Error> for Errcode {
    fn from(err: rtnetlink::Error) -> Self {
        Errcode::NetworkError(err.to_string())
    }
}

impl From<std::io::Error> for Errcode {
    fn from(err: std::io::Error) -> Self {
        Errcode::NetworkError(err.to_string())
    }
}

impl From<std::net::AddrParseError> for Errcode {
    fn from(err: std::net::AddrParseError) -> Self {
        Errcode::NetworkError(err.to_string())
    }
}

pub fn exit_with_retcode(res: Result<(), Errcode>) {
    match res {
        Ok(_) => {
            log::debug!("Exit without any error, returning 0");
            exit(0);
        },

        Err(e) => {
            let retcode = e.get_retcode();
            log::error!("Error on exit:\n\t{}\n\tReturning {}", e, retcode);
            exit(retcode);
        }
    }
}

pub fn exit_with_errcode(res: Errcode) {
    log::error!("Exiting for error {res}");
    exit(res.get_retcode());
}
