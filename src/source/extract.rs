use std::path::{
        Path,
        PathBuf,
    };

use xxhash_rust::xxh3::xxh3_64;

use crate::{
        error::Result,
        source::{
            VcsProtocol,
            Protocol,
            Source,
            IntegFile,
        }
    };

pub(crate) fn extract<P: AsRef<Path>>(dir: P, sources: &Vec<Source>) -> Result<()> {
    let rel = PathBuf::from("../..");
    for source in sources.iter() {
        let mut original = None;
        match &source.protocol {
            Protocol::Netfile { protocol: _ } => {
                let integ_files = IntegFile::vec_from_source(source);
                if let Some(integ_file) = integ_files.last() {
                    original = Some(rel.join(integ_file.get_path()));
                }
            },
            Protocol::Vcs { protocol } =>
                if let VcsProtocol::Git = protocol {
                    original = Some(rel
                        .join(format!("sources/git/{:016x}",
                                xxh3_64(source.url.as_bytes()))));
                },
            Protocol::Local => (),
        }
        if let Some(original) = original {
            crate::filesystem::symlink_force(original, 
                dir.as_ref().join(&source.name))?
        }
    }
    Ok(())
}

// makepkg loves abslute link, but as we use chroot, that breaks up a lot
// pub(crate) fn fix_src_links<P: AsRef<Path>>(srcdir: P) -> Result<()>{
//     if ! srcdir.as_ref().is_dir() {
//         log::error!("'{}' is not a srcdir", srcdir.as_ref().display());
//         return Err(())
//     }
//     let reader = read_dir(&srcdir).or_else(|e|{
//         log::error!("Failed to read dir '{}': {}", srcdir.as_ref().display(), e);
//         Err(())
//     })?;
//     for entry in reader {
//         let entry = entry.or_else(|e|{
//             log::error!("Failed to read dir entry from '{}': {}",
//                     srcdir.as_ref().display(), e);
//             Err(())
//         })?;
//         let ftype = entry.file_type().or_else(|e|{
//             log::error!(
//                 "Failed to read file type form entry '{:?}': {}", entry, e);
//             Err(())
//         })?;
//         if ! ftype.is_symlink() {
//             continue
//         }
//         let link_path = entry.path();
//         let link_target = read_link(&link_path).or_else(op)

//     }
//     Ok(())
// }