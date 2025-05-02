use crate::LockFreeHeap;
use core::{
    alloc::Layout,
    sync::atomic::{AtomicUsize, Ordering},
};
use pi_pointer::GetDataBase;

/// 这个实现是用来进行单元测试的，list_test 也是用这个函数
struct GetDataBaseImpl;

static HEAP_BASE: AtomicUsize = AtomicUsize::new(0);

#[crate_interface::impl_interface]
impl GetDataBase for GetDataBaseImpl {
    fn get_data_base() -> usize {
        HEAP_BASE.load(Ordering::Relaxed)
    }
}

#[test]
fn test_empty_heap() {
    let heap = LockFreeHeap::<32>::new();
    assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());
}

#[test]
fn test_heap_add() {
    let heap = LockFreeHeap::<32>::new();
    assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::Relaxed);
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

#[test]
fn test_heap_add_large() {
    // Max size of block is 2^7 == 128 bytes
    let heap = LockFreeHeap::<8>::new();
    assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

    // 512 bytes of space
    let space: [u8; 512] = [0; 512];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::Relaxed);
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(512) as usize);
    }
    let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

#[test]
fn test_heap_oom() {
    let heap = LockFreeHeap::<32>::new();
    let space: [usize; 100] = [0; 100];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::Relaxed);
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }

    assert!(heap
        .alloc_(Layout::from_size_align(100 * size_of::<usize>(), 1).unwrap())
        .is_err());
    assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_ok());
}

#[test]
fn test_heap_alloc_and_free() {
    let heap = LockFreeHeap::<32>::new();
    assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::Relaxed);
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    for _ in 0..100 {
        let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap()).unwrap();
        heap.dealloc_(addr, Layout::from_size_align(1, 1).unwrap());
    }
}

#[test]
fn test_heap_merge_final_order() {
    const NUM_ORDERS: usize = 5;

    let backing_size = 1 << NUM_ORDERS;
    let backing_layout = Layout::from_size_align(backing_size, backing_size).unwrap();

    // create a new heap with 5 orders
    let heap = LockFreeHeap::<NUM_ORDERS>::new();

    // allocate host memory for use by heap
    let backing_allocation = unsafe { std::alloc::alloc(backing_layout) };

    let start = backing_allocation as usize;
    let middle = unsafe { backing_allocation.add(backing_size / 2) } as usize;
    let end = unsafe { backing_allocation.add(backing_size) } as usize;

    HEAP_BASE.store(start, Ordering::Relaxed);
    // add two contiguous ranges of memory
    unsafe { heap.add_to_heap(start, middle) };
    unsafe { heap.add_to_heap(middle, end) };

    // NUM_ORDERS - 1 is the maximum order of the heap
    let layout = Layout::from_size_align(1 << (NUM_ORDERS - 1), 1).unwrap();

    // allocation should succeed, using one of the added ranges
    let alloc = heap.alloc_(layout).unwrap();

    // deallocation should not attempt to merge the two contiguous ranges as the next order does not exist
    heap.dealloc_(alloc, layout);
}
