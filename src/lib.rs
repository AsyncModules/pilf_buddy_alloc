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

/// SAFETY:
///
/// 直接针对整个模块进行测试的时候，可能会使用并行的测试，
/// 但是 `get_data_base` 是使用一个全局变量来存储数据段的基地址，
/// 在进行不同的测试时，可能会导致出错，因此需要单独对每个函数进行测试，而不是整体测试
/// 或者传递参数 `--test-threads=1`
#[cfg(test)]
mod heap_tests;

pub fn get_data_base() -> usize {
    crate_interface::call_interface!(pi_pointer::GetDataBase::get_data_base)
}
