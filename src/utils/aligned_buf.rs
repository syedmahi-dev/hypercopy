use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ops::{Deref, DerefMut};

/// Sector-aligned memory buffer for direct unbuffered DMA I/O.
/// Direct I/O (FILE_FLAG_NO_BUFFERING / O_DIRECT) requires that the memory buffer
/// address and transfer size be an exact multiple of the disk sector size (typically 4096 bytes).
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: Layout,
    capacity: usize,
}

unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

impl AlignedBuffer {
    pub const DEFAULT_ALIGN: usize = 4096;

    /// Allocate an aligned buffer of the requested capacity.
    /// The capacity will be rounded up to a multiple of `align`.
    pub fn new(capacity: usize, align: usize) -> Result<Self, String> {
        let align = align.max(Self::DEFAULT_ALIGN);
        let rounded_cap = (capacity + align - 1) & !(align - 1);

        let layout = Layout::from_size_align(rounded_cap, align)
            .map_err(|e| format!("Invalid layout: {}", e))?;

        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err("Failed to allocate aligned memory buffer".to_string());
        }

        Ok(Self {
            ptr,
            layout,
            capacity: rounded_cap,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.capacity) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.capacity) }
    }
}

impl Deref for AlignedBuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}
