use godot::prelude::*;

pub mod core;

struct MinesExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MinesExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct MinesCore {
    base: Base<Node>,
}

// 3. Lifecycle Methods (Like _init and _ready in GDScript)
#[godot_api]
impl INode for MinesCore {
    fn init(base: Base<Node>) -> Self {
        godot_print!("Rust MinesCore initialized successfully!");
        Self { base }
    }
}

// 4. Custom Functions to call from GDScript
#[godot_api]
impl MinesCore {
    // Adding a test function so you can verify it works
    #[func]
    pub fn test_rust_connection(&mut self) -> () {
        godot_print!("Hello from compiled Rust code!");
    }
}
