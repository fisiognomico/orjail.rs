use capctl::caps::{Cap, FullCapState};
use crate::errors::Errcode;
const CAPABILITIES_DROP: [Cap; 21] = [
    Cap::AUDIT_CONTROL,
    Cap::AUDIT_READ,
    Cap::AUDIT_WRITE,
    Cap::BLOCK_SUSPEND,
    Cap::DAC_READ_SEARCH,
    Cap::DAC_OVERRIDE,
    Cap::FSETID,
    Cap::IPC_LOCK,
    Cap::MAC_ADMIN,
    Cap::MAC_OVERRIDE,
    Cap::MKNOD,
    Cap::SETFCAP,
    Cap::SYSLOG,
    Cap::SYS_ADMIN,
    Cap::SYS_BOOT,
    Cap::SYS_MODULE,
    Cap::SYS_NICE,
    Cap::SYS_RAWIO,
    Cap::SYS_RESOURCE,
    Cap::SYS_TIME,
    Cap::WAKE_ALARM
];

pub fn setcapabilities() -> Result<(), Errcode> {
    log::debug!("Clearing unwanted capabilities");
    // if let Ok(mut caps) = FullCapState::get_current() {
    match FullCapState::get_current() {
        Ok(mut caps) => {
            caps.bounding.drop_all(CAPABILITIES_DROP.iter().map(|&cap| cap));
            caps.inheritable.drop_all(CAPABILITIES_DROP.iter().map(|&cap| cap));
            Ok(())
        },
        Err(e) => Err(Errcode::Capabilities(format!("Error while trying to get an handler to thread capabilities: {e}"))),
    }
}
