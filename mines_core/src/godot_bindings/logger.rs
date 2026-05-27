use std::sync::atomic::{AtomicI64, Ordering};

use godot::classes::Engine;
use godot::prelude::*;
use log::{Level, Log, Metadata};

pub struct GodotLogger;
pub static LOGGER: GodotLogger = GodotLogger {};

static LOGGER_ID: AtomicI64 = AtomicI64::new(0);

impl Log for GodotLogger {
    #[inline]
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let message = Variant::from(format!("[{}] {}", record.target(), record.args()));

        let level_str = match record.level() {
            Level::Debug => "debug",
            Level::Error => "error",
            Level::Info => "info",
            Level::Trace => "verbose",
            Level::Warn => "warn",
        };

        let cached_id = LOGGER_ID.load(Ordering::Relaxed);
        if let Some(instance_id) = InstanceId::try_from_i64(cached_id) {
            if let Ok(mut logger_node) = Gd::<Node>::try_from_instance_id(instance_id) {
                logger_node.call(
                    level_str,
                    &[message.to_variant(), env!("CARGO_PKG_NAME").to_variant()],
                );
                return;
            } else {
                LOGGER_ID.store(0, Ordering::Relaxed);
            }
        }

        if let Some(main_loop) = Engine::singleton().get_main_loop()
            && let Ok(tree) = main_loop.try_cast::<SceneTree>()
            && let Some(root) = tree.get_root()
            && let Some(mut logger_node) = root.try_get_node_as::<Node>("Log")
        {
            LOGGER_ID.store(logger_node.instance_id().to_i64(), Ordering::Relaxed);
            let module_name = env!("CARGO_PKG_NAME").to_variant();
            logger_node.call("add_module", &[module_name.clone()]);
            logger_node.call(level_str, &[message.to_variant(), module_name]);
            return;
        } else {
            godot_print!("Logger uninitialised. [{level_str}] {message}");
        }
    }

    fn flush(&self) {}
}
