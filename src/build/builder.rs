use std::{path::{PathBuf, Path}, process::{Command, Child}, fs::{File, remove_dir_all, create_dir_all}, io::{Read, stdout, Write}};

use crate::{roots::OverlayRoot, identity::Identity, filesystem::remove_dir_recursively};

use super::{pkgbuild::{PKGBUILD, PKGBUILDs}, dir::BuildDir};

enum BuilderStatus {
    None,
    Extracting {
        child: Child
    },
    Extracted,
    Boostrapping {
        child: Child
    },
    Bootstrapped {
        root: OverlayRoot
    },
    Building {
        root: OverlayRoot,
        child: Child
    },
    // Built OverlayRoot),
}

struct Builder<'a> {
    pkgbuild: &'a PKGBUILD,
    builddir: BuildDir,
    temp_pkgdir: PathBuf,
    command: Command,
    _root: OverlayRoot,
    tries: usize,
    status: BuilderStatus,
}

impl <'a> Builder<'a> {
    fn from_pkgbuild(
        pkgbuild: &'a PKGBUILD, actual_identity: &Identity, nonet: bool
    ) 
        -> Result<Self, ()> 
    {
        let builddir = BuildDir::new(&pkgbuild.base)?;
        let root = pkgbuild.get_overlay_root(
            actual_identity, nonet)?;
        let temp_pkgdir = pkgbuild.get_temp_pkgdir()?;
        let command = pkgbuild.get_build_command(
            actual_identity, &root, &temp_pkgdir)?;
        let status = if pkgbuild.extracted {
            BuilderStatus::Extracted
        } else {
            BuilderStatus::None
        };
        Ok(Self {
            pkgbuild,
            builddir,
            temp_pkgdir,
            command,
            _root: root,
            tries: 0,
            status,
        })
    }
}


fn file_to_stdout<P: AsRef<Path>>(file: P) -> Result<(), ()> {
    let file_p = file.as_ref();
    let mut file = match File::open(&file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open '{}': {}", file_p.display(), e);
            return Err(())
        },
    };
    let mut buffer = vec![0; 4096];
    loop {
        match file.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    return Ok(())
                }
                if let Err(e) = stdout().write_all(&buffer[0..size]) 
                {
                    eprintln!("Failed to write log content to stdout: {}", e);
                    return Err(())
                }
            },
            Err(e) => {
                eprintln!("Failed to read from '{}': {}", file_p.display(), e);
                return Err(())
            },
        }

    }
}

fn prepare_pkgdir() -> Result<(), ()> {
    let _ = remove_dir_all("pkgs/updated");
    let _ = remove_dir_all("pkgs/latest");
    if let Err(e) = create_dir_all("pkgs/updated") {
        eprintln!("Failed to create pkgs/updated: {}", e);
        return Err(())
    }
    if let Err(e) = create_dir_all("pkgs/latest") {
        eprintln!("Failed to create pkgs/latest: {}", e);
        return Err(())
    }
    Ok(())
}

struct Builders<'a> {
    pkgbuilds: &'a PKGBUILDs,
    builders: Vec<Builder<'a>>,
}

impl<'a> Builders<'a> {
    const BUILD_MAX_TRIES: usize = 3;
    // fn wait_noop(&mut self, actual_identity: &Identity, sign: Option<&str>) 
    //     -> bool 
    // {
    //     let mut bad = false;
    //     loop {
    //         let mut finished = None;
    //         for (id, builder) in 
    //             self.0.iter_mut().enumerate() 
    //         {
    //             match builder.child.try_wait() {
    //                 Ok(status) => match status {
    //                     Some(_) => {
    //                         finished = Some(id);
    //                         break
    //                     },
    //                     None => continue,
    //                 }
    //                 Err(e) => { // Kill bad child
    //                     eprintln!("Failed to wait for child: {}", e);
    //                     if let Err(e) = builder.child.kill() {
    //                         eprintln!("Failed to kill child: {}", e);
    //                     }
    //                     finished = Some(id);
    //                     bad = true;
    //                     break
    //                 },
    //             };
    //         }
    //         let mut builder = match finished {
    //             Some(finished) => self.0.swap_remove(finished),
    //             None => break, // No child waitable
    //         };
    //         println!("Log of building '{}':", &builder.pkgbuild.pkgid);
    //         if file_to_stdout(&builder.log_path).is_err() {
    //             println!("Warning: failed to read log to stdout, \
    //                 you could still manually check the log file '{}'",
    //                 builder.log_path.display())
    //         }
    //         println!("End of Log of building '{}'", &builder.pkgbuild.pkgid);
    //         if builder.pkgbuild.remove_build().is_err() {
    //             eprintln!("Failed to remove build dir");
    //             bad = true;
    //         }
    //         match builder.child.wait() {
    //             Ok(status) => {
    //                 match status.code() {
    //                     Some(code) => {
    //                         if code == 0 {
    //                             if builder.pkgbuild.build_finish(
    //                                 actual_identity,
    //                                 &builder.temp_pkgdir, sign).is_err() 
    //                             {
    //                                 eprintln!("Failed to finish build for {}",
    //                                     &builder.pkgbuild.base);
    //                                 bad = true
    //                             }
    //                             continue
    //                         }
    //                         eprintln!("Bad return from builder child: {}",
    //                                     code);
    //                     },
    //                     None => eprintln!("Failed to get return code from\
    //                             builder child"),
    //                 }
    //             },
    //             Err(e) => {
    //                 eprintln!("Failed to get child output: {}", e);
    //                 bad = true;
    //             },
    //         };
    //         if builder.tries >= Self::BUILD_MAX_TRIES {
    //             eprintln!("Max retries met for building {}, giving up",
    //                 &builder.pkgbuild.base);
    //             if let Err(e) = remove_dir_all(
    //                 &builder.temp_pkgdir
    //             ) {
    //                 eprintln!("Failed to remove temp pkg dir for failed \
    //                         build: {}", e);
    //                 bad = true
    //             }
    //             continue
    //         }
    //         if builder.pkgbuild.extract_source(actual_identity).is_err() {
    //             eprintln!("Failed to re-extract source to rebuild");
    //             bad = true;
    //             continue
    //         }
    //         let log_file = match File::create(&builder.log_path) {
    //             Ok(log_file) => log_file,
    //             Err(e) => {
    //                 eprintln!("Failed to create log file: {}", e);
    //                 continue
    //             },
    //         };
    //         builder.tries += 1;
    //         builder.child = match builder.command.stdout(log_file).spawn() {
    //             Ok(child) => child,
    //             Err(e) => {
    //                 eprintln!("Failed to spawn child: {}", e);
    //                 bad = true;
    //                 continue
    //             },
    //         };
    //         self.0.push(builder)
    //     }
    //     bad
    // }

    fn from_pkgbuilds(
        pkgbuilds: &'a PKGBUILDs, actual_identity: &Identity, 
        nonet: bool, sign: Option<&str>
    ) -> Result<Self, ()> 
    {
        prepare_pkgdir()?;
        let mut bad = false;
        // let cpuinfo = procfs::CpuInfo::new().or_else(|e|{
        //     eprintln!("Failed to get cpuinfo: {}", e);
        //     Err(())
        // })?;
        // let cores = cpuinfo.num_cores();
        let mut builders = vec![];
        for pkgbuild in pkgbuilds.0.iter() {
            if ! pkgbuild.need_build {
                continue
            }
            match Builder::from_pkgbuild(pkgbuild, actual_identity, nonet) {
                Ok(builder) => builders.push(builder),
                Err(_) => return Err(()),
            }
        }
        Ok(Self {
            pkgbuilds,
            builders,
        })
    }

    fn finish(&self, actual_identity: &Identity, sign: Option<&str>) {
        // let thread_cleaner =
        //     thread::spawn(|| Self::remove_builddir());
        // println!("Finishing building '{}'", &self.pkgid);
        // if self.pkgdir.exists() {
        //     if let Err(e) = remove_dir_all(&self.pkgdir) {
        //         eprintln!("Failed to remove existing pkgdir: {}", e);
        //         return Err(())
        //     }
        // }
        // if let Some(key) = sign {
        //     Self::sign_pkgs(actual_identity, temp_pkgdir, key)?;
        // }
        // if let Err(e) = rename(&temp_pkgdir, &self.pkgdir) {
        //     eprintln!("Failed to rename temp pkgdir '{}' to persistent pkgdir \
        //         '{}': {}", temp_pkgdir.display(), self.pkgdir.display(), e);
        //     return Err(())
        // }
        // self.link_pkgs()?;
        // println!("Finished building '{}'", &self.pkgid);
        // let _ = thread_cleaner.join()
        //     .expect("Failed to join cleaner thread");
        // Ok(())
    }
}