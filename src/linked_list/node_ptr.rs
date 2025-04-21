use core::sync::atomic::{AtomicPtr, Ordering};

use crate::get_data_base;

/// 该类型代表一个链表节点。
/// 其值可以看作一个可能带有标记的、地址无关的、原子的指针。
pub(crate) type ListNode = MarkedPtr<PIPtr<AtomicPtr<()>>>;

pub(crate) const EMPTY_FLAG: *mut () = 0x74e as *mut ();
pub(crate) const DELETE_MARK: usize = 0b1;

#[repr(C)]
pub(crate) struct MarkedPtr<T>(T);

#[repr(C)]
pub(crate) struct PIPtr<T>(T);

pub(crate) trait LinkedPtr {
    /// 获取该节点的值，没有进行去标记和地址转换
    fn value(&self) -> *mut ();
    /// 获取该节点的值，且经过了某种将值变为有效的指针的转换。
    /// 最外层类型的ptr函数的返回值可以直接作为指针访问内存地址。
    fn ptr(&self) -> *mut ();
    /// 新建节点，直接将传入的值存入节点。
    fn from_value(value: *mut ()) -> Self;
    /// 新建节点，认为传入的值是指针，对其做ptr函数内变换的逆变换后存入节点。
    fn from_ptr(ptr: *mut ()) -> Self;
    /// 将指向该节点的指针转换为对该节点的引用
    /// 该函数中不需要地址转换，因为其不涉及将指针存储入节点。
    fn from_its_ptr(its_ptr: *mut ()) -> &'static Self;
    /// 修改该节点存储的值
    fn set(&self, value: *mut ());
    /// 对该节点存储的值做CAS操作
    fn compare_exchange(&self, current: *mut (), new: *mut ()) -> Result<*mut (), *mut ()>; // 这里的参数视为value

    /// 根据默认值（EMPTY_FLAG）新建节点
    fn default() -> Self
    where
        Self: Sized,
    {
        Self::from_value(EMPTY_FLAG)
    }
    /// 判断两个节点的值是否相等
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
    /// 获取该节点作为指针指向的下一个节点的引用
    fn next(&'static self) -> &'static Self {
        Self::from_its_ptr(self.ptr())
    }
}

impl<T> LinkedPtr for MarkedPtr<T>
where
    T: LinkedPtr,
{
    fn value(&self) -> *mut () {
        self.0.value()
    }

    fn ptr(&self) -> *mut () {
        ((self.0.value() as usize) & !DELETE_MARK) as *mut ()
    }

    fn from_value(value: *mut ()) -> Self {
        Self(T::from_value(value))
    }

    fn from_ptr(ptr: *mut ()) -> Self {
        Self(T::from_ptr(ptr))
    }

    fn from_its_ptr(its_ptr: *mut ()) -> &'static Self {
        T::from_its_ptr(its_ptr);
        unsafe { &*(its_ptr as *mut Self) }
    }

    fn set(&self, value: *mut ()) {
        self.0.set(value)
    }

    fn compare_exchange(&self, current: *mut (), new: *mut ()) -> Result<*mut (), *mut ()> {
        self.0.compare_exchange(current, new)
    }
}

impl<T> MarkedPtr<T>
where
    T: LinkedPtr,
{
    /// 判断自身是否有标记
    pub fn is_marked(&self) -> bool {
        (self.0.value() as usize) & DELETE_MARK != 0
    }

    /// 返回自身被标记后的值，不改变自身
    pub fn mark(&self) -> *mut () {
        ((self.0.value() as usize) | DELETE_MARK) as *mut ()
    }

    /// 返回自身去掉标记后的值，不改变自身
    pub fn unmark(&self) -> *mut () {
        ((self.0.value() as usize) & !DELETE_MARK) as *mut ()
    }

    /// 获取该节点的值，且经过了地址变换。
    /// 返回值与有效指针的唯一区别是返回值可能带有标记。
    pub fn marked_ptr(&self) -> MarkedPtr<*mut ()> {
        MarkedPtr(self.0.ptr())
    }
}

impl<T> LinkedPtr for PIPtr<T>
where
    T: LinkedPtr,
{
    fn value(&self) -> *mut () {
        self.0.value()
    }

    fn ptr(&self) -> *mut () {
        let ptr = self.0.ptr();
        if ptr == EMPTY_FLAG {
            ptr
        } else {
            unsafe { (ptr as usize + get_data_base()) as *mut () }
        }
    }

    fn from_value(value: *mut ()) -> Self {
        Self(T::from_value(value))
    }

    fn from_ptr(ptr: *mut ()) -> Self {
        if ptr == EMPTY_FLAG {
            Self(T::from_ptr(ptr))
        } else {
            Self(T::from_ptr(unsafe {
                (ptr as usize - get_data_base()) as *mut ()
            }))
        }
    }

    fn from_its_ptr(its_ptr: *mut ()) -> &'static Self {
        T::from_its_ptr(its_ptr);
        unsafe { &*(its_ptr as *mut Self) }
    }

    fn set(&self, value: *mut ()) {
        self.0.set(value)
    }

    fn compare_exchange(&self, current: *mut (), new: *mut ()) -> Result<*mut (), *mut ()> {
        self.0.compare_exchange(current, new)
    }
}

impl LinkedPtr for AtomicPtr<()> {
    fn value(&self) -> *mut () {
        self.load(Ordering::Acquire)
    }

    fn ptr(&self) -> *mut () {
        self.load(Ordering::Acquire)
    }

    fn from_value(value: *mut ()) -> Self {
        Self::new(value)
    }

    fn from_ptr(ptr: *mut ()) -> Self {
        Self::new(ptr)
    }

    fn from_its_ptr(its_ptr: *mut ()) -> &'static Self {
        unsafe { Self::from_ptr(its_ptr as *mut *mut ()) }
    }

    fn set(&self, value: *mut ()) {
        self.store(value, Ordering::Release);
    }

    fn compare_exchange(&self, current: *mut (), new: *mut ()) -> Result<*mut (), *mut ()> {
        self.compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
    }
}

/// 因为trait函数不能为const，因此只能单独实现这些const函数
impl ListNode {
    pub const fn const_default() -> Self {
        Self(PIPtr(AtomicPtr::new(EMPTY_FLAG)))
    }
}

/// 该类型代表指向链表节点的指针（且指针自身的位置不在链表上）。
/// 其与有效指针的唯一区别是其可能带有标记。
pub(crate) type NodePtr = MarkedPtr<*mut ()>;

impl MarkedPtr<*mut ()> {
    /// 判断自身是否有标记
    pub fn is_marked(&self) -> bool {
        (self.0 as usize) & DELETE_MARK != 0
    }

    /// 返回自身被标记后的值，不改变自身
    pub fn mark(&self) -> *mut () {
        ((self.0 as usize) | DELETE_MARK) as *mut ()
    }

    /// 返回自身去掉标记后的值，不改变自身
    pub fn unmark(&self) -> *mut () {
        ((self.0 as usize) & !DELETE_MARK) as *mut ()
    }

    pub fn value(&self) -> *mut () {
        self.0
    }

    pub fn ptr(&self) -> *mut () {
        ((self.0 as usize) & !DELETE_MARK) as *mut ()
    }

    pub fn from_value(value: *mut ()) -> Self {
        Self(value)
    }

    /// 根据默认值（EMPTY_FLAG）新建指针
    pub fn default() -> Self
    where
        Self: Sized,
    {
        Self::from_value(EMPTY_FLAG)
    }
    /// 判断两个指针的值是否相等
    pub fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }

    /// 获取指针指向的下一个节点的指针
    /// 与ListNode::next不同，该函数还包含将ListNode转化为NodePtr的过程。
    pub fn next(&self) -> Self {
        self.pointed_node().marked_ptr()
    }

    pub fn pointed_node(&self) -> &ListNode {
        ListNode::from_its_ptr(self.ptr())
    }

    /// 将指针转化为链表上存储的位置无关形式
    pub fn linked_value(&self) -> *mut () {
        if self.unmark() == EMPTY_FLAG {
            self.value()
        } else {
            unsafe { (self.value() as usize - get_data_base()) as *mut () }
        }
    }
}

impl Clone for NodePtr {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Copy for NodePtr {}
