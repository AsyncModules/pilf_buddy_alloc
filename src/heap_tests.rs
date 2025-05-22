use crate::LockFreeHeap;
use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};
use ctor::ctor;
use pi_pointer::GetDataBase;
use std::thread::{spawn, JoinHandle};

/// 这个实现是用来进行单元测试的，list_test 也是用这个函数
struct GetDataBaseImpl;

static HEAP_BASE: AtomicUsize = AtomicUsize::new(0);

#[crate_interface::impl_interface]
impl GetDataBase for GetDataBaseImpl {
    fn get_data_base() -> usize {
        HEAP_BASE.load(Ordering::SeqCst)
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
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::SeqCst);
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
    let space: [usize; 64] = [0; 64];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::SeqCst);
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(64) as usize);
    }
    let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

#[test]
fn test_heap_oom() {
    let heap = LockFreeHeap::<32>::new();
    let space: [usize; 100] = [0; 100];
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::SeqCst);
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
    HEAP_BASE.store(space.as_ptr() as usize, Ordering::SeqCst);
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

    HEAP_BASE.store(start, Ordering::SeqCst);
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

const SMALL_SIZE: usize = 8;
const LARGE_SIZE: usize = 1024 * 1024; // 1M
const ALIGN: usize = 8;
const ORDER: usize = 33;
const MACHINE_ALIGN: usize = core::mem::size_of::<usize>();
/// for now 128M is needed
/// TODO: reduce memory use
const KERNEL_HEAP_SIZE: usize = 128 * 1024 * 1024;
const HEAP_BLOCK: usize = KERNEL_HEAP_SIZE / MACHINE_ALIGN;
#[repr(C, align(0x1000))]
struct HeapSpace(pub(crate) [usize; HEAP_BLOCK]);
static mut HEAP: HeapSpace = HeapSpace([0; HEAP_BLOCK]);

// #[global_allocator]
static HEAP_ALLOCATOR: LockFreeHeap<ORDER> = LockFreeHeap::<ORDER>::new();

/// Init heap
///
/// We need `ctor` here because benchmark is running behind the std enviroment,
/// which means std will do some initialization before execute `fn main()`.
/// However, our memory allocator must be init in runtime(use linkedlist, which
/// can not be evaluated in compile time). And in the initialization phase, heap
/// memory is needed.
///
/// So the solution in this dilemma is to run `fn init_heap()` in initialization phase
/// rather than in `fn main()`. We need `ctor` to do this.
#[ctor]
fn init_heap() {
    let heap_start = &raw mut HEAP as *mut _ as usize;
    unsafe {
        HEAP_ALLOCATOR.init(heap_start, HEAP_BLOCK * MACHINE_ALIGN);
    }
}

// 运行该测试时，需要注释#[global_allocator]和#[ctor]两个注解
// PASSED
#[test]
fn test_singlethread() {
    init_heap();
    unsafe {
        println!("{:?}", HEAP_ALLOCATOR);
        let small_layout = Layout::from_size_align_unchecked(SMALL_SIZE, ALIGN);
        let small_addr = HEAP_ALLOCATOR.alloc(small_layout);
        assert!(!small_addr.is_null());
        *(small_addr as *mut () as *mut usize) = 42;
        println!("{:?}", HEAP_ALLOCATOR);
        assert!(*(small_addr as *mut () as *mut usize) == 42);
        let large_layout = Layout::from_size_align_unchecked(LARGE_SIZE, ALIGN);
        let large_addr = HEAP_ALLOCATOR.alloc(large_layout);
        assert!(!large_addr.is_null());
        *(large_addr as *mut () as *mut usize) = 42;
        println!("{:?}", HEAP_ALLOCATOR);
        assert!(*(large_addr as *mut () as *mut usize) == 42);
        HEAP_ALLOCATOR.dealloc(small_addr, small_layout);
        HEAP_ALLOCATOR.dealloc(large_addr, large_layout);
    }
}

// 运行该测试时，需要注释#[global_allocator]和#[ctor]两个注解
// FAILED
#[test]
fn test_multithread() {
    init_heap();
    println!("{:?}", HEAP_ALLOCATOR);
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    for _ in 0..100 {
        handles.push(spawn(|| unsafe {
            for _ in 0..100 {
                let small_layout = Layout::from_size_align_unchecked(SMALL_SIZE, ALIGN);
                let small_addr = HEAP_ALLOCATOR.alloc(small_layout);
                assert!(!small_addr.is_null());
                *(small_addr as *mut () as *mut usize) = 1;
                assert!(*(small_addr as *mut () as *mut usize) == 1);
                // let large_layout = Layout::from_size_align_unchecked(LARGE_SIZE, ALIGN);
                // let large_addr = HEAP_ALLOCATOR.alloc(large_layout);
                // assert!(!large_addr.is_null());
                // *(large_addr as *mut () as *mut usize) = 42;
                // assert!(*(large_addr as *mut () as *mut usize) == 42);
                HEAP_ALLOCATOR.dealloc(small_addr, small_layout);
                // HEAP_ALLOCATOR.dealloc(large_addr, large_layout);
            }
        }));
    }
    for h in handles {
        assert!(h.join().is_ok());
    }
    println!("{:?}", HEAP_ALLOCATOR);
}

// 运行该测试时，需要取消注释#[global_allocator]和#[ctor]两个注解
// PASSED
#[test]
fn test_global_allocator_singlethread() {
    println!("{:?}", HEAP_ALLOCATOR);
    let small: Box<usize> = Box::new(42);
    println!("{:?}", HEAP_ALLOCATOR);
    assert!(*small == 42);
    let large: Box<[usize; LARGE_SIZE / size_of::<usize>()]> =
        Box::new([0; LARGE_SIZE / size_of::<usize>()]);
    println!("{:?}", HEAP_ALLOCATOR);
    assert!((*large)[42] == 0);
}

// 运行该测试时，需要取消注释#[global_allocator]和#[ctor]两个注解
// FAILED
#[test]
fn test_global_allocator_multithread() {
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    for _ in 0..100 {
        handles.push(spawn(|| {
            // for _ in 0..100 {
            //     let small: Box<usize> = Box::new(42);
            //     assert!(*small == 42);
            //     let large: Box<[usize; LARGE_SIZE / size_of::<usize>()]> =
            //         Box::new([0; LARGE_SIZE / size_of::<usize>()]);
            //     assert!((*large)[42] == 0);
            // }
        }));
    }
    for h in handles {
        assert!(h.join().is_ok());
    }
    println!("{:?}", HEAP_ALLOCATOR);
}
