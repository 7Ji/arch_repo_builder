use std::{
        fs::File,
        io::Read,
        path::Path,
    };

use crate::error::{
        Error,
        Result,
    };

pub(crate) fn http(url: &str, path: &Path, proxy: Option<&str>)
    -> Result<()>
{
    let mut target = match File::create(path) {
        Ok(target) => target,
        Err(e) => {
            log::error!("Failed to open {} as write-only: {}",
                        path.display(), e);
            return Err(Error::IoError(e))
        },
    };
    let response = match proxy {
        Some(proxy) => {
            let proxy_opt = ureq::Proxy::new(proxy).map_err(|e|
            {
                log::error!("Failed to create proxy from '{}': {}", proxy, e);
                Error::UreqError(e)
            })?;
            ureq::AgentBuilder::new().proxy(proxy_opt).build().get(url)
        },
        None => ureq::get(url),
    }.call().map_err(
        |e|{
            log::error!("Failed to GET url '{}': {}", url, e);
            Error::UreqError(e)
        })?;
    let len = match response.header("content-length") {
        Some(len) => len.parse().unwrap(),
        None => {
            log::info!("Warning: response does not have 'content-length', limit \
                max download size to 4GiB");
            0x100000000
        }
    };
    match std::io::copy(
        &mut response.into_reader().take(len), &mut target)
    {
        Ok(size) => {
            log::info!("Downloaded {} bytes from '{}' into '{}'",
                size, url, path.display());
            Ok(())
        },
        Err(e) => {
            log::error!("Failed to copy download '{}' into '{}': {}",
                        url, path.display(), e);
            Err(Error::IoError(e))
        },
    }
}