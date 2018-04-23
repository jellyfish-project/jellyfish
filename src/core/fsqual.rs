use std::fs::{remove_file, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::OpenOptionsExt;
use std::result::Result;

use linux_aio::{AlignedBuf, Context, ControlBlock};
use libc::{getrusage, rusage, timeval, O_DIRECT, RUSAGE_THREAD};
use scopeguard::guard;

const BLOCK_SIZE: usize = 4096;
const NUM_IO_SUBMITS: usize = 1000;
const THRESHOLD: f64 = 0.1;

fn zeroed_rusage() -> rusage {
    rusage {
        ru_utime: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_stime: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_maxrss: 0,
        ru_ixrss: 0,
        ru_idrss: 0,
        ru_isrss: 0,
        ru_minflt: 0,
        ru_majflt: 0,
        ru_nswap: 0,
        ru_inblock: 0,
        ru_oublock: 0,
        ru_msgsnd: 0,
        ru_msgrcv: 0,
        ru_nsignals: 0,
        ru_nvcsw: 0,
        ru_nivcsw: 0,
    }
}

pub fn filesystem_has_good_aio_support(file_path: &str) -> Result<(), &str> {
    let f = try!(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .custom_flags(O_DIRECT)
            .mode(0o600)
            .open(file_path)
            .map_err(|_| "Failed to open temp file.")
    );
    let _ = guard((), |_| {
        remove_file(file_path).unwrap();
    });

    let _ = try!(
        f.set_len((BLOCK_SIZE * NUM_IO_SUBMITS) as u64)
            .map_err(|_| "Failed to call ftruncate(2).")
    );
    let raw_fd = f.as_raw_fd();

    let mut csws: i64 = 0;
    let mut rusg = zeroed_rusage();
    let mut context = try!(Context::setup(1).map_err(|_| "Failed to init an AIO context."));
    let sets = (0..NUM_IO_SUBMITS).map({ |i| (i, AlignedBuf::new(BLOCK_SIZE)) });
    for set in sets {
        let (i, buf) = set;

        unsafe {
            getrusage(RUSAGE_THREAD, &mut rusg as *mut rusage);
            csws -= rusg.ru_nvcsw;
        }

        try!(
            context
                .submit(ControlBlock::pwrite(raw_fd, buf, BLOCK_SIZE * i))
                .map_err(|_| "Failed to call iosubmmit(2).")
        );

        unsafe {
            getrusage(RUSAGE_THREAD, &mut rusg as *mut rusage);
            csws += rusg.ru_nvcsw;
        }

        let events = try!(
            context
                .get_events(1, 1, None)
                .map_err(|_| "Failed to call iogetevents(2).")
        );
        events.into_iter().nth(0).expect("Never reach here.");
    }

    if (csws as f64) > THRESHOLD {
        return Err("This filesystem doesn't have good AIO support.");
    }

    return Ok(());
}
