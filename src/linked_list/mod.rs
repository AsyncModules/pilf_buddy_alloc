/// 位置无关的无锁侵入式链表
use crate::get_data_base;
use core::marker::PhantomData;
use core::{fmt, ptr};
use node_ptr::{LinkedPtr, ListNode, NodePtr, EMPTY_FLAG};

mod node_ptr;

/// An intrusive linked list
///
/// A clean room implementation of the one used in CS140e 2018 Winter
///
/// Thanks Sergio Benitez for his excellent work,
/// See [CS140e](https://cs140e.sergio.bz/) for more information
// #[derive(Copy, Clone)]
pub struct LinkedList {
    /// 为了接近论文中的链表结构，将head也实现为节点。
    head: ListNode,
}

unsafe impl Send for LinkedList {}

// pub(crate) const EMPTY_FLAG: *mut usize = 0x74f as *mut usize;

impl LinkedList {
    /// Create a new LinkedList
    pub const fn new() -> LinkedList {
        LinkedList {
            head: ListNode::const_default(),
        }
    }

    /// Return `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        self.head == EMPTY_FLAG
    }

    /// Push `item` to the front of the list
    /// item 是相较于数据段的偏移，需要获取到实际的地址才可以进行操作
    pub unsafe fn push(&mut self, item: *mut usize) {
        *((item as usize + get_data_base()) as *mut usize) = self.head as usize; // 读
        // *item = self.head as usize;
        self.head = item; // 写（没有验证这次写和上次读的一致性，应该改成CAS操作？）
    }

    /// Try to remove the first item in the list
    pub fn pop(&mut self) -> Option<*mut usize> {
        match self.is_empty() { // 读
            true => None,
            false => {
                // Advance head pointer
                let item = self.head; // 读
                self.head =
                    unsafe { *((item as usize + get_data_base()) as *mut usize) as *mut usize }; // 写（没有验证这次写和上次读的一致性，应该改成CAS操作？）
                // self.head = unsafe { *item as *mut usize };
                Some(item)
            }
        }
    }

    // /// Return an iterator over the items in the list
    // pub fn iter(&self) -> Iter {
    //     Iter {
    //         curr: self.head,
    //         list: PhantomData,
    //     }
    // }

    // /// Return an mutable iterator over the items in the list
    // /// 这里的 prev 的设置还有点问题
    // pub fn iter_mut(&mut self) -> IterMut {
    //     IterMut {
    //         prev: unsafe { (&mut self.head as *mut *mut usize) as usize - get_data_base() }
    //             as *mut usize, // 我觉得这个设置没问题啊
    //         // prev: &mut self.head as *mut *mut usize as *mut usize,
    //         curr: self.head,
    //         list: PhantomData,
    //     } // 该函数中虽然进行了两次对`self`的读取，但其中的`&mut self.head`在函数执行过程中是不变的，因此不涉及同步问题？
    // }
}

// private函数
impl LinkedList {
    /// item使用地址无关指针形式
    fn search_with_ptr(&self, item: *mut ()) -> (NodePtr, NodePtr) { // 两个返回值分别为left_node和right_node
        let mut left_node: NodePtr = NodePtr::default();
        let mut left_node_next: NodePtr = NodePtr::default();
        let mut right_node: NodePtr = NodePtr::default();
        loop {
            loop {
                let mut t: NodePtr = NodePtr::from_value(&self.head as *const ListNode as *mut ListNode as *mut ());
                let mut t_next: NodePtr = self.head.marked_ptr();
    
                /* 1: Find left_node and right_node */
                loop {
                    if !t_next.is_marked() {
                        left_node = t;
                        left_node_next = t_next;
                    }
                    t = NodePtr::from_value(t_next.unmark());
                    if t.eq(&NodePtr::from_value(EMPTY_FLAG)) {
                        break;
                    } 
                    t_next = t.next();
                    // rust没有do-while，因此这样退出循环
                    if !(t_next.is_marked() || (t.ptr() != ListNode::from_value(item).ptr())) { // if的第二个条件，将原论文的按值查找改为了按指针查找
                        break;
                    }
                }
                right_node = t;
    
                /* 2: Check nodes are adjacent*/
                if left_node_next.eq(&right_node) {
                    if (!right_node.eq(&NodePtr::from_value(EMPTY_FLAG))) && right_node.pointed_node().is_marked() {
                        break;
                    }
                    else {
                        return (left_node, right_node);
                    }
                }
                
                /* 3: Remove one or more marked nodes */
                if (left_node.pointed_node().compare_exchange(left_node_next.linked_value(), right_node.linked_value())).is_ok() {
                    if (!right_node.eq(&NodePtr::from_value(EMPTY_FLAG))) && right_node.pointed_node().is_marked() {
                        break;
                    }
                    else {
                        return (left_node, right_node);
                    }
                }
            }
        }
    } 

    /// item使用地址无关指针形式
    fn get_headptr_head(&self) -> (NodePtr, NodePtr) { // 两个返回值分别为&head和head
        let mut left_node: NodePtr = NodePtr::default();
        let mut left_node_next: NodePtr = NodePtr::default();
        let mut right_node: NodePtr = NodePtr::default();
        loop {
            loop {
                let mut t: NodePtr = NodePtr::from_value(&self.head as *const ListNode as *mut ListNode as *mut ());
                let mut t_next: NodePtr = self.head.marked_ptr();
    
                /* 1: Find left_node and right_node */
                loop {
                    if !t_next.is_marked() {
                        left_node = t;
                        left_node_next = t_next;
                    }
                    t = NodePtr::from_value(t_next.unmark());
                    if t.eq(&NodePtr::from_value(EMPTY_FLAG)) {
                        break;
                    } 
                    t_next = t.next();
                    // rust没有do-while，因此这样退出循环
                    if !t_next.is_marked() { // if的第二个条件，将原论文的按值查找改为了按指针查找
                        break;
                    }
                }
                right_node = t;
    
                /* 2: Check nodes are adjacent*/
                if left_node_next.eq(&right_node) {
                    if (!right_node.eq(&NodePtr::from_value(EMPTY_FLAG))) && right_node.pointed_node().is_marked() {
                        break;
                    }
                    else {
                        return (left_node, right_node);
                    }
                }
                
                /* 3: Remove one or more marked nodes */
                if (left_node.pointed_node().compare_exchange(left_node_next.linked_value(), right_node.linked_value())).is_ok() {
                    if (!right_node.eq(&NodePtr::from_value(EMPTY_FLAG))) && right_node.pointed_node().is_marked() {
                        break;
                    }
                    else {
                        return (left_node, right_node);
                    }
                }
            }
        }
    } 
}

// impl fmt::Debug for LinkedList {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_list().entries(self.iter()).finish()
//     }
// }

// /// An iterator over the linked list
// /// Iter自身没有同步问题（不能共享），因此看其访问的节点是否一致即可
// pub struct Iter<'a> {
//     curr: *mut usize,
//     list: PhantomData<&'a LinkedList>,
// }

// impl<'a> Iterator for Iter<'a> {
//     type Item = *mut usize;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.curr == EMPTY_FLAG {
//             None
//         } else {
//             let item = self.curr; // 获得某节点指针
//             let next = unsafe { *((item as usize + get_data_base()) as *mut usize) as *mut usize }; // 访问某节点内容，因为只有一次读操作，因此没有同步问题
//             // let next = unsafe { *item as *mut usize };
//             self.curr = next;
//             Some(item)
//         }
//     }
// }

// /// Represent a mutable node in `LinkedList`
// pub struct ListNode {
//     prev: *mut usize,
//     curr: *mut usize,
// }

// /// 虽然对IterMut有所有权约束保证唯一，但可以从IterMut中取出几个ListNode再同时操作，因此ListNode没有唯一性，需要考虑同步问题。
// /// 甚至，在使用ListNode操作前，还需要检查prev是否依然指向curr。
// impl ListNode {
//     /// Remove the node from the list
//     /// 不用给出实际的地址，只给出偏移量
//     pub fn pop(self) -> *mut usize {
//         // Skip the current one
//         // 这句先读了本节点，再写了上一节点。需要考虑同步问题。
//         unsafe {
//             *((self.prev as usize + get_data_base()) as *mut usize) =
//                 *((self.curr as usize + get_data_base()) as *mut usize);
//         }
//         self.curr
//     }

//     /// Returns the pointed address
//     /// 不用给出实际的地址，只给出偏移量
//     pub fn value(&self) -> *mut usize {
//         self.curr
//     }
// }

// /// A mutable iterator over the linked list
// pub struct IterMut<'a> {
//     list: PhantomData<&'a mut LinkedList>,
//     prev: *mut usize,
//     curr: *mut usize,
// }

// // 同样，对IterMut也不需考虑自身字段的同步问题。
// impl<'a> Iterator for IterMut<'a> {
//     type Item = ListNode;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.curr == EMPTY_FLAG {
//             None
//         } else {
//             let res = ListNode {
//                 prev: self.prev,
//                 curr: self.curr,
//             };
//             self.prev = self.curr;
//             self.curr =
//                 unsafe { *((self.curr as usize + get_data_base()) as *mut usize) as *mut usize }; // 只有一次读操作，因此没有同步问题
//             Some(res)
//         }
//     }
// }
