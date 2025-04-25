use pi_pointer::NULL_PTR;

use crate::linked_list;
use crate::linked_list::LinkedList;
// use crate::linked_list::EMPTY_FLAG;
// use crate::Heap;
// use crate::LockedHeapWithRescue;
use core::mem::size_of;
use core::ptr::null_mut;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::time::Duration;
use std::sync::Barrier;
use std::thread;

/// 该函数用于test_linked_list和test_linked_list_concurrent
#[no_mangle]
fn get_data_base() -> usize {
    0x8000
    // 0
}
#[test]
fn test_linked_list_func() {
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
    assert_eq!(value4, (&value3 as *const usize as usize) - get_data_base());
    assert_eq!(value3, (&value2 as *const usize as usize) - get_data_base());
    assert_eq!(value2, (&value1 as *const usize as usize) - get_data_base());
    assert_eq!(value1, NULL_PTR as usize);

    // Test delete
    assert_eq!(list.delete(&mut value2 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value2 as *mut usize as *mut ()), false);
    assert_eq!(list.delete(&mut value4 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value4 as *mut usize as *mut ()), false);
    assert_eq!(list.delete(&mut value3 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value3 as *mut usize as *mut ()), false);
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), true);
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), false);

    unsafe {
        list.push(&mut value1 as *mut usize as *mut ());
        list.push(&mut value2 as *mut usize as *mut ());
        list.push(&mut value3 as *mut usize as *mut ());
        list.push(&mut value4 as *mut usize as *mut ());
    }

    // Test pop
    assert_eq!(list.pop(), Some(&mut value4 as *mut usize as *mut ()));
    assert_eq!(list.pop(), Some(&mut value3 as *mut usize as *mut ()));
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize as *mut ()));
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);
}

#[test]
fn test_delete() {
    use linked_list::DELETE_MARK;

    let mut value1: usize = 0;
    let mut value2: usize = 0;
    let mut value3: usize = 0;
    let list = linked_list::LinkedList::new();

    // 删除不存在元素，链表为空
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), false);
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 = 1
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    assert_eq!(list.delete(&mut value2 as *mut usize as *mut ()), false);
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 = 1且队尾被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    value1 = value1 | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value2 as *mut usize as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    assert_eq!(list.delete(&mut value3 as *mut usize as *mut ()), false);
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize as *mut ()));
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1且队尾被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    value1 = value1 | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value3 as *mut usize as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1且队尾的前驱被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    value2 = value2 | DELETE_MARK; // 手动标记value2
    assert_eq!(list.delete(&mut value3 as *mut usize as *mut ()), false); // delete中的search过程不会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize as *mut ())); // pop出value1的同时，其中的search过程还会删除被标记的value2
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 = 1
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), true);
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 = 1且目标被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    value1 = value1 | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), true);
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1且目标被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    value1 = value1 | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1且目标的前驱被标记
    unsafe { list.push(&mut value1 as *mut usize as *mut ()) };
    unsafe { list.push(&mut value2 as *mut usize as *mut ()) };
    value2 = value2 | DELETE_MARK; // 手动标记value2
    assert_eq!(list.delete(&mut value1 as *mut usize as *mut ()), true); // delete中的search过程会删除被标记的value2
    assert_eq!(list.pop(), None);
}

#[test]
fn test_linked_list_concurrent() {
    use std::sync::Arc;
    use std::thread;

    const NUM_PRODUCERS: usize = 20;
    const NUM_DELETE_CONSUMERS: usize = 10;
    const NUM_POP_CONSUMERS: usize = 10;
    const NUM_DATA_PER_THREAD: usize = 500;
    assert!(NUM_PRODUCERS == NUM_DELETE_CONSUMERS + NUM_POP_CONSUMERS);

    let mut handles = Vec::with_capacity(NUM_PRODUCERS + NUM_DELETE_CONSUMERS + NUM_POP_CONSUMERS);
    let values: Arc<[usize; NUM_PRODUCERS * NUM_DATA_PER_THREAD]> =
        Arc::new([0; NUM_PRODUCERS * NUM_DATA_PER_THREAD]);
    // 用于记录values的每个位置被从链表中取出了几次
    let pop_nums: Arc<[AtomicUsize; NUM_PRODUCERS * NUM_DATA_PER_THREAD]> =
        Arc::new([const { AtomicUsize::new(0) }; NUM_PRODUCERS * NUM_DATA_PER_THREAD]);
    // println!("&value = {:?}", values.as_ptr_range());
    let list = Arc::new(linked_list::LinkedList::new());
    // let barrier = Arc::new(Barrier::new(NUM_PRODUCERS + 1));

    for i in 0..NUM_PRODUCERS {
        let l = list.clone();
        let v = values.clone();
        // let b = barrier.clone();
        handles.push(thread::spawn(move || {
            let mut value_ptr: [*mut (); NUM_DATA_PER_THREAD] = [null_mut(); NUM_DATA_PER_THREAD];
            // println!("&value = {:?}", v.as_ptr_range());
            for j in 0..NUM_DATA_PER_THREAD {
                value_ptr[j] = ((v.as_ptr() as *const () as usize)
                    + i * NUM_DATA_PER_THREAD * size_of::<usize>()
                    + j * size_of::<usize>()) as *mut ();
                // println!("&value[{i}][{j}] = {:p}", value_ptr[j]);
            }

            for j in 0..NUM_DATA_PER_THREAD {
                unsafe {
                    l.push(value_ptr[j]);
                }
            }
            // println!("producer {i} finished");

            // b.wait();
        }));
    }

    // barrier.wait();

    for i in 0..NUM_DELETE_CONSUMERS {
        let l = list.clone();
        let v = values.clone();
        let p = pop_nums.clone();
        handles.push(thread::spawn(move || {
            let mut value_ptr: [*mut (); NUM_DATA_PER_THREAD] = [null_mut(); NUM_DATA_PER_THREAD];
            for j in 0..NUM_DATA_PER_THREAD {
                value_ptr[j] = ((v.as_ptr() as *const () as usize)
                    + i * NUM_DATA_PER_THREAD * size_of::<usize>()
                    + j * size_of::<usize>()) as *mut ();
            }

            let mut j = 0; // 删除计数
            while j < NUM_DATA_PER_THREAD {
                if l.delete(value_ptr[j]) {
                    // 删除指定位置成功
                    p[i * NUM_DATA_PER_THREAD + j].fetch_add(1, Ordering::AcqRel);
                    j += 1; // 只有删除成功才会增加删除计数
                } else {
                    if let Some(ptr) = l.pop() {
                        // 删除指定位置失败，因此改为pop一个元素，以确保每个消费者删除的元素数量恒定
                        let offset =
                            (ptr as usize - v.as_ptr() as *const () as usize) / size_of::<usize>();
                        p[offset].fetch_add(1, Ordering::AcqRel);
                        j += 1; // 只有删除成功才会增加删除计数
                    }
                }
            }
        }));
    }

    for i in NUM_DELETE_CONSUMERS..NUM_DELETE_CONSUMERS + NUM_POP_CONSUMERS {
        let l = list.clone();
        let v = values.clone();
        let p = pop_nums.clone();
        handles.push(thread::spawn(move || {
            let mut j = 0; // 删除计数
            let pop_num = if i <= 0 {
                NUM_DATA_PER_THREAD
            } else {
                NUM_DATA_PER_THREAD
            };
            while j < pop_num {
                if let Some(ptr) = l.pop() {
                    let offset =
                        (ptr as usize - v.as_ptr() as *const () as usize) / size_of::<usize>();
                    p[offset].fetch_add(1, Ordering::AcqRel);
                    j += 1; // 只有删除成功才会增加删除计数
                }
            }
            // println!("consumer {i} finished");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 用于多次运行；当持续时间过长，直接不通过
    // thread::sleep(Duration::from_secs(1));
    // for handle in handles {
    //     assert!(handle.is_finished());
    // }

    assert!(list.is_empty()); // 验证列表为空
    for i in 0..NUM_PRODUCERS * NUM_DATA_PER_THREAD {
        assert!(pop_nums[i].load(Ordering::Acquire) == 1); // 验证所有元素恰好被取出一次。
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
