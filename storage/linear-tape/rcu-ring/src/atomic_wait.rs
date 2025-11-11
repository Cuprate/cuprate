/// Code modified from: <https://github.com/m-ou-se/atomic-wait>
/// We need to support intra-process futexs which that crate doesn't.
/*
Copyright (c) 2022, Mara Bos <m-ou.se@m-ou.se>

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSEARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
pub use futex::*;

#[cfg(windows)]
mod futex {
    use std::sync::atomic::AtomicU32;

    use windows_sys::Win32::System::{
        Threading::{WaitOnAddress, WakeByAddressAll, WakeByAddressSingle},
        WindowsProgramming::INFINITE,
    };

    #[inline]
    pub fn wait(a: &AtomicU32, expected: u32) {
        let ptr: *const AtomicU32 = a;
        let expected_ptr: *const u32 = &expected;
        unsafe { WaitOnAddress(ptr.cast(), expected_ptr.cast(), 4, INFINITE) };
    }

    #[inline]
    pub fn wake_one(ptr: *const AtomicU32) {
        unsafe { WakeByAddressSingle(ptr.cast()) };
    }

    #[inline]
    pub fn wake_all(ptr: *const AtomicU32) {
        unsafe { WakeByAddressAll(ptr.cast()) };
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod futex {
    use std::sync::atomic::AtomicU32;

    #[inline]
    pub fn wait(a: &AtomicU32, expected: u32) {
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                a,
                libc::FUTEX_WAIT,
                expected,
                core::ptr::null::<libc::timespec>(),
            );
        };
    }

    #[inline]
    pub fn wake_one(ptr: *const AtomicU32) {
        unsafe {
            libc::syscall(libc::SYS_futex, ptr, libc::FUTEX_WAKE, 1i32);
        };
    }

    #[inline]
    pub fn wake_all(ptr: *const AtomicU32) {
        unsafe {
            libc::syscall(libc::SYS_futex, ptr, libc::FUTEX_WAKE, i32::MAX);
        };
    }
}

#[cfg(target_os = "freebsd")]
mod futex {
    use std::sync::atomic::AtomicU32;

    #[inline]
    pub fn wait(a: &AtomicU32, expected: u32) {
        let ptr: *const AtomicU32 = a;
        unsafe {
            libc::_umtx_op(
                ptr as *mut libc::c_void,
                libc::UMTX_OP_WAIT_UINT,
                expected as libc::c_ulong,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        };
    }

    #[inline]
    pub fn wake_one(ptr: *const AtomicU32) {
        unsafe {
            libc::_umtx_op(
                ptr as *mut libc::c_void,
                libc::UMTX_OP_WAKE,
                1 as libc::c_ulong,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        };
    }

    #[inline]
    pub fn wake_all(ptr: *const AtomicU32) {
        unsafe {
            libc::_umtx_op(
                ptr as *mut libc::c_void,
                libc::UMTX_OP_WAKE,
                i32::MAX as libc::c_ulong,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        };
    }
}

// for any other OS we just use a spin lock, notably macOS is included here.

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd"
)))]
mod futex {
    use std::sync::atomic::{AtomicU32, Ordering};

    #[inline]
    pub fn wait(a: &AtomicU32, expected: u32) {
        while a.load(Ordering::Relaxed) == expected {
            std::hint::spin_loop();
        }
    }

    #[inline]
    pub fn wake_one(_ptr: *const AtomicU32) {}

    #[inline]
    pub fn wake_all(_ptr: *const AtomicU32) {}
}
