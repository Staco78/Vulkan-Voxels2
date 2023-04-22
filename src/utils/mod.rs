mod deref_once;
pub use deref_once::*;

use anyhow::Result;
use std::{
    mem::{size_of, size_of_val, MaybeUninit},
    ptr, slice,
};

/// Call `closure` with `data` interpreted as [`&\[T\]`]. If the alignment of `data` is not sufficient for [`T`],
/// this will alloc a new aligned space and copy `data` into.
///
/// # Safety
/// Same as `transmute::<F, T>`.
///
/// # Panic
/// Will panic if the size of [`T`] * `data.len()` is not divisible by the size of [`F`].
pub unsafe fn with_convert<F, T, C, R>(data: &[F], closure: C) -> R
where
    F: Copy,
    C: Fn(&[T]) -> R,
{
    let data_size = size_of_val(data);
    assert_eq!(data_size % size_of::<T>(), 0);
    let (a, b, c) = unsafe { data.align_to::<T>() };
    if a.is_empty() && c.is_empty() {
        closure(b)
    } else {
        let new_size = data_size / size_of::<T>();
        let mut allocated_data = Box::<[T]>::new_uninit_slice(new_size);
        let slice = unsafe {
            slice::from_raw_parts_mut(
                allocated_data.as_mut_ptr() as *mut MaybeUninit<F>,
                data.len(),
            )
        };
        MaybeUninit::write_slice(slice, data);
        let data = unsafe { MaybeUninit::slice_assume_init_ref(&allocated_data) };
        closure(data)
    }
}

/// Drop `val` then call `closure` to compute the new value.
#[inline]
pub fn drop_then_new<T, C>(val: &mut T, closure: C) -> Result<()>
where
    C: Fn() -> Result<T>,
{
    unsafe {
        ptr::drop_in_place(val);
        let new = closure()?;
        ptr::write(val, new);
    }
    Ok(())
}

/// Call `closure` to initialize each element.
#[inline]
pub fn try_init_array<T, const N: usize, C>(closure: C) -> Result<[T; N]>
where
    C: Fn() -> Result<T>,
{
    let mut array: [MaybeUninit<T>; N] = MaybeUninit::uninit_array();
    for buff in array.iter_mut() {
        buff.write(closure()?);
    }
    Ok(unsafe { MaybeUninit::array_assume_init(array) })
}

/// Call `closure` to initialize each element.
#[inline]
#[allow(unused)]
pub fn init_array<T, const N: usize, C>(closure: C) -> [T; N]
where
    C: Fn() -> T,
{
    unsafe { try_init_array(|| Ok(closure())).unwrap_unchecked() }
}
