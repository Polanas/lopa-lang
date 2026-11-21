#[macro_export]
macro_rules! notify {
    ($db:expr, $($input:tt)*) => {
        salsa::attach($db, || {
            notify_rust::Notification::new().body(&format!($($input)*)).show().unwrap();
        });
    };
}
