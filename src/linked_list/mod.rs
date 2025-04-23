/// 位置无关的无锁侵入式链表
use crate::get_data_base;
use core::marker::PhantomData;
use core::{fmt, ptr};
// pub(crate) use node_ptr::EMPTY_FLAG;
use node_ptr::{ListNode, NodePtr};

#[allow(unused)]
mod node_ptr;

/// An intrusive linked list
///
/// A clean room implementation of the one used in CS140e 2018 Winter
///
/// Thanks Sergio Benitez for his excellent work,
/// See [CS140e](https://cs140e.sergio.bz/) for more information
///
/// 对该链表的无锁改造参考了论文[A Pragmatic Implementation of Non-Blocking Linked-Lists](https://timharris.uk/papers/2001-disc.pdf)
///
/// 各个链表操作的参数和返回值都是实际地址。
/// 将实际转换为地址无关地址的过程在链表内部完成。
// #[derive(Copy, Clone)]
pub struct LinkedList {
    /// 为了接近论文中的链表结构，将head也实现为节点。
    head: ListNode,
}

unsafe impl Send for LinkedList {}
unsafe impl Sync for LinkedList {}

impl LinkedList {
    pub(crate) const EMPTY_LIST: Self = Self::new();

    /// Create a new LinkedList
    pub const fn new() -> LinkedList {
        LinkedList {
            head: ListNode::null(),
        }
    }

    /// Return `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        let (_, right_node) = self.get_headptr_head();
        return right_node.is_null();
    }

    /// Push `item` to the front of the list
    /// SAFETY: item需要指向一个有效的内存地址
    pub unsafe fn push(&self, item: *mut ()) {
        loop {
            let (left_node, right_node) = self.get_headptr_head();
            let new_node = NodePtr::from_value(item);
            new_node
                .pointed_node()
                .unwrap()
                .store(right_node.linked_value());
            if left_node
                .pointed_node()
                .unwrap()
                .compare_exchange(right_node.linked_value(), new_node.linked_value())
                .is_ok()
            {
                return;
            }
        }
    }

    /// Try to remove the first item in the list
    pub fn pop(&self) -> Option<*mut ()> {
        let mut left_node = NodePtr::null();
        let mut right_node = NodePtr::null();
        let mut right_node_next = NodePtr::null();
        loop {
            // 查找与逻辑删除
            loop {
                (left_node, right_node) = self.get_headptr_head();
                if right_node.is_null() {
                    return None;
                }
                right_node_next = right_node.next().unwrap();
                if !right_node_next.is_marked() {
                    // 此处实际判断的是right_node节点是否被标记
                    if right_node
                        .pointed_node()
                        .unwrap()
                        .compare_exchange(
                            right_node_next.linked_value(),
                            NodePtr::from_value(right_node_next.mark()).linked_value(),
                        )
                        .is_ok()
                    {
                        break;
                    }
                }
            }
            // 物理删除
            if left_node
                .pointed_node()
                .unwrap()
                .compare_exchange(right_node.linked_value(), right_node_next.linked_value())
                .is_err()
            {
                let (_, _) = self.search_with_ptr(right_node.value());
                // 之后回到大循环，因为需要重新pop一项出来
            } else {
                assert!(!right_node.is_marked());
                return Some(right_node.value());
            }
        }
    }

    /// 从链表中查找指针所指的项并删除。
    /// 虽然没有显式地返回被删除的项，但算法保证每个项只会被删除一次，且函数返回时该项一定已被删除。
    /// 因此，可以认为调用该函数后，线程就拥有了被删除项。
    /// 返回值true代表链表中有所找项并成功删除；false代表没有所找项。
    /// 不会出现链表中有所找项但删除失败的情况。
    pub fn delete(&self, item: *mut ()) -> bool {
        let mut left_node = NodePtr::null();
        let mut right_node = NodePtr::null();
        let mut right_node_next = NodePtr::null();

        // 查找与逻辑删除
        loop {
            (left_node, right_node) = self.search_with_ptr(item);
            if right_node.is_null() {
                return false;
            }
            right_node_next = right_node.next().unwrap();
            if !right_node_next.is_marked() {
                // 此处实际判断的是right_node节点是否被标记
                if right_node
                    .pointed_node()
                    .unwrap()
                    .compare_exchange(
                        right_node_next.linked_value(),
                        NodePtr::from_value(right_node_next.mark()).linked_value(),
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }
        // 物理删除
        if left_node
            .pointed_node()
            .unwrap()
            .compare_exchange(right_node.linked_value(), right_node_next.linked_value())
            .is_err()
        {
            let (_, _) = self.search_with_ptr(right_node.value());
        }
        return true;
    }
}

// private函数
impl LinkedList {
    fn search_with_ptr(&self, item: *mut ()) -> (NodePtr, NodePtr) {
        // 两个返回值分别为left_node和right_node
        let mut left_node: NodePtr = NodePtr::null();
        let mut left_node_next: NodePtr = NodePtr::null();
        let mut right_node: NodePtr = NodePtr::null();
        loop {
            loop {
                let mut t: NodePtr =
                    NodePtr::from_value(&self.head as *const ListNode as *mut ListNode as *mut ());
                let mut t_next: NodePtr = self.head.marked_ptr();

                /* 1: Find left_node and right_node */
                loop {
                    if !t_next.is_marked() {
                        left_node = t;
                        left_node_next = t_next;
                    }
                    t = NodePtr::from_value(t_next.unmark());
                    if t.is_null() {
                        break;
                    }
                    t_next = t.next().unwrap();
                    // rust没有do-while，因此这样退出循环
                    if !(t_next.is_marked() || (t.ptr() != item)) {
                        // if的第二个条件，将原论文的按值查找改为了按指针查找
                        break;
                    }
                }
                right_node = t;

                /* 2: Check nodes are adjacent*/
                if left_node_next.value() == right_node.value() {
                    if !right_node.is_null() && right_node.pointed_node().unwrap().is_marked() {
                        break;
                    } else {
                        return (left_node, right_node);
                    }
                }

                /* 3: Remove one or more marked nodes */
                if left_node
                    .pointed_node()
                    .unwrap()
                    .compare_exchange(left_node_next.linked_value(), right_node.linked_value())
                    .is_ok()
                {
                    if !right_node.is_null() && right_node.pointed_node().unwrap().is_marked() {
                        break;
                    } else {
                        return (left_node, right_node);
                    }
                }
            }
        }
    }

    fn get_headptr_head(&self) -> (NodePtr, NodePtr) {
        // 两个返回值分别为&head和head
        let mut left_node: NodePtr = NodePtr::null();
        let mut left_node_next: NodePtr = NodePtr::null();
        let mut right_node: NodePtr = NodePtr::null();
        loop {
            loop {
                let mut t: NodePtr =
                    NodePtr::from_value(&self.head as *const ListNode as *mut ListNode as *mut ());
                let mut t_next: NodePtr = self.head.marked_ptr();

                /* 1: Find left_node and right_node */
                loop {
                    if !t_next.is_marked() {
                        left_node = t;
                        left_node_next = t_next;
                    }
                    t = NodePtr::from_value(t_next.unmark());
                    if t.is_null() {
                        break;
                    }
                    t_next = t.next().unwrap();
                    // rust没有do-while，因此这样退出循环
                    if !t_next.is_marked() {
                        break;
                    }
                }
                right_node = t;

                /* 2: Check nodes are adjacent*/
                if left_node_next.value() == right_node.value() {
                    if !right_node.is_null() && right_node.pointed_node().unwrap().is_marked() {
                        break;
                    } else {
                        return (left_node, right_node);
                    }
                }

                /* 3: Remove one or more marked nodes */
                if left_node
                    .pointed_node()
                    .unwrap()
                    .compare_exchange(left_node_next.linked_value(), right_node.linked_value())
                    .is_ok()
                {
                    if !right_node.is_null() && right_node.pointed_node().unwrap().is_marked() {
                        break;
                    } else {
                        return (left_node, right_node);
                    }
                }
            }
        }
    }
}
