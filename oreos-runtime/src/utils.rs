#[macro_export]
macro_rules! install_defmt_timestamp {
    () => {
        #[$crate::defmt::panic_handler]
        fn defmt_panic() -> ! {
            loop {}
        }
        static __OREOS_TIME_INITIALIZED: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);

        fn __oreos_defmt_timestamp_micros() -> u64 {
            if __OREOS_TIME_INITIALIZED.load(::core::sync::atomic::Ordering::Relaxed) {
                $crate::embassy_time::Instant::now().as_micros()
            } else {
                0
            }
        }

        $crate::defmt::timestamp!("{=u64}", __oreos_defmt_timestamp_micros());
    };

    ($($panic:expr)?) => {
        #[$crate::defmt::panic_handler]
        fn defmt_panic() -> ! {
            loop {
                $($panic)?
            }
        }
        static __OREOS_TIME_INITIALIZED: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);

        fn __oreos_defmt_timestamp_micros() -> u64 {
            if __OREOS_TIME_INITIALIZED.load(::core::sync::atomic::Ordering::Relaxed) {
                $crate::embassy_time::Instant::now().as_micros()
            } else {
                0
            }
        }

        $crate::defmt::timestamp!("{=u64}", __oreos_defmt_timestamp_micros());
    };
}
