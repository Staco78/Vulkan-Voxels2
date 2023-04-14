use std::{
    ops::{Deref, DerefMut},
    sync::OnceLock,
};

#[derive(Debug)]
pub struct DerefOnceLock<T, const ERROR_MSG: &'static str> {
    inner: OnceLock<T>,
}

impl<T, const ERROR_MSG: &'static str> Deref for DerefOnceLock<T, ERROR_MSG> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner.get().expect(ERROR_MSG)
    }
}
impl<T, const ERROR_MSG: &'static str> DerefMut for DerefOnceLock<T, ERROR_MSG> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.get_mut().expect(ERROR_MSG)
    }
}

impl<T, const ERROR_MSG: &'static str> DerefOnceLock<T, ERROR_MSG> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    #[inline(always)]
    pub fn inner(&self) -> &OnceLock<T> {
        &self.inner
    }
}
