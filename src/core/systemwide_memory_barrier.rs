use std::cell::RefCell;
use std::io;

use libc;
use memmap::{MmapMut, MmapOptions};


#[repr(i32)]
#[allow(dead_code, non_camel_case_types)]
enum membarrier_cmd {
    MEMBARRIER_CMD_QUERY = 0,
    MEMBARRIER_CMD_GLOBAL = 1 << 0,
    MEMBARRIER_CMD_GLOBAL_EXPEDITED = 1 << 1,
    MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED = 1 << 2,
    MEMBARRIER_CMD_PRIVATE_EXPEDITED = 1 << 3,
    MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED = 1 << 4,
    MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE = 1 << 5,
    MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE = 1 << 6,
}

/// Call membarrier systemcall.
#[inline]
fn membarrier(cmd: membarrier_cmd) -> libc::c_long {
    unsafe { libc::syscall(libc::SYS_membarrier, cmd as libc::c_int, 0 as libc::c_int) }
}

lazy_static! {
    /// Represents whether the `sys_membarrier` system call is supported.
    static ref HAS_NATIVE_MEMBARRIER: bool = {
        let ret = membarrier(membarrier_cmd::MEMBARRIER_CMD_QUERY);
        if ret < 0 ||
            ret & membarrier_cmd::MEMBARRIER_CMD_PRIVATE_EXPEDITED as libc::c_long == 0 ||
            ret & membarrier_cmd::MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED as libc::c_long == 0
        {
            return false;
        }

        if membarrier(membarrier_cmd::MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED) < 0 {
            return false;
        }
        true
    };
}

fn try_native_mebarrier() -> bool {
    if *HAS_NATIVE_MEMBARRIER {
        membarrier(membarrier_cmd::MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED);
        true
    } else {
        false
    }
}

/// cause all threads to invoke a full memory barrier.
pub fn systemwide_memory_barrier() -> Result<(), io::Error> {
    if try_native_mebarrier() {
        return Ok(());
    }

    thread_local!(pub static MEM: RefCell<MmapMut> = {
        let mem = unsafe {
            MmapOptions::new()
                .len(libc::sysconf(libc::_SC_PAGESIZE) as usize)
                .map_anon()
                .unwrap()
        };
        RefCell::new(mem)
    });
    match MEM.with(|m| {
        // Force page into memory to make madvise() have real work to do
        if let Some(first) = m.borrow_mut().first_mut() {
            *first = 3;
        }
        unsafe {
            // Evict page to force kernel to send IPI to all threads, with
            // a side effect of executing a memory barrier on those threads
            libc::madvise(m.borrow_mut().as_mut_ptr() as *mut libc::c_void,
                          libc::sysconf(libc::_SC_PAGESIZE) as usize,
                          libc::MADV_DONTNEED)
        }
    }) {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error())
    }
}


#[cfg(not(target_arch = "aarch64"))]
pub fn try_systemwide_memory_barrier() -> bool {
    if try_native_mebarrier() {
        return true;
    }
    systemwide_memory_barrier().is_ok()
}


#[cfg(target_arch = "aarch64")]
pub fn try_systemwide_memory_barrier() -> bool {
    // Some (not all) ARM processors can broadcast TLB invalidations using the
    // TLBI instruction. On those, the mprotect trick won't work.
    try_native_mebarrier()
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn barriers() {
        systemwide_memory_barrier();
        try_systemwide_memory_barrier();
    }
}
