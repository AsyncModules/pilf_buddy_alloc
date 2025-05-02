//! 这个库是直接参考 rcore-os 组织下的 [buddy_system_allocator](https://github.com/rcore-os/buddy_system_allocator)
//! 实现的位置无关的堆分配器，用在 vDSO 中
//! list_tests.rs 中进行了简单的测试以及大规模的并发测试
//! heap_tests.rs 中对无锁堆分配器进行了简单的测试，没有进行大规模并发测试
#![cfg_attr(not(test), no_std)]

mod imp;
mod linked_list;
pub use imp::LockFreeHeap;
pub use linked_list::LinkedList;

#[cfg(test)]
mod list_tests;

#[cfg(test)]
mod heap_tests;

pub fn get_data_base() -> usize {
    crate_interface::call_interface!(pi_pointer::GetDataBase::get_data_base)
}
