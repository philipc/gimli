#[cfg(feature = "read")]
use alloc::boxed::Box;
#[cfg(feature = "read")]
use alloc::vec::Vec;
use core::fmt;
use core::mem::MaybeUninit;
use core::ops;
use core::ptr;
use core::slice;

mod sealed {
    /// # Safety
    /// Implementer must not modify the content in storage.
    pub unsafe trait ArrayLikedSealed {
        type Storage;

        fn new_storage() -> Self::Storage;

        fn grow(_storage: &mut Self::Storage, _additional: usize) -> Result<(), CapacityFull> {
            Err(CapacityFull)
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct CapacityFull;
}

use sealed::*;

/// Marker trait for types that can be used as backing storage when a growable array type is needed.
///
/// This trait is sealed and cannot be implemented for types outside this crate.
pub trait ArrayLike: ArrayLikedSealed {
    /// Type of the elements being stored.
    type Item;

    #[doc(hidden)]
    fn as_slice(storage: &Self::Storage) -> &[MaybeUninit<Self::Item>];

    #[doc(hidden)]
    fn as_mut_slice(storage: &mut Self::Storage) -> &mut [MaybeUninit<Self::Item>];
}

// TODO sealed
#[allow(missing_docs)]
pub trait VecLike: VecLikeSealed {}

pub(crate) trait VecLikeStorage: Default + ops::Deref<Target = [Self::Item]> + ops::DerefMut {
    type Item;

    fn clear(&mut self);
    fn pop(&mut self) -> Option<Self::Item>;
    fn try_push(&mut self, value: Self::Item) -> Result<(), CapacityFull>;
    fn try_insert(&mut self, index: usize, element: Self::Item) -> Result<(), CapacityFull>;
    fn swap_remove(&mut self, index: usize) -> Self::Item;
}

#[allow(missing_docs)]
pub(crate) trait VecLikeSealed {
    type Storage: VecLikeStorage<Item = Self::Item>;
    type Item;
}

// SAFETY: does not modify the content in storage.
unsafe impl<T, const N: usize> ArrayLikedSealed for [T; N] {
    type Storage = [MaybeUninit<T>; N];

    fn new_storage() -> Self::Storage {
        // SAFETY: An uninitialized `[MaybeUninit<_>; _]` is valid.
        unsafe { MaybeUninit::uninit().assume_init() }
    }
}

impl<T, const N: usize> ArrayLike for [T; N] {
    type Item = T;

    fn as_slice(storage: &Self::Storage) -> &[MaybeUninit<T>] {
        storage
    }

    fn as_mut_slice(storage: &mut Self::Storage) -> &mut [MaybeUninit<T>] {
        storage
    }
}

#[cfg(feature = "read")]
impl<T, const N: usize> VecLike for [T; N] {}

#[cfg(feature = "read")]
impl<T, const N: usize> VecLikeSealed for [T; N] {
    type Storage = ArrayVec<[T; N]>;
    type Item = T;
}

#[cfg(feature = "read")]
// SAFETY: does not modify the content in storage.
unsafe impl<T, const N: usize> ArrayLikedSealed for Box<[T; N]> {
    type Storage = Box<[MaybeUninit<T>; N]>;

    fn new_storage() -> Self::Storage {
        // SAFETY: An uninitialized `[MaybeUninit<_>; _]` is valid.
        Box::new(unsafe { MaybeUninit::uninit().assume_init() })
    }
}

#[cfg(feature = "read")]
impl<T, const N: usize> ArrayLike for Box<[T; N]> {
    type Item = T;

    fn as_slice(storage: &Self::Storage) -> &[MaybeUninit<T>] {
        &storage[..]
    }

    fn as_mut_slice(storage: &mut Self::Storage) -> &mut [MaybeUninit<T>] {
        &mut storage[..]
    }
}

#[cfg(feature = "read")]
impl<T, const N: usize> VecLike for Box<[T; N]> {}

#[cfg(feature = "read")]
impl<T, const N: usize> VecLikeSealed for Box<[T; N]> {
    type Storage = ArrayVec<Box<[T; N]>>;
    type Item = T;
}

pub(crate) struct ArrayVec<A: ArrayLike> {
    storage: A::Storage,
    len: usize,
}

impl<A: ArrayLike> ArrayVec<A> {
    pub fn new() -> Self {
        Self {
            storage: A::new_storage(),
            len: 0,
        }
    }
}

impl<A: ArrayLike> VecLikeStorage for ArrayVec<A> {
    type Item = A::Item;

    fn clear(&mut self) {
        let ptr: *mut [A::Item] = &mut **self;
        // Set length first so the type invariant is upheld even if `drop_in_place` panicks.
        self.len = 0;
        // SAFETY: `ptr` contains valid elements only and we "forget" them by setting the length.
        unsafe { ptr::drop_in_place(ptr) };
    }

    fn try_push(&mut self, value: A::Item) -> Result<(), CapacityFull> {
        let mut storage = A::as_mut_slice(&mut self.storage);
        if self.len >= storage.len() {
            A::grow(&mut self.storage, 1)?;
            storage = A::as_mut_slice(&mut self.storage);
        }

        storage[self.len] = MaybeUninit::new(value);
        self.len += 1;
        Ok(())
    }

    fn try_insert(&mut self, index: usize, element: A::Item) -> Result<(), CapacityFull> {
        assert!(index <= self.len);

        let mut storage = A::as_mut_slice(&mut self.storage);
        if self.len >= storage.len() {
            A::grow(&mut self.storage, 1)?;
            storage = A::as_mut_slice(&mut self.storage);
        }

        // SAFETY: storage[index] is filled later.
        unsafe {
            let p = storage.as_mut_ptr().add(index);
            core::ptr::copy(p as *const _, p.add(1), self.len - index);
        }
        storage[index] = MaybeUninit::new(element);
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<A::Item> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: this element is valid and we "forget" it by setting the length.
            Some(unsafe { A::as_slice(&self.storage)[self.len].as_ptr().read() })
        }
    }

    fn swap_remove(&mut self, index: usize) -> A::Item {
        assert!(self.len > 0);
        A::as_mut_slice(&mut self.storage).swap(index, self.len - 1);
        self.pop().unwrap()
    }
}

#[cfg(feature = "read")]
impl<T> VecLike for Vec<T> {}

#[cfg(feature = "read")]
impl<T> VecLikeSealed for Vec<T> {
    type Storage = Vec<T>;
    type Item = T;
}

#[cfg(feature = "read")]
impl<T> VecLikeStorage for Vec<T> {
    type Item = T;

    fn clear(&mut self) {
        Vec::clear(self)
    }

    fn try_push(&mut self, value: T) -> Result<(), CapacityFull> {
        Vec::push(self, value);
        Ok(())
    }

    fn try_insert(&mut self, index: usize, element: T) -> Result<(), CapacityFull> {
        Vec::insert(self, index, element);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        Vec::pop(self)
    }

    fn swap_remove(&mut self, index: usize) -> T {
        Vec::swap_remove(self, index)
    }
}

impl<A: ArrayLike> Drop for ArrayVec<A> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<A: ArrayLike> Default for ArrayVec<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: ArrayLike> ops::Deref for ArrayVec<A> {
    type Target = [A::Item];

    fn deref(&self) -> &[A::Item] {
        let slice = &A::as_slice(&self.storage);
        debug_assert!(self.len <= slice.len());
        // SAFETY: valid elements.
        unsafe { slice::from_raw_parts(slice.as_ptr() as _, self.len) }
    }
}

impl<A: ArrayLike> ops::DerefMut for ArrayVec<A> {
    fn deref_mut(&mut self) -> &mut [A::Item] {
        let slice = &mut A::as_mut_slice(&mut self.storage);
        debug_assert!(self.len <= slice.len());
        // SAFETY: valid elements.
        unsafe { slice::from_raw_parts_mut(slice.as_mut_ptr() as _, self.len) }
    }
}

impl<A: ArrayLike> Clone for ArrayVec<A>
where
    A::Item: Clone,
{
    fn clone(&self) -> Self {
        let mut new = Self::default();
        for value in &**self {
            new.try_push(value.clone()).unwrap();
        }
        new
    }
}

impl<A: ArrayLike> PartialEq for ArrayVec<A>
where
    A::Item: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<A: ArrayLike> Eq for ArrayVec<A> where A::Item: Eq {}

impl<A: ArrayLike> fmt::Debug for ArrayVec<A>
where
    A::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

pub(crate) struct VecStorage<V: VecLike> {
    storage: V::Storage,
}

impl<T> From<VecStorage<Vec<T>>> for Vec<T> {
    fn from(storage: VecStorage<Vec<T>>) -> Self {
        storage.storage
    }
}

impl<V: VecLike> Default for VecStorage<V> {
    fn default() -> Self {
        Self {
            storage: V::Storage::default(),
        }
    }
}

impl<V: VecLike> Clone for VecStorage<V>
where
    V::Item: Clone,
{
    fn clone(&self) -> Self {
        let mut new = Self::default();
        for value in &*self.storage {
            new.storage.try_push(value.clone()).unwrap();
        }
        new
    }
}

impl<V: VecLike> ops::Deref for VecStorage<V> {
    type Target = V::Storage;

    fn deref(&self) -> &V::Storage {
        &self.storage
    }
}

impl<V: VecLike> ops::DerefMut for VecStorage<V> {
    fn deref_mut(&mut self) -> &mut V::Storage {
        &mut self.storage
    }
}

impl<V: VecLike> PartialEq for VecStorage<V>
where
    V::Item: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        &*self.storage == &*other.storage
    }
}

impl<V: VecLike> Eq for VecStorage<V> where V::Item: Eq {}

impl<V: VecLike> fmt::Debug for VecStorage<V>
where
    V::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.storage, f)
    }
}
