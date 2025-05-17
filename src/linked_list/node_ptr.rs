use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use pi_pointer::{AtomicWrappedPtr, PIPtr, WrappedPtr, NULL_PTR};

// 此处，使用了指针的最低位作为标记。
// 为了保证这样带标记的指针能够进行正常的位置无关地址转换，
// 从get_data_base获取的基地址需要至少按2字节对齐。
#[derive(Copy, Clone)]
pub(crate) struct MarkedPtr<T: WrappedPtr>(T);
pub(crate) const DELETE_MARK: usize = 0b1;

impl<T> WrappedPtr for MarkedPtr<T>
where
    T: WrappedPtr,
{
    fn value(&self) -> *mut () {
        self.0.value()
    }

    fn ptr(&self) -> *mut () {
        T::from_value(self.unmark()).ptr()
    }

    fn from_value(value: *mut ()) -> Self {
        Self(T::from_value(value))
    }

    fn from_ptr(ptr: *mut ()) -> Self {
        Self(T::from_ptr(ptr))
    }

    fn set(&mut self, value: *mut ()) {
        self.0.set(value)
    }

    fn is_null(&self) -> bool {
        T::from_value(self.unmark()).is_null()
    }
}

impl<T> MarkedPtr<T>
where
    T: WrappedPtr,
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
        let mark = (self.0.value() as usize) & DELETE_MARK;
        MarkedPtr(((self.ptr() as usize) | mark) as *mut ())
    }
}

/// 该类型代表一个链表节点。
/// 其指针字段可以看作一个可能带有标记的、地址无关的、原子的指针。
/// 其引用计数字段用于避免其它线程正在访问节点时，某个线程释放了该节点。
/// 注意：应该通过NodePtr访问ListNode，以正确维护引用计数。
pub(crate) struct ListNode {
    ptr: AtomicWrappedPtr<MarkedPtr<PIPtr>>,
    rc: AtomicUsize,
}

impl ListNode {
    /// 将指向该节点的指针转换为对该节点的引用
    /// 该函数中不需要地址转换，因为其不涉及将指针存储入节点。
    /// SAFETY: its_ptr需要指向有效的ListNode
    pub(crate) unsafe fn from_its_ptr(its_ptr: *mut ()) -> &'static Self {
        &*(its_ptr as *mut Self)
    }

    pub(crate) fn next(&'static self) -> Option<&'static Self> {
        let value = self.ptr.load();
        if value.is_null() {
            None
        } else {
            unsafe { Some(Self::from_its_ptr(value.ptr())) }
        }
    }

    pub(crate) fn rc_increase(&self) {
        self.rc.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn rc_decrease(&self) {
        self.rc.fetch_sub(1, Ordering::AcqRel);
        assert!(self.rc.load(Ordering::Acquire) & (usize::MAX - (usize::MAX >> 1)) == 0);
        // 溢出检测
    }

    pub(crate) fn rc(&self) -> usize {
        self.rc.load(Ordering::Acquire)
    }
}

// 暴露内部方法
impl ListNode {
    pub(crate) fn load_value(&self) -> *mut () {
        self.ptr.load_value()
    }

    pub(crate) fn load_ptr(&self) -> *mut () {
        self.ptr.load_ptr()
    }

    pub(crate) fn load(&self) -> MarkedPtr<PIPtr> {
        self.ptr.load()
    }

    pub(crate) fn from_value(value: *mut ()) -> Self {
        Self {
            ptr: AtomicWrappedPtr::from_value(value),
            rc: AtomicUsize::new(0),
        }
    }

    pub(crate) fn from_ptr(ptr: *mut ()) -> Self {
        Self {
            ptr: AtomicWrappedPtr::from_ptr(ptr),
            rc: AtomicUsize::new(0),
        }
    }

    pub(crate) const fn null() -> Self {
        Self {
            ptr: AtomicWrappedPtr::null(),
            rc: AtomicUsize::new(0),
        }
    }

    pub(crate) fn store(&self, value: *mut ()) {
        self.ptr.store(value);
    }

    pub(crate) fn compare_exchange(
        &self,
        current: *mut (),
        new: *mut (),
    ) -> Result<*mut (), *mut ()> {
        self.ptr.compare_exchange(current, new)
    }

    pub(crate) fn is_marked(&self) -> bool {
        self.ptr.load().is_marked()
    }

    pub(crate) fn mark(&self) -> *mut () {
        self.ptr.load().mark()
    }

    pub(crate) fn unmark(&self) -> *mut () {
        self.ptr.load().unmark()
    }

    pub(crate) fn marked_ptr(&self) -> NodePtr {
        NodePtr::from_marked_ptr(self.ptr.load().marked_ptr())
    }
}

/// 该类型代表指向链表节点的指针（且指针自身的位置不在链表上）。
/// 其值与有效指针的唯一区别是其可能带有标记。
/// 每一个指向节点的该类型指针，都会在其存在期间提升其指向节点的引用节点。
// #[derive(Copy, Clone)]
pub(crate) struct NodePtr(MarkedPtr<*mut ()>);

// 构造与析构函数，涉及引用计数的维护
impl NodePtr {
    pub(crate) fn from_value(value: *mut ()) -> Self {
        let self_ = Self(MarkedPtr::from_value(value));
        if let Some(node) = self_.pointed_node() {
            node.rc_increase();
        }
        self_
    }

    pub(crate) fn from_ptr(ptr: *mut ()) -> Self {
        let self_ = Self(MarkedPtr::from_ptr(ptr));
        if let Some(node) = self_.pointed_node() {
            node.rc_increase();
        }
        self_
    }

    pub(crate) fn from_marked_ptr(marked_ptr: MarkedPtr<*mut ()>) -> Self {
        let self_ = Self(marked_ptr);
        if let Some(node) = self_.pointed_node() {
            node.rc_increase();
        }
        self_
    }

    pub(crate) fn null() -> Self {
        Self(MarkedPtr::null())
    }

    /// 获取指针指向的下一个节点的指针
    /// 如果指针指向的节点值为NULL_PTR，返回Some(NULL_PTR)
    /// 如果指针自身值为NULL_PTR，返回None
    /// 与ListNode::next不同，该函数还包含将ListNode转化为NodePtr的过程。
    pub fn next(&self) -> Option<Self> {
        if let Some(node) = self.pointed_node() {
            let next = Self(node.ptr.load().marked_ptr());
            if let Some(node_) = next.pointed_node() {
                node_.rc_increase();
            }
            Some(next)
        } else {
            None
        }
    }
}

// 构造与析构函数，涉及引用计数的维护
impl Clone for NodePtr {
    fn clone(&self) -> Self {
        if let Some(node) = self.pointed_node() {
            node.rc_increase();
        }
        Self(self.0.clone())
    }
}

// 构造与析构函数，涉及引用计数的维护
impl Drop for NodePtr {
    fn drop(&mut self) {
        if let Some(node) = self.pointed_node() {
            node.rc_decrease();
        }
    }
}

impl NodePtr {
    /// 获取指针指向节点的引用
    /// 如果指针指向的节点值为NULL_PTR，返回Some
    /// 如果指针自身值为NULL_PTR，返回None
    pub(crate) fn pointed_node(&self) -> Option<&'static ListNode> {
        if self.is_null() {
            None
        } else {
            unsafe { Some(ListNode::from_its_ptr(self.ptr())) }
        }
    }

    /// 将指针转化为链表上存储的位置无关形式
    /// 可能带有标记
    pub fn linked_value(&self) -> *mut () {
        let mark = (self.0.value() as usize) & DELETE_MARK;
        (PIPtr::from_ptr(self.0.ptr()).value() as usize | mark) as *mut ()
    }
}

// 暴露内部方法
impl NodePtr {
    pub(crate) fn value(&self) -> *mut () {
        self.0.value()
    }

    pub(crate) fn ptr(&self) -> *mut () {
        self.0.ptr()
    }

    pub(crate) fn set(&mut self, value: *mut ()) {
        self.0.set(value);
    }

    pub(crate) fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub(crate) fn is_marked(&self) -> bool {
        self.0.is_marked()
    }

    pub(crate) fn mark(&self) -> *mut () {
        self.0.mark()
    }

    pub(crate) fn unmark(&self) -> *mut () {
        self.0.unmark()
    }
}
