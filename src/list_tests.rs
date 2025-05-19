use pi_pointer::NULL_PTR;

use crate::linked_list;
use crate::LinkedList;
use core::mem::size_of;
use core::ptr::null_mut;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::usize;

use crate::get_data_base;

#[test]
fn test_linked_list_func() {
    let mut value1: [usize; 2] = [0; 2];
    let mut value2: [usize; 2] = [0; 2];
    let mut value3: [usize; 2] = [0; 2];
    let mut value4: [usize; 2] = [0; 2];
    let list = linked_list::LinkedList::new();
    unsafe {
        list.push(&mut value1 as *mut [usize] as *mut ());
        list.push(&mut value2 as *mut [usize] as *mut ());
        list.push(&mut value3 as *mut [usize] as *mut ());
        list.push(&mut value4 as *mut [usize] as *mut ());
    }

    // Test links
    // 访问链表内的内容，因此需要偏移
    assert_eq!(
        value4[0],
        (&value3 as *const usize as usize) - get_data_base()
    );
    assert_eq!(
        value3[0],
        (&value2 as *const usize as usize) - get_data_base()
    );
    assert_eq!(
        value2[0],
        (&value1 as *const usize as usize) - get_data_base()
    );
    assert_eq!(value1[0], NULL_PTR as usize);

    // Test delete
    assert_eq!(list.delete(&mut value2 as *mut [usize] as *mut ()), true);
    assert_eq!(list.delete(&mut value2 as *mut [usize] as *mut ()), false);
    assert_eq!(list.delete(&mut value4 as *mut [usize] as *mut ()), true);
    assert_eq!(list.delete(&mut value4 as *mut [usize] as *mut ()), false);
    assert_eq!(list.delete(&mut value3 as *mut [usize] as *mut ()), true);
    assert_eq!(list.delete(&mut value3 as *mut [usize] as *mut ()), false);
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), true);
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), false);

    unsafe {
        list.push(&mut value1 as *mut [usize] as *mut ());
        list.push(&mut value2 as *mut [usize] as *mut ());
        list.push(&mut value3 as *mut [usize] as *mut ());
        list.push(&mut value4 as *mut [usize] as *mut ());
    }

    // Test pop
    assert_eq!(list.pop(), Some(&mut value4 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), Some(&mut value3 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), Some(&mut value2 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), Some(&mut value1 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);
}

#[test]
#[allow(unused_assignments)]
fn test_delete() {
    use linked_list::DELETE_MARK;

    let mut value1: [usize; 2] = [0; 2];
    let mut value2: [usize; 2] = [0; 2];
    let mut value3: [usize; 2] = [0; 2];
    let list = linked_list::LinkedList::new();

    // 删除不存在元素，链表为空
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), false);
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 = 1
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    assert_eq!(list.delete(&mut value2 as *mut [usize] as *mut ()), false);
    assert_eq!(list.pop(), Some(&mut value1 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 = 1且队尾被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value2 as *mut [usize] as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    assert_eq!(list.delete(&mut value3 as *mut [usize] as *mut ()), false);
    assert_eq!(list.pop(), Some(&mut value2 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), Some(&mut value1 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1且队尾被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value3 as *mut [usize] as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value2 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除不存在元素，链表元素 > 1且队尾的前驱被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    value2[0] = value2[0] | DELETE_MARK; // 手动标记value2
    assert_eq!(list.delete(&mut value3 as *mut [usize] as *mut ()), false); // delete中的search过程不会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value1 as *mut [usize] as *mut ())); // pop出value1的同时，其中的search过程还会删除被标记的value2
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 = 1
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), true);
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 = 1且目标被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), true);
    assert_eq!(list.pop(), Some(&mut value2 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1且目标被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK; // 手动标记value1
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), false); // delete中的search过程会删除被标记的value1
    assert_eq!(list.pop(), Some(&mut value2 as *mut [usize] as *mut ()));
    assert_eq!(list.pop(), None);

    // 删除存在元素，链表元素 > 1且目标的前驱被标记
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    value2[0] = value2[0] | DELETE_MARK; // 手动标记value2
    assert_eq!(list.delete(&mut value1 as *mut [usize] as *mut ()), true); // delete中的search过程会删除被标记的value2
    assert_eq!(list.pop(), None);
}

#[test]
#[allow(unused_assignments)]
/// 测试search函数是否能正常地删除搜索元素旁边的标记元素
fn test_search() {
    use linked_list::DELETE_MARK;

    let mut value1: [usize; 2] = [0; 2];
    let mut value2: [usize; 2] = [0; 2];
    let mut value3: [usize; 2] = [0; 2];
    let list = linked_list::LinkedList::new();

    // 标记情况：无、无、无
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &mut value1 as *mut [usize] as *mut ());
    assert!(right_node.ptr() == &mut value2 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value1-->value2-->value3-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value1 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value1 as *const [usize] as *const () as *const usize) },
        &value2 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value2 as *const [usize] as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}

    // 标记情况：无、无、有
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value3[0] = value3[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &mut value1 as *mut [usize] as *mut ());
    assert!(right_node.ptr() == &mut value2 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value1-->value2-->value3(marked)-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value1 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value1 as *const [usize] as *const () as *const usize) },
        &value2 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value2 as *const [usize] as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR | DELETE_MARK
    );
    while let Some(_) = list.pop() {}

    // 标记情况：无、有、无
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value2[0] = value2[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &mut value1 as *mut [usize] as *mut ());
    assert!(right_node.ptr() == &mut value3 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value1-->value3-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value1 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value1 as *const [usize] as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}

    // 标记情况：无、有、有
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value2[0] = value2[0] | DELETE_MARK;
    value3[0] = value3[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &mut value1 as *mut [usize] as *mut ());
    assert!(right_node.ptr() == NULL_PTR as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value1-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value1 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value1 as *const [usize] as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}

    // 标记情况：有、无、无
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &list as *const LinkedList as *const () as *mut ());
    assert!(right_node.ptr() == &mut value2 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value2-->value3-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value2 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value2 as *const [usize] as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}

    // 标记情况：有、无、有
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK;
    value3[0] = value3[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &list as *const LinkedList as *const () as *mut ());
    assert!(right_node.ptr() == &mut value2 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value2-->value3(marked)-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value2 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value2 as *const [usize] as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR | DELETE_MARK
    );
    while let Some(_) = list.pop() {}

    // 标记情况：有、有、无
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK;
    value2[0] = value2[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &list as *const LinkedList as *const () as *mut ());
    assert!(right_node.ptr() == &mut value3 as *mut [usize] as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->value3-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        &value3 as *const [usize] as *const () as usize
    );
    assert_eq!(
        unsafe { *(&value3 as *const [usize] as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}

    // 标记情况：有、有、有
    unsafe { list.push(&mut value3 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value2 as *mut [usize] as *mut ()) };
    unsafe { list.push(&mut value1 as *mut [usize] as *mut ()) };
    value1[0] = value1[0] | DELETE_MARK;
    value2[0] = value2[0] | DELETE_MARK;
    value3[0] = value3[0] | DELETE_MARK;
    let (left_node, right_node) = list.search_with_ptr(&mut value2 as *mut [usize] as *mut ());
    assert!(left_node.ptr() == &list as *const LinkedList as *const () as *mut ());
    assert!(right_node.ptr() == NULL_PTR as *mut ());
    drop(left_node);
    drop(right_node);
    // list: head-->NULL
    assert_eq!(
        unsafe { *(&list as *const LinkedList as *const () as *const usize) },
        NULL_PTR
    );
    while let Some(_) = list.pop() {}
}

#[test]
fn test_linked_list_concurrent() {
    use std::sync::Arc;
    use std::thread;

    // 重复测试的次数，用于进行批量测试以检测死锁等情况
    // 在多次运行时，建议增加参数`-- --nocapture`，从而显示打印的已完成次数
    const TEST_NUM: usize = 100;
    // const TEST_NUM: usize = 1;

    const NUM_PRODUCERS: usize = 20;
    const NUM_DELETE_CONSUMERS: usize = 10;
    const NUM_POP_CONSUMERS: usize = 10;
    const NUM_DATA_PER_THREAD: usize = 500;
    assert!(NUM_PRODUCERS == NUM_DELETE_CONSUMERS + NUM_POP_CONSUMERS);

    let mut temp: [usize; 2] = [0; 2];
    temp[0] = 3;

    for current_num in 0..TEST_NUM {
        let mut handles =
            Vec::with_capacity(NUM_PRODUCERS + NUM_DELETE_CONSUMERS + NUM_POP_CONSUMERS);
        let values: Arc<[[usize; 2]; NUM_PRODUCERS * NUM_DATA_PER_THREAD]> =
            Arc::new([[0; 2]; NUM_PRODUCERS * NUM_DATA_PER_THREAD]);
        // 用于记录values的每个位置被从链表中取出了几次
        let pop_nums: Arc<[AtomicUsize; NUM_PRODUCERS * NUM_DATA_PER_THREAD]> =
            Arc::new([const { AtomicUsize::new(0) }; NUM_PRODUCERS * NUM_DATA_PER_THREAD]);
        let list = Arc::new(linked_list::LinkedList::new());

        for i in 0..NUM_PRODUCERS {
            let l = list.clone();
            let v = values.clone();
            handles.push(thread::spawn(move || {
                let mut value_ptr: [*mut (); NUM_DATA_PER_THREAD] =
                    [null_mut(); NUM_DATA_PER_THREAD];
                for j in 0..NUM_DATA_PER_THREAD {
                    value_ptr[j] = ((v.as_ptr() as *const () as usize)
                        + i * NUM_DATA_PER_THREAD * size_of::<usize>() * 2
                        + j * size_of::<usize>() * 2) as *mut ();
                }

                for j in 0..NUM_DATA_PER_THREAD {
                    unsafe {
                        l.push(value_ptr[j]);
                    }
                }
            }));
        }

        for i in 0..NUM_DELETE_CONSUMERS {
            let l = list.clone();
            let v = values.clone();
            let p = pop_nums.clone();
            handles.push(thread::spawn(move || {
                let mut value_ptr: [*mut (); NUM_DATA_PER_THREAD] =
                    [null_mut(); NUM_DATA_PER_THREAD];
                for j in 0..NUM_DATA_PER_THREAD {
                    value_ptr[j] = ((v.as_ptr() as *const () as usize)
                        + i * NUM_DATA_PER_THREAD * size_of::<usize>() * 2
                        + j * size_of::<usize>() * 2) as *mut ();
                }

                let mut j = 0; // 删除计数
                while j < NUM_DATA_PER_THREAD {
                    if l.delete(value_ptr[j]) {
                        // 删除指定位置成功
                        unsafe {
                            // assert!(*((value_ptr[j] as *mut usize).add(1)) == 0);
                            // *(value_ptr[j] as *mut usize) = usize::MAX;
                            *(value_ptr[j] as *mut usize) =
                                &temp as *const usize as *const () as usize;
                        }
                        p[i * NUM_DATA_PER_THREAD + j].fetch_add(1, Ordering::AcqRel);
                        j += 1; // 只有删除成功才会增加删除计数
                    } else {
                        if let Some(ptr) = l.pop() {
                            // 删除指定位置失败，因此改为pop一个元素，以确保每个消费者删除的元素数量恒定
                            unsafe {
                                // assert!(*((ptr as *mut usize).add(1)) == 0);
                                // *(ptr as *mut usize) = usize::MAX;
                                *(ptr as *mut usize) = &temp as *const usize as *const () as usize;
                            }
                            let offset = (ptr as usize - v.as_ptr() as *const () as usize)
                                / size_of::<usize>()
                                / 2;
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
                        unsafe {
                            // assert!(*((ptr as *mut usize).add(1)) == 0);
                            // *(ptr as *mut usize) = usize::MAX;
                            *(ptr as *mut usize) = &temp as *const usize as *const () as usize;
                        }
                        let offset = (ptr as usize - v.as_ptr() as *const () as usize)
                            / size_of::<usize>()
                            / 2;
                        p[offset].fetch_add(1, Ordering::AcqRel);
                        j += 1; // 只有删除成功才会增加删除计数
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(list.is_empty()); // 验证列表为空
        for i in 0..NUM_PRODUCERS * NUM_DATA_PER_THREAD {
            assert!(pop_nums[i].load(Ordering::Acquire) == 1); // 验证所有元素恰好被取出一次。
        }

        // 用于多次运行，打印出当前已完成的次数
        println!("finished num: {}", current_num + 1);
    }
}
