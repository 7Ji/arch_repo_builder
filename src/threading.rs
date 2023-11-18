use std::{
        collections::HashMap,
        thread::{
            JoinHandle,
            sleep,
        },
        time::Duration,
    };

pub(crate) fn wait_if_too_busy(
    threads: &mut Vec<JoinHandle<Result<(), ()>>>,
    max_threads: usize,
    job: &str,
) -> Result<(), ()>
{
    if threads.len() >= max_threads {
        if max_threads > 1 {
            log::info!("Waiting for any one of {} threads {} ...",
                    threads.len(), job);
        }
        let mut thread_id_finished = None;
        loop {
            for (thread_id, thread) in
                threads.iter().enumerate()
            {
                if thread.is_finished() {
                    thread_id_finished = Some(thread_id);
                    break
                }
            }
            if let None = thread_id_finished {
                sleep(Duration::from_millis(10));
            } else {
                break
            }
        }
        if let Some(thread_id_finished) = thread_id_finished {
            if max_threads > 1 {
                log::info!("One of {} threads {} ended", threads.len(), job);
            }
            match threads
                        .swap_remove(thread_id_finished)
                        .join()
            {
                Ok(r) => return r,
                Err(e) => {
                    log::error!("Failed to join finished thread: {:?}", e);
                    return Err(())
                },
            }
        } else {
            log::error!("Failed to get finished thread ID");
            return Err(())
        }
    }
    Ok(())
}

pub(crate) fn wait_remaining(
    mut threads: Vec<JoinHandle<Result<(), ()>>>, job: &str
) -> Result<(), ()>
{
    if threads.len() == 0 {
        return Ok(())
    }
    let mut changed = true;
    let mut bad_threads = 0;
    while threads.len() > 0 {
        if changed {
            log::info!("Waiting for {} threads {} ...", threads.len(), job);
        }
        changed = false;
        let mut thread_id_finished = None;
        for (thread_id, thread) in
            threads.iter().enumerate()
        {
            if thread.is_finished() {
                thread_id_finished = Some(thread_id);
                break
            }
        }
        match thread_id_finished {
            Some(thread_id) => {
                log::info!("One of {} threads {} ended", threads.len(), job);
                match threads
                    .swap_remove(thread_id)
                    .join()
                {
                    Ok(r) => match r {
                        Ok(_) => (),
                        Err(_) => bad_threads += 1,
                    },
                    Err(e) => {
                        log::error!(
                            "Failed to join finished thread: {:?}", e);
                        bad_threads += 1;
                    },
                };
                changed = true;
            },
            None => sleep(Duration::from_millis(10)),
        }
    }
    log::info!("Finished waiting for all threads {}", job);
    if bad_threads > 0 {
        log::error!("{} threads {} has bad return", bad_threads, job);
        Err(())
    } else {
        Ok(())
    }
}

pub(crate) fn wait_thread_map<T>(
    map: &mut HashMap<T, Vec<JoinHandle<Result<(), ()>>>>, job: &str
) -> Result<(), ()>
{
    let mut bad = false;
    for threads in map.values_mut() {
        if threads.len() == 0 {
            continue
        }
        loop {
            let mut thread_id_finished = None;
            for (thread_id, thread) in
                threads.iter().enumerate()
            {
                if thread.is_finished() {
                    thread_id_finished = Some(thread_id);
                    break
                }
            }
            match thread_id_finished {
                Some(thread_id) => {
                    log::info!("One of {} threads {} ended", threads.len(), job);
                    match threads
                        .swap_remove(thread_id)
                        .join()
                    {
                        Ok(r) => match r {
                            Ok(_) => (),
                            Err(_) => bad = true,
                        },
                        Err(e) => {
                            log::error!(
                                "Failed to join finished thread: {:?}", e);
                            bad = true;
                        },
                    };
                },
                None => break,
            }
        }
    }
    if bad {
        Err(())
    } else {
        Ok(())
    }
}