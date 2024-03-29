use std::{
        os::unix::prelude::OsStrExt,
        path::Path,
    };

use alpm::{
        Alpm,
        Package,
    };


use crate::error::{
        Error,
        Result
    };

pub(crate) struct DbHandle {
    alpm_handle: Alpm,
}

impl DbHandle {
    pub(crate) fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let handle = match Alpm::new(
            root.as_ref().as_os_str().as_bytes(),
            root.as_ref().join("var/lib/pacman")
                .as_os_str().as_bytes())
        {
            Ok(handle) => handle,
            Err(e) => {
                log::error!("Failed to open pacman DB at root '{}': {}",
                root.as_ref().display(), e);
                return Err(Error::AlpmError(e))
            },
        };
        let content = match std::fs::read_to_string(
            "/etc/pacman.conf")
        {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to open pacman config: {}", e);
                return Err(Error::IoError(e))
            },
        };
        let config = crate::config::PacmanConfig::
            from_pacman_conf_content(&content)?;
        let _new_config = config.with_cusrepo(
            "arch_repo_builder_internal_do_not_use",
            "/srv/repo_builder/pkgs");
        let sig_level = handle.default_siglevel();
        for repo in config.repos.iter() {
            if let Err(e) = handle.register_syncdb(repo.name, sig_level){
                log::error!("Failed to register repo '{}': {}",
                                repo.name, e);
                return Err(Error::AlpmError(e))
            }
        }
        if handle.syncdbs().len() == 0 {
            log::error!("No DBs defined, please check your pacman config");
            return Err(Error::InvalidConfig)
        }
        Ok(DbHandle { alpm_handle: handle })
    }

    pub(super) fn find_satisfier<S: AsRef<str>>(&self, dep: S)
        -> Option<&Package>
    {
        let mut pkg_satisfier = None;
        for db in self.alpm_handle.syncdbs() {
            if let Ok(pkg) = db.pkg(dep.as_ref()) {
                return Some(&pkg)
            }
            if let Some(pkg) =
                db.pkgs().find_satisfier(dep.as_ref())
            {
                pkg_satisfier = Some(pkg)
            }
        }
        pkg_satisfier
    }

    pub(super) fn is_installed<S: AsRef<str>>(&self, pkg: S) -> bool {
        match self.alpm_handle.localdb().pkg(pkg.as_ref()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}