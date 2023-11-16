use std::{path::{Path, PathBuf}, fs::{read_dir, remove_dir, remove_file, remove_dir_all, File, create_dir_all, create_dir}, io::{stdout, Read, Write}, os::unix::fs::chown};


// build/*/pkg being 0111 would cause remove_dir_all() to fail, in this case
// we use our own implementation
pub(crate) fn remove_dir_recursively<P: AsRef<Path>>(dir: P) 
    -> Result<(), std::io::Error>
{
    for entry in read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_symlink() && path.is_dir() {
            let er = 
                remove_dir_recursively(&path);
            match remove_dir(&path) {
                Ok(_) => (),
                Err(e) => {
                    log::error!(
                        "Failed to remove subdir '{}' recursively: {}", 
                        path.display(), e);
                    if let Err(e) = er {
                        log::error!("Subdir failure: {}", e)
                    }
                    return Err(e);
                },
            }
        } else {
            remove_file(&path)?
        }
    }
    Ok(())
}


pub(crate) fn remove_dir_all_try_best<P: AsRef<Path>>(dir: P) 
    -> Result<(), ()>
{
    log::info!("Removing dir '{}' recursively...", dir.as_ref().display());
    match remove_dir_all(&dir) {
        Ok(_) => return Ok(()),
        Err(e) => {
            log::error!("Failed to remove dir '{}' recursively naively: {}", 
                dir.as_ref().display(), e);
        },
    }
    remove_dir_recursively(&dir).or_else(|e|{
        log::error!("Failed to remove dir '{}' recursively: {}", 
            dir.as_ref().display(), e);
        Err(())
    })?;
    remove_dir(&dir).or_else(|e|{
        log::error!("Failed to remove dir '{}' itself: {}",
            dir.as_ref().display(), e);
        Err(())
    })?;
    log::info!("Removed dir '{}' recursively", dir.as_ref().display());
    Ok(())

}

pub(crate) fn file_to_stdout<P: AsRef<Path>>(file: P) -> Result<(), ()> {
    let file_p = file.as_ref();
    let mut file = match File::open(&file) {
        Ok(file) => file,
        Err(e) => {
            log::error!("Failed to open '{}': {}", file_p.display(), e);
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
                    log::error!("Failed to write log content to stdout: {}", e);
                    return Err(())
                }
            },
            Err(e) => {
                log::error!("Failed to read from '{}': {}", file_p.display(), e);
                return Err(())
            },
        }
    }
}

pub(crate) fn prepare_updated_latest_dirs() -> Result<(), ()> {
    let mut bad = false;
    let dir = PathBuf::from("pkgs");
    for subdir in ["updated", "latest"] {
        let dir = dir.join(subdir);
        if dir.exists() && remove_dir_all_try_best(&dir).is_err(){
            bad = true
        }
        if let Err(e) = create_dir_all(&dir) {
            log::error!("Failed to create dir '{}': {}", dir.display(), e);
            bad = true
        }
    }
    if bad { Err(()) } else { Ok(()) }
}

pub(crate) fn create_dir_all_under_owned_by<P, Q>(
    path: P, parent: Q, uid: u32, gid: u32
) -> Result<(), ()> 
where
    P: AsRef<Path>, 
    Q: AsRef<Path>
{
    let mut path_buffer = parent.as_ref().to_owned();
    for component in path.as_ref().components() {
        path_buffer.push(component);
        if ! path_buffer.exists() {
            create_dir(&path_buffer).map_err(|e|
                log::error!("Failed to create dir '{}': {}", 
                path_buffer.display(), e))?;
        }
        chown(&path_buffer, Some(uid), Some(gid)).map_err(
            |e|log::error!("Failed to chown '{}' to {}:{}: {}",
            path_buffer.display(), uid, gid, e))?;
    }
    Ok(())
}