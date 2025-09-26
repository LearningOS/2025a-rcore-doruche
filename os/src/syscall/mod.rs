//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// unlinkat syscall
const SYSCALL_UNLINKAT: usize = 35;
/// linkat syscall
const SYSCALL_LINKAT: usize = 37;
/// open syscall
const SYSCALL_OPEN: usize = 56;
/// close syscall
const SYSCALL_CLOSE: usize = 57;
/// read syscall
const SYSCALL_READ: usize = 63;
/// write syscall
const SYSCALL_WRITE: usize = 64;
/// fstat syscall
const SYSCALL_FSTAT: usize = 80;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// setpriority syscall
const SYSCALL_SET_PRIORITY: usize = 140;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// getpid syscall
const SYSCALL_GETPID: usize = 172;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// fork syscall
const SYSCALL_FORK: usize = 220;
/// exec syscall
const SYSCALL_EXEC: usize = 221;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// waitpid syscall
const SYSCALL_WAITPID: usize = 260;
/// spawn syscall
const SYSCALL_SPAWN: usize = 400;

mod fs;
mod process;

use core::mem::size_of;

use fs::*;
use process::*;

use crate::{config::PAGE_SIZE, fs::Stat, mm::translated_byte_buffer, task::current_user_token};

bitflags! {
    /// Memory protection flags, used in `mmap` syscall
    pub struct ProtFlags: usize {
        /// No permissions
        const PROT_NONE  = 0;
        /// Pages can be read
        const PROT_READ  = 1 << 0;
        /// Pages can be written
        const PROT_WRITE = 1 << 1;
        /// Pages can be executed
        const PROT_EXEC  = 1 << 2;
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 4]) -> isize {
    match syscall_id {
        SYSCALL_OPEN => sys_open(args[1] as *const u8, args[2] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_LINKAT => sys_linkat(args[1] as *const u8, args[3] as *const u8),
        SYSCALL_UNLINKAT => sys_unlinkat(args[1] as *const u8),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut Stat),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as isize),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

/// dst pointer may span across pages, so we need to handle it carefully
pub fn write_small_data<T>(dst: *mut T, src: T) {
    assert!(size_of::<T>() < PAGE_SIZE * 2);

    let mut buffers = translated_byte_buffer(
        current_user_token(), 
        dst as *const u8, 
        size_of::<T>(),
    );
    
    match buffers.len() {
        1 => {
            let buffer = buffers.get_mut(0).unwrap().as_mut();
            let buffer = &mut unsafe {
                core::mem::transmute::<&mut [u8], &mut [T]>(buffer)
            }[0];
            *buffer = src;
        },
        2 => {
            let src_bytes = unsafe {
                core::slice::from_raw_parts(
                    &src as *const T as *const u8,
                    size_of::<T>()
                )
            };
            
            let buffer1 = buffers.get_mut(0).unwrap().as_mut();
            let split = buffer1.len();
            buffer1.copy_from_slice(&src_bytes[0..split]);
            let buffer2 = buffers.get_mut(1).unwrap().as_mut();
            buffer2.copy_from_slice(&src_bytes[split..]);
        },
        _ => unreachable!(),
    }
}