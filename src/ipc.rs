#![allow(dead_code)]
use crate::errors::Errcode;

use std::os::unix::io::{AsRawFd, OwnedFd};
use nix::sys::socket::{socketpair, AddressFamily, SockType, SockFlag, send, MsgFlags, recv};

pub fn generate_socketpair() -> Result<(OwnedFd, OwnedFd), Errcode> {
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

pub fn send_boolean(fd: OwnedFd, boolean: bool) -> Result<(), Errcode> {
    let data: [u8; 1] = [boolean.into()];
    let raw_fd = fd.as_raw_fd();
    if let Err(e) = send(raw_fd, &data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not send {} to fd {}: {}", boolean, raw_fd.to_be(), e)));
    };
    Ok(())
}

pub fn recv_boolean(fd: OwnedFd) -> Result<bool, Errcode> {
    let mut data: [u8; 1] = [0];
    let raw_fd = fd.as_raw_fd();
    if let Err(e) = recv(raw_fd, &mut data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not read value from fd {}: {}", raw_fd.to_be(), e)));
    }
    Ok(data[0] == 1)
}

pub fn send_u32(fd: OwnedFd, value: u32) -> Result<(), Errcode> {
    let data: &[u8; 4] = &value.to_be_bytes();
    let raw_fd = fd.as_raw_fd();
    if let Err(e) = send(raw_fd, data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not send {} to fd {}: {}", value, raw_fd.to_be(), e)));
    };
    Ok(())
}

pub fn recv_u32(fd: OwnedFd) -> Result<u32, Errcode> {
    let mut data: [u8; 4] = [0; 4];
    let raw_fd = fd.as_raw_fd();
    if let Err(e) = recv(raw_fd, &mut data, MsgFlags::empty()) {
        return Err(Errcode::SocketError(format!("Can not read value from fd {}: {}", raw_fd.to_be(), e)));
    }
    Ok(u32::from_be_bytes(data))
}
