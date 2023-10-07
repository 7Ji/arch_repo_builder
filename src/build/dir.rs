use std::{path::{PathBuf, Path}, ffi::OsStr, fs::{create_dir_all, File}};

use rand::Rng;

pub(super) struct BuildDir {
    path: PathBuf,
    log_path: PathBuf,
}

impl BuildDir {
    pub(super) fn new<S: AsRef<OsStr>>(name: S) -> Result<Self, ()> {
        let path = PathBuf::from("build").join(name.as_ref());
        if path.exists() {
            if ! path.is_dir() {
                eprintln!("Existing path for build dir is not a dir");
                return Err(())
            }
        } else {
            create_dir_all(&path).or_else(|e|{
                eprintln!("Failed to create build dir: {}", e);
                Err(())
            })?;
        }
        let log_path = path.clone();
        Ok(Self { path, log_path })
    }

    fn fill_log_path(&mut self) -> Result<(), ()> {
        let mut log_name = String::from("log");
        self.log_path.push(&log_name);
        let mut i = 0;
        loop {
            if ! self.log_path.exists() {
                return Ok(())
            }
            i += 1;
            if i > 1000 {
                eprintln!("Failed to get valid log name after 1000 tries");
                return Err(())
            }
            if ! self.log_path.pop() {
                eprintln!("Failed to pop last part from log path");
                return Err(())
            }
            log_name.shrink_to(3);
            for char in rand::thread_rng().sample_iter(
                rand::distributions::Alphanumeric).take(7) 
            {
                log_name.push(char::from(char))
            }
            self.log_path.push(&log_name);
        }
    }

    pub(super) fn finish(&mut self) -> Result<(), ()> {
        self.fill_log_path()
    }

    pub(super) fn get_log_file(&self) -> Result<File, ()> {
        File::create(&self.log_path).or_else(|e|{
            eprintln!("Failed to create log file at '{}': {}", 
                self.log_path.display(), e);
            Err(())
        })
    }
}

impl Drop for BuildDir {
    fn drop(&mut self) {
        if crate::filesystem::remove_dir_all_try_best(&self.path).is_err() {
            eprintln!("Warning: failed to remove build dir '{}'", 
                self.path.display())
        }
    }
}