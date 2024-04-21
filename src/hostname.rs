use nix::unistd::sethostname;

use crate::errors::Errcode;
use crate::utils::generate_random_str;

// TODO catch RNG init error?
pub fn generate_hostname() -> Result<String, Errcode> {
    let base_str: String = "orjail".to_string();
    let rand_str: String = generate_random_str(4);
    Ok(format!("{}-{}", base_str, rand_str))
}

// Uses the sethostname syscall to set the hostname inside the container
pub fn set_container_hostname(hostname: &String) -> Result<(), Errcode> {
    match sethostname(hostname) {
        Ok(_) => {
            log::debug!("Container hostname is now: {}", hostname);
            Ok(())
        }
        Err(e) => {
            log::error!("Cannot set container hostname {}: {:?}", hostname, e);
            Err(Errcode::HostnameError(0))
        }
    }
}
