#![no_std]

pub use noexcept_impl::abort_on_panic;

#[doc(hidden)]
pub mod __private {
    #[doc(hidden)]
    pub struct AbortOnDrop;

    impl Drop for AbortOnDrop {
        #[inline]
        fn drop(&mut self) {
            abort();
        }
    }

    #[inline]
    fn abort() -> ! {
        //debug_assert!(std::thread::panicking());
        panic!("panic inside of #[abort_on_panic]");
    }
}
