//! Guest memory: base+offset model using anonymous mmap.

use std::io;
use std::ptr;

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: *mut libc_void = !0usize as *mut libc_void;

#[repr(C)]
struct libc_void(u8);

extern "C" {
    fn mmap(
        addr: *mut libc_void,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut libc_void;
    fn munmap(addr: *mut libc_void, length: usize) -> i32;
}

pub struct GuestMemory {
    ptr: *mut u8,
    size: usize,
}

impl GuestMemory {
    pub fn new() -> io::Result<Self> {
        for size in [1usize << 32, 256 << 20, 64 << 20] {
            let p = unsafe {
                mmap(
                    ptr::null_mut(),
                    size,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if p != MAP_FAILED && !p.is_null() {
                return Ok(Self {
                    ptr: p as *mut u8,
                    size,
                });
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "failed to map guest memory"))
    }

    pub fn base_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn write_bytes(&mut self, addr: u32, bytes: &[u8]) {
        let a = (addr as usize) % self.size;
        unsafe {
            for (i, b) in bytes.iter().enumerate() {
                if a + i >= self.size {
                    break;
                }
                *self.ptr.add(a + i) = *b;
            }
        }
    }

    pub fn fetch_guest_bytes(&self, pc: u64) -> [u8; 4] {
        let a = (pc as usize) % self.size;
        let mut out = [0u8; 4];
        unsafe {
            for i in 0..4 {
                out[i] = *self.ptr.add((a + i) % self.size);
            }
        }
        out
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                munmap(self.ptr as *mut libc_void, self.size);
            }
        }
    }
}

// GuestMemory is !Send by default due to raw pointer; mark explicitly for tests.
unsafe impl Send for GuestMemory {}
