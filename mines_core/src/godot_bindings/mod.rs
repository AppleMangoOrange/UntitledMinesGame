pub mod godot_bindings;
pub mod logger;

use godot::prelude::*;
use logger::LOGGER;

struct GodotRust;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRust {
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace));
            log::info!("Godot logger initialising...");
        }
    }
}
