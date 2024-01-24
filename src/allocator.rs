use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

pub struct Dummy;

#[global_allocator]
static ALLOCATOR: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        panic!("dealloc should be never called")
    }
}
