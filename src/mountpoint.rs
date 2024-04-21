// Chroot mount point management
use crate::errors::Errcode;
use crate::utils::generate_random_str;

use std::path::PathBuf;
use std::fs::{create_dir_all, remove_dir};
use nix::mount::{mount, MsFlags, umount2, MntFlags};
use nix::unistd::{pivot_root, chdir};

pub fn set_container_mountpoint(mount_dir: &PathBuf, addpaths: &Vec<(PathBuf, PathBuf)>) -> Result<(), Errcode> {
    log::debug!("setting mount points");
    // Setting the mount flags
    // MS_PRIVATE prevents any mount/unmount operation to be propagated
    // MS_REC applies it recursively
    // This will remount the root of our filesystem to avoid the propagation
    // of any new mount operation
    // TODO bubblewrap performs it on the "old_mount" directory, check how to improve it
    mount_directory(None, &PathBuf::from("/"), vec![MsFlags::MS_REC, MsFlags::MS_PRIVATE])?;

    // Create the target directory
    // TODO move this suffix to a global variable with the lifetime of the program
    // ie 'static, so that the clean_mounts function can unmount and clean it
    let random_suffix =generate_random_str(10);
    let new_root = PathBuf::from(format!("/tmp/orjail.{}", random_suffix));
    log::debug!("Mounting new root: {}", new_root.as_path().to_str().unwrap());
    create_directory(&new_root)?;
    // MS_BIND to create a bind mount that is visible outside the mounted filesystem
    mount_directory(Some(&mount_dir), &new_root, vec![MsFlags::MS_BIND, MsFlags::MS_PRIVATE])?;

    // Mount additional paths present in the configuration
    for (inpath, mntpath) in addpaths.iter(){
        let outpath = new_root.join(mntpath);
        create_directory(&outpath)?;
        mount_directory(Some(inpath), &outpath, vec![MsFlags::MS_PRIVATE, MsFlags::MS_BIND])?;
    }
    // MAGIC: now we set the /tmp/orjail. directory as the new / root filesystem, and we will
    // move the old / root filesystem in a new directory in /tmp/orjail./oldroot.
    // We will then take the hurdle of unmounting it to avoid that the container 
    // has access to the host filesystem
    let old_root_name = format!("oldroot.{}", generate_random_str(6));
    let old_root = new_root.join(PathBuf::from(old_root_name.clone()));
    create_directory(&old_root)?;
    log::debug!("Pivoting root to {}", old_root.as_path().to_str().unwrap());
    if let Err(_) = pivot_root(&new_root, &old_root) {
        return Err(Errcode::MountsError(4));
    }

    // Now we unmount the old root, and we also take care of being out of
    // the directory that we are unmounting
    let root_inside_container = PathBuf::from(format!("/{}", old_root_name));
    if let Err(e) = chdir(&PathBuf::from("/")) {
        log::error!("Cannot change cwd to root: {}", e);
        return Err(Errcode::MountsError(5));
    }
    unmount_path(&root_inside_container)?;
    delete_dir(&root_inside_container)?;

    Ok(())
}

pub fn mount_directory(path: Option<&PathBuf>, mount_point: &PathBuf, flags: Vec<MsFlags>) -> Result<(), Errcode> {
    // In theory we can also accept an empty flags vector
    let mut ms_flags = MsFlags::empty();
    for f in flags.iter() {
        ms_flags.insert(*f);
    }

    // Call the mount syscall, check error
    match mount::<PathBuf, PathBuf, PathBuf, PathBuf>(path, mount_point, None, ms_flags, None) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(p) = path {
                log::error!("Cannot mount {} to {}: {}",
                p.to_str().unwrap(), mount_point.to_str().unwrap(), e);
            } else {
                log::error!("Cannot remount {}: {}",
                mount_point.to_str().unwrap(), e);
            }
            Err(Errcode::MountsError(3))
        }
    }
}


pub fn create_directory(path: &PathBuf) -> Result<(), Errcode> {
    match create_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Cannot create directory {} : {}", path.to_str().unwrap(), e);
            Err(Errcode::MountsError(2))
        }
    }
}

pub fn unmount_path(path: &PathBuf) -> Result<(), Errcode> {
    match umount2(path, MntFlags::MNT_DETACH) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Unable to detach directory {}: {}", path.to_str().unwrap(), e);
            Err(Errcode::MountsError(1))
        }
    }
}

pub fn delete_dir(path: &PathBuf) -> Result<(), Errcode> {
    match remove_dir(path.as_path()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Unable to delete directory {} : {}", path.to_str().unwrap(), e);
            Err(Errcode::MountsError(1))
        }
    }
}

pub fn clean_mounts(rootpath: &PathBuf) -> Result<(), Errcode> {
    // TODO complete this function, in order to do it we need to keep track
    // of the random suffix of the root mountpoint
    // unmount_path(&rootpath);
    Ok(())
}