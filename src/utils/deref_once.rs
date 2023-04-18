use std::{
    cell::OnceCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::OnceLock,
};

macro_rules! define_once {
    ($name: ident, $inner: ident) => {
        #[derive(Debug)]
        pub struct $name<T, const ERROR_MSG: &'static str> {
            inner: $inner<T>,
        }

        impl<T, const ERROR_MSG: &'static str> Deref for $name<T, ERROR_MSG> {
            type Target = T;
            fn deref(&self) -> &Self::Target {
                self.inner.get().expect(ERROR_MSG)
            }
        }
        impl<T, const ERROR_MSG: &'static str> DerefMut for $name<T, ERROR_MSG> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.get_mut().expect(ERROR_MSG)
            }
        }

        impl<T, const ERROR_MSG: &'static str> $name<T, ERROR_MSG> {
            #[inline(always)]
            pub const fn new() -> Self {
                Self {
                    inner: $inner::new(),
                }
            }

            #[inline(always)]
            pub fn inner(&self) -> &$inner<T> {
                &self.inner
            }
        }
    };
}

define_once!(DerefOnceLock, OnceLock);
define_once!(DerefOnceCell, OnceCell);
