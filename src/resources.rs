#![allow(dead_code)]
use rlimit::{setrlimit, Resource};
use cgroups_rs::cgroup_builder::CgroupBuilder;
use cgroups_rs::hierarchies::V2;
use cgroups_rs::{MaxValue, CgroupPid};
use nix::unistd::Pid;
use std::fs::{canonicalize, remove_dir};
use std::convert::TryInto;

use crate::errors::Errcode;

const KMEM_LIMIT: i64 = 1024 * 1024 * 1024;
const MEM_LIMIT: i64 = KMEM_LIMIT;
const MAX_PID: MaxValue = MaxValue::Value(256);
const NOFILE_RLIMIT: u64 = 256;

pub fn restrict_resources(hostname: &String, pid: Pid) -> Result<(), Errcode>{
    log::debug!("Restricting resources for hostname {}", hostname);
    // Cgroups
    let cgs = CgroupBuilder::new(hostname)
        // Allocate less CPU time than other processes
        .cpu().shares(256).done()
        // Limiting the memory usage to 1 GiB
        // The user can limit it to less than this, never increase above 1Gib
        .memory().kernel_memory_limit(KMEM_LIMIT).memory_hard_limit(MEM_LIMIT).done()
        // This process can only create a maximum of 64 child processes
        .pid().maximum_number_of_processes(MAX_PID).done()
        // Give an access priority to block IO lower than the system
        .blkio().weight(50).done()
        .build(Box::new(V2::new()));
    // We apply the cgroups rules to the child process we just created
    let pid : u64 = pid.as_raw().try_into().unwrap();
    if let Err(e) = cgs.add_task(CgroupPid::from(pid)) {
        return Err(Errcode::ResourcesError(format!("Error during cgroups conf for PID {pid}: {e}")));
    };
    // Rlimit
    // Can create only 64 file descriptors
    if let Err(e) = setrlimit(Resource::NOFILE, NOFILE_RLIMIT, NOFILE_RLIMIT){
        return Err(Errcode::ResourcesError(format!("Cgroups: setrlimit returned error {e}")));
    }
    Ok(())
}

pub fn clean_cgroups(hostname: &String) -> Result<(), Errcode>{
    log::debug!("Cleaning cgroups");
    match canonicalize(format!("/sys/fs/cgroup/{}/", hostname)){
        Ok(d) => {
            if let Err(e) = remove_dir(&d) {
                return Err(Errcode::ResourcesError(format!("Error while trying to delete dir {}: {}", d.to_str().unwrap(), e)));
            }
        },
        Err(e) => {
            log::error!("Error while canonicalize path: {}", e);
            return Err(Errcode::ResourcesError(format!("Error while canonicalize path: {}", e)));
        }
    }
    Ok(())
}
