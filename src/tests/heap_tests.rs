// use crate::LockFreeHeap;
// use core::alloc::Layout;
// // /// 该函数用于test_linked_list和test_linked_list_concurrent
// // #[no_mangle]
// // fn get_data_base() -> usize {
// //     0x8000
// // }

// static mut SPACE: [usize; 0x1000] = [0; 0x1000];

// /// 除了最后一个测试，其他的测试都需要使用下面的 get_data_base() 来获取数据段的偏移
// #[no_mangle]
// fn get_data_base() -> usize {
//     &raw mut SPACE as usize - 0x1000
// }

// #[test]
// fn test_empty_heap() {
//     let heap = LockFreeHeap::<32>::new();
//     assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());
// }

// #[test]
// fn test_heap_add() {
//     let heap = LockFreeHeap::<32>::new();
//     assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap());
//     assert!(addr.is_ok());
// }

// #[test]
// fn test_heap_add_large() {
//     // Max size of block is 2^7 == 128 bytes
//     let heap = LockFreeHeap::<8>::new();
//     assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap());
//     assert!(addr.is_ok());
// }

// #[test]
// fn test_heap_oom() {
//     let heap = LockFreeHeap::<32>::new();
//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }

//     assert!(heap
//         .alloc_(Layout::from_size_align(1000 * size_of::<usize>(), 1).unwrap())
//         .is_err());
//     assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_ok());
// }

// #[test]
// fn test_heap_alloc_and_free() {
//     let heap = LockFreeHeap::<32>::new();
//     assert!(heap.alloc_(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     for _ in 0..1000 {
//         let addr = heap.alloc_(Layout::from_size_align(1, 1).unwrap()).unwrap();
//         heap.dealloc_(addr, Layout::from_size_align(1, 1).unwrap());
//     }
// }

// /// 测试时，需要将函数内的 get_data_base() 函数取消注释
// #[test]
// fn test_heap_merge_final_order() {
//     const NUM_ORDERS: usize = 5;
//     const BACKING_SIZE: usize = 1 << NUM_ORDERS;

//     // static BACKING_ALLOCATION: spin::Lazy<usize> = spin::Lazy::new(|| {
//     //     let backing_layout = Layout::from_size_align(BACKING_SIZE, BACKING_SIZE).unwrap();
//     //     unsafe { std::alloc::alloc(backing_layout) as usize }
//     // });
//     // #[no_mangle]
//     // fn get_data_base() -> usize {
//     //     *BACKING_ALLOCATION - 0x1000
//     // }

//     // create a new heap with 5 orders
//     let heap = LockFreeHeap::<NUM_ORDERS>::new();

//     let start = 0x1000;
//     let middle = BACKING_SIZE / 2 + start;
//     let end = BACKING_SIZE + start;

//     // add two contiguous ranges of memory
//     unsafe { heap.add_to_heap(start, middle) };
//     unsafe { heap.add_to_heap(middle, end) };

//     // NUM_ORDERS - 1 is the maximum order of the heap
//     let layout = Layout::from_size_align(1 << (NUM_ORDERS - 1), 1).unwrap();

//     // allocation should succeed, using one of the added ranges
//     let alloc = heap.alloc_(layout).unwrap();

//     // deallocation should not attempt to merge the two contiguous ranges as the next order does not exist
//     heap.dealloc_(alloc, layout);
// }
