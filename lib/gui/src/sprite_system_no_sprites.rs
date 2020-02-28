#![allow(missing_docs)]

use basegl::display::camera::Camera2d;
use basegl::display::navigation::navigator::Navigator;
use basegl::display::object::DisplayObject;
use basegl::display::object::DisplayObjectOps;
use basegl::display::symbol::geometry::Sprite;
use basegl::display::symbol::geometry::SpriteSystem;
use basegl::display::world::*;
use basegl::prelude::*;
use basegl::system::web::forward_panic_hook_to_console;
use basegl::system::web::set_stdout;
use basegl::system::web;
use nalgebra::Vector2;
use nalgebra::Vector3;
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
#[allow(dead_code)]
pub fn run_example_sprite_system_no_sprites() {
    forward_panic_hook_to_console();
    set_stdout();
    init(&WorldData::new(&web::body()));
}

fn init(world:&World) {
    let scene         = world.scene();
    let camera        = scene.camera()  ;
    let sprite_system = SpriteSystem::new(world);
    world.add_child(&sprite_system);
    world.on_frame(move |time_ms| {
        let _keep_alive = &sprite_system;
    }).forget();
}
