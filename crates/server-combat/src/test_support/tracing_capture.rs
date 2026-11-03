//! Scoped tracing capture that sees every callsite, whichever test thread hits it first.
//!
//! Workaround for tracing-core 0.1.36 (tokio-rs/tracing#3611): while only one dispatcher
//! is registered, a callsite's first registration caches interest from the registering
//! thread's default. A subscriber-less test thread then caches `Interest::never`
//! process-wide and a capture on another thread loses that callsite. Two permanent
//! dispatchers keep every registration on the locked path over all live dispatchers.
//! Retire when the `has_just_one` path evaluates the registered dispatcher.
use bevy::log::tracing::{Dispatch, Subscriber, subscriber::NoSubscriber};
use std::sync::OnceLock;

/// `tracing::subscriber::with_default`, isolated from other test threads' registrations.
pub fn with_default<T>(
    subscriber: impl Subscriber + Send + Sync + 'static,
    run: impl FnOnce() -> T,
) -> T {
    pin_dispatchers();
    bevy::log::tracing::subscriber::with_default(subscriber, run)
}

fn pin_dispatchers() {
    static PINS: OnceLock<[Dispatch; 2]> = OnceLock::new();
    PINS.get_or_init(|| {
        [
            Dispatch::new(NoSubscriber::default()),
            Dispatch::new(NoSubscriber::default()),
        ]
    });
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Entity;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> bevy::log::tracing_subscriber::fmt::MakeWriter<'a> for Buffer {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn capture_keeps_a_callsite_first_hit_by_a_thread_without_subscriber() {
        let buffer = Buffer::default();
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_max_level(bevy::log::tracing::Level::DEBUG)
            .with_writer(buffer.clone())
            .with_ansi(false)
            .finish();
        super::with_default(subscriber, || {
            std::thread::spawn(|| {
                crate::navigation::hazard_trace::emit(
                    "detected",
                    None,
                    Entity::from_bits(1),
                    || serde_json::Value::Null,
                )
            })
            .join()
            .unwrap();
            crate::navigation::hazard_trace::emit("detected", None, Entity::from_bits(2), || {
                serde_json::json!({})
            });
        });
        let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert!(
            text.contains("FOLLOWER_HAZARD_TRACE") && text.contains("\"actor\":2"),
            "capture lost a callsite another thread registered first: {text:?}"
        );
    }
}
