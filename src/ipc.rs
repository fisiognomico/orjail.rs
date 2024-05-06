use crate::errors::Errcode;

use std::os::unix::io::RawFd;
use nix::sys::socket::{socketpair, AddressFamily, SockType, SockFlag, send, MsgFlags, recv};

pub fn generate_socketpair() -> Result<(RawFd, RawFd), Errcode> {
    match socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC)
        {
            Ok(res) => Ok(res),
            Err(e) => Err(Errcode::SocketError(format!("Can not generate socket pair: {}", e)))
    }
}

pub fn send_boolean(fd: RawFd, boolean: bool) -> Result<(), Errcode> {
    let data: [u8; 1] = [boolean.into()];
    if let Err(e) = send(fd, &data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not send {} to fd {}: {}", boolean, fd.to_be(), e)));
    };
    Ok(())
}

pub fn recv_boolean(fd: RawFd) -> Result<bool, Errcode> {
    let mut data: [u8; 1] = [0];
    if let Err(e) = recv(fd, &mut data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not read value from fd {}: {}", fd.to_be(), e)));
    }
    Ok(data[0] == 1)
}

pub fn send_u32(fd: RawFd, value: u32) -> Result<(), Errcode> {
    let data: &[u8; 4] = &value.to_be_bytes();
    if let Err(e) = send(fd, data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not send {} to fd {}: {}", value, fd.to_be(), e)));
    };
    Ok(())
}

pub fn recv_u32(fd: RawFd) -> Result<u32, Errcode> {
    let mut data: [u8; 4] = [0; 4];
    if let Err(e) = recv(fd, &mut data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not read value from fd {}: {}", fd.to_be(), e)));
    }
    Ok(u32::from_be_bytes(data))
}
