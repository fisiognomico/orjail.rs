#![allow(dead_code)]
use crate::errors::Errcode;
use crate::namespaces::{open_namespace, run_in_namespace};

use futures::TryStreamExt;
use rtnetlink::{new_connection, AddressHandle, Handle};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

static NETNS: &str = "/var/run/netns/";

pub async fn prepare_net(ns_name: &String, veth_ip: &str, veth_2_ip: &str, subnet: u8) -> Result<(u32, u32), Errcode> {
    let (connection, _handle, _) = new_connection()?;
    tokio::spawn(connection);

    let ns_fd = open_namespace(&ns_name).await?;

    let (veth_idx, veth_2_idx) = create_veth_pair(&ns_name, veth_ip, veth_2_ip, subnet).await?;

    // moved to namespace crate
    join_veth_to_ns_fd(veth_2_idx, ns_fd).await?;
    run_in_namespace(&ns_name, veth_ip, veth_2_ip).await?;
    // Try to set lo up in namespace set_lo_up(ns_name).await?;
    Ok((veth_idx, veth_2_idx))
}

async fn get_bridge_idx(handle: &Handle, bridge_name: String) -> Result<u32, Errcode> {
    let bridge_idx = handle.link().get().match_name(bridge_name.clone()).execute().try_next().await?
        .ok_or_else(|| Errcode::NetworkError(format!("Can not find bridge index of {}", bridge_name)))?
        .header.index;

    Ok(bridge_idx)
}

async fn create_bridge(name: String, bridge_ip: &str, subnet: u8) -> Result<u32, Errcode> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Create a bridge
    handle.link().add().bridge(name.clone()).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Can not create bridge {}: {}", name, e))
        })?;

    // Bring up the bridge
    let bridge_idx = handle.link().get().match_name(name.clone()).execute().try_next().await?
            .ok_or_else(|| Errcode::NetworkError(format!("Failed to get idx for bridge {}", name)))?
            .header.index;

    // Add ip to the bridge
    let bridge_addr = IpAddr::V4(Ipv4Addr::from_str(bridge_ip)?);
    AddressHandle::new(handle.clone()).add(bridge_idx, bridge_addr, subnet).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Can not add ip {} to bridge {}: {}", bridge_ip, name, e))
        })?;

    // Set bridge up
    handle.link().set(bridge_idx).up().execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Can not set bridge {} up: {}", name, e))
        })?;

    Ok(bridge_idx)
}

async fn create_veth_pair(veth_name: &String, veth_addr: &str, veth2_addr: &str, subnet: u8) -> Result<(u32, u32), Errcode> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    let veth = format!("{}", veth_name);
    let veth_2 = format!("{}_peer", veth);

    handle.link().add().veth(veth.clone(), veth_2.clone()).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Can not create veth interfaces: {}", e))
        })?;

    let veth_idx = handle.link().get().match_name(veth.clone()).execute().try_next().await?
        .ok_or_else(|| Errcode::NetworkError(format!("Failed to get index for {}", veth)))?
        .header.index;

    let veth_2_idx = handle.link().get().match_name(veth_2.clone()).execute().try_next().await?
        .ok_or_else(|| Errcode::NetworkError(format!("Failed to get index for {}", veth_2)))?
        .header.index;

    // set master veth up
    handle.link().set(veth_idx).up().execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting veth {} up failed: {}", veth, e));
    }).unwrap();

    let veth_ip_addr = IpAddr::V4(Ipv4Addr::from_str(veth_addr)?);
    AddressHandle::new(handle.clone()).add(veth_idx, veth_ip_addr, subnet).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting addr {} to veth {} failed: {}", veth_addr, veth, e));
    }).unwrap();

    let veth2_ip_addr = IpAddr::V4(Ipv4Addr::from_str(veth2_addr)?);
    AddressHandle::new(handle.clone()).add(veth_2_idx, veth2_ip_addr, subnet).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting addr {} to veth {} failed: {}", veth2_addr, veth_2, e));
    }).unwrap();

    // set interface veth2 up
    handle.link().set(veth_2_idx).up().execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting veth with idx {} up failed: {}", veth_idx, e));
    }).unwrap();

    // set lo interface up
    // TODO move to another function called in the namespace
    // let lo_idx = handle.link().get().match_name("lo".to_string()).execute().try_next().await?
    //             .ok_or_else(|| Errcode::NetworkError(format!("Can not find lo interface for namespace {}", ns_ip)))?
    //             .header.index;

    // handle.link().set(lo_idx).up().execute().await
    //     .map_err(|e| {Errcode::NetworkError(format!("Can not set lo interface up: {}", e))
    // });

    Ok((veth_idx, veth_2_idx))

}

pub async fn join_veth_to_ns_pid(veth_idx: u32, pid: u32) -> Result<(), Errcode> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    handle.link().set(veth_idx).setns_by_pid(pid).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Set veth {} to process {} failed: {}", veth_idx, pid, e))
    })?;

    Ok(())
}

pub async fn join_veth_to_ns_fd(veth_idx: u32, fd: i32) -> Result<(), Errcode> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    handle.link().set(veth_idx).setns_by_fd(fd).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Set veth {} to fd {} failed: {}", veth_idx, fd, e))
    })?;

    Ok(())
}

// TODO continue configure address interface definition
pub async fn setup_veth_peer(veth_idx: u32, ns_ip: &String, subnet: u8) -> Result<(), Errcode> {
    let (_connection, handle, _) = new_connection()?;

    let veth2_addr = IpAddr::V4(Ipv4Addr::from_str(ns_ip)?);

    // Setup veth peer interface address
    AddressHandle::new(handle.clone()).add(veth_idx, veth2_addr, subnet).execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting addr {} to veth with index {} failed: {}", ns_ip, veth_idx, e));
    }).unwrap();

    // set interface veth2 up
    handle.link().set(veth_idx).up().execute().await
        .map_err(|e| {
            Errcode::NetworkError(format!("Setting veth with idx {} up failed: {}", veth_idx, e));
    }).unwrap();

    // set lo interface up
    let lo_idx = handle.link().get().match_name("lo".to_string()).execute().try_next().await?
                .ok_or_else(|| Errcode::NetworkError(format!("Can not find lo interface for namespace {}", ns_ip)))?
                .header.index;

    handle.link().set(lo_idx).up().execute().await
        .map_err(|e| {Errcode::NetworkError(format!("Can not set lo interface up: {}", e))
    }).unwrap();

    Ok(())
}
