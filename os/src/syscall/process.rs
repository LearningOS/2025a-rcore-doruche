//! Process management syscalls

use core::{mem::size_of};

use crate::{config::PAGE_SIZE, mm::{translated_byte_buffer, MapArea, MapPermission, MapType, PageTable, VirtAddr}, syscall::ProtFlags, task::{change_program_brk, current_user_token, exit_current_and_run_next, map_area, munmap, suspend_current_and_run_next, trace_syscall_count}, timer::get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    
    let usec = get_time_us();
    let sec = usec / 1_000_000;
    let usec = usec % 1_000_000;
    let timeval = TimeVal { sec, usec };


    let mut buffers = translated_byte_buffer(
        current_user_token(), 
        ts as *const u8, 
        size_of::<TimeVal>(),
    );
    match buffers.len() {
        1 => {
            let buffer = buffers.get_mut(0).unwrap().as_mut();
            let buffer = &mut unsafe {
                core::mem::transmute::<&mut [u8], &mut [TimeVal]>(buffer)
            }[0];
            *buffer = timeval;
        },
        2 => {
            let timeval_bytes = unsafe {
                core::slice::from_raw_parts(
                    &timeval as *const TimeVal as *const u8, 
                    size_of::<TimeVal>()
                )
            };
            let buffer1 = buffers.get_mut(0).unwrap().as_mut();
            let split = buffer1.len();
            buffer1.copy_from_slice(&timeval_bytes[0..split]);
            let buffer2 = buffers.get_mut(1).unwrap().as_mut();
            buffer2.copy_from_slice(&timeval_bytes[split..]);
        },
        _ => unreachable!(),
    }
    
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    
    const REQ_READ: usize = 0;
    const REQ_WRITE: usize = 1;
    const REQ_COUNT: usize = 2;

    match trace_request {
        REQ_READ => {
            let token = current_user_token();
            let pgtbl = PageTable::from_token(token);
            let vpn = VirtAddr::from(id).floor().into();
            if let Some(pte) = pgtbl.translate(vpn) {
                if pte.is_user() && pte.readable() {
                    let pa = pte.ppn().0 << 12 | (id & 0xfff);
                    let ptr = pa as *const u8;
                    let val = unsafe { *ptr };
                    return val as isize;
                }   
            }
            -1
        },
        REQ_WRITE => {
            let token = current_user_token();
            let pgtbl = PageTable::from_token(token);
            let vpn = VirtAddr::from(id).floor().into();
            if let Some(pte) = pgtbl.translate(vpn) {
                if pte.is_user() && pte.writable() {
                    let pa = pte.ppn().0 << 12 | (id & 0xfff);
                    let ptr = pa as *mut u8;
                    unsafe {
                        ptr.write(data as u8);
                    }
                    return 0;       
                }   
            }
            -1
        },
        REQ_COUNT => trace_syscall_count(id) as isize,
        _ => -1,
    }
}


// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    if len == 0 {
        return 0;
    }
    if start % 4096 != 0 {
        return -1;
    }
    if prot & 0b111 == 0 || prot & !0b111 != 0 {
        return -1;
    }

    let prot = ProtFlags::from_bits_truncate(prot);
    
    let svpn_aligned = VirtAddr::from(start).floor();
    let evpn_aligned = VirtAddr::from(start + len).ceil();
    let area = MapArea::new(
        svpn_aligned.into(),
        evpn_aligned.into(),
        MapType::Framed,
        prot.into(),
    );
    match map_area(area) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");

    if len == 0 {
        return 0;
    }
    if start % 4096 != 0 {
        return -1;
    }

    let svpn = VirtAddr::from(start).floor();
    let npages = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    match munmap(svpn, npages) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
