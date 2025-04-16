use crate::linked_list;
use crate::linked_list::LinkedList;
use crate::linked_list::EMPTY_FLAG;
// use crate::Heap;
// use crate::LockedHeapWithRescue;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::mem::size_of;

/// 该函数用于test_linked_list和test_linked_list_concurrent
#[no_mangle]
fn get_data_base() -> usize {
    0x8000
}
#[test]
fn test_linked_list() {
    let mut value1: usize = 0;
    let mut value2: usize = 0;
    let mut value3: usize = 0;
    let mut value4: usize = 0;
    let list = linked_list::LinkedList::new();
    unsafe {
        list.push(&mut value1 as *mut usize as *mut ());
        list.push(&mut value2 as *mut usize as *mut ());
        list.push(&mut value3 as *mut usize as *mut ());
        list.push(&mut value4 as *mut usize as *mut ());
    }

    // Test links
    // 访问链表内的内容，因此需要偏移
    assert_eq!(value4 + get_data_base(), &value3 as *const usize as usize);
    assert_eq!(value3 + get_data_base(), &value2 as *const usize as usize);
    assert_eq!(value2 + get_data_base(), &value1 as *const usize as usize);
    assert_eq!(value1, EMPTY_FLAG as usize);

    // Test delete
    assert_eq!(list.delete(&mut value2 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value4 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value4 as *mut usize as *mut ()), false);

    // Test pop
    assert_eq!(list.pop(), Some(&mut value3 as *mut usize as *mut ()));
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);
}

#[test]
fn test_linked_list_concurrent() {
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;

    let mut handles = Vec::with_capacity(6);
    let barrier1 = Arc::new(Barrier::new(6));
    let barrier2 = Arc::new(Barrier::new(6));
    let mut values: [usize; 6] = [0; 6];
    let list = Arc::new(linked_list::LinkedList::new());

    for i in 0..6 {
        let b1 = barrier1.clone();
        let b2 = barrier2.clone();
        let l = list.clone();
        handles.push(thread::spawn(move || {
            // 读写分开进行的测试，更好从输出的值中判断是否行为正确
            let value_ptr = ((&values as *const [usize] as *const () as usize) + i * size_of::<usize>()) as *mut ();
            println!("value{i} = {:#p}", value_ptr);
            unsafe { l.push(value_ptr); }
            println!("*value{i} = {:#x} after push", unsafe {*(value_ptr as *mut usize)});
            b1.wait();
            let poped = l.pop().unwrap();
            println!("thread{i} get addr {:#p} after pop", poped);

            // 读写同时进行的测试，验证其不会死锁
            b2.wait();
            unsafe { l.push(value_ptr); }
            assert!(l.delete(value_ptr));
            println!("thread{i} test complete!")

        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// static mut SPACE: [usize; 0x1000] = [0; 0x1000];

// /// 除了最后一个测试，其他的测试都需要使用下面的 get_data_base() 来获取数据段的偏移
// #[no_mangle]
// fn get_data_base() -> usize {
//     &raw mut SPACE as usize - 0x1000
// }

// #[test]
// fn test_empty_heap() {
//     let mut heap = Heap::<32>::new();
//     assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());
// }

// #[test]
// fn test_heap_add() {
//     let mut heap = Heap::<32>::new();
//     assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
//     assert!(addr.is_ok());
// }

// #[test]
// fn test_heap_add_large() {
//     // Max size of block is 2^7 == 128 bytes
//     let mut heap = Heap::<8>::new();
//     assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
//     assert!(addr.is_ok());
// }

// #[test]
// fn test_heap_oom() {
//     let mut heap = Heap::<32>::new();
//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }

//     assert!(heap
//         .alloc(Layout::from_size_align(1000 * size_of::<usize>(), 1).unwrap())
//         .is_err());
//     assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_ok());
// }

// #[test]
// fn test_heap_oom_rescue() {
//     let heap = LockedHeapWithRescue::new(|heap: &mut Heap<32>, _layout: &Layout| unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     });

//     unsafe {
//         assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()) as usize != 0);
//     }
// }

// #[test]
// fn test_heap_alloc_and_free() {
//     let mut heap = Heap::<32>::new();
//     assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

//     unsafe {
//         heap.add_to_heap(0x1000, 0x2000);
//     }
//     for _ in 0..1000 {
//         let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
//         heap.dealloc(addr, Layout::from_size_align(1, 1).unwrap());
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
//     let mut heap = Heap::<NUM_ORDERS>::new();

//     let start = 0x1000;
//     let middle = BACKING_SIZE / 2 + start;
//     let end = BACKING_SIZE + start;

//     // add two contiguous ranges of memory
//     unsafe { heap.add_to_heap(start, middle) };
//     unsafe { heap.add_to_heap(middle, end) };

//     // NUM_ORDERS - 1 is the maximum order of the heap
//     let layout = Layout::from_size_align(1 << (NUM_ORDERS - 1), 1).unwrap();

//     // allocation should succeed, using one of the added ranges
//     let alloc = heap.alloc(layout).unwrap();

//     // deallocation should not attempt to merge the two contiguous ranges as the next order does not exist
//     heap.dealloc(alloc, layout);
// }
