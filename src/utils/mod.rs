mod deref_once_lock;
pub use deref_once_lock::DerefOnceLock;

use std::{
    mem::{size_of, size_of_val, MaybeUninit},
    slice,
};

/// Call `closure` with `data` interpreted as [`&\[T\]`]. If the alignment of `data` is not sufficient for [`T`],
/// this will alloc a new aligned space and copy `data` into.
///
/// # Safety:
/// Same as `transmute::<F, T>`.
///
/// # Panic:
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
