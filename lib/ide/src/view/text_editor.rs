//! This module contains TextEditor, an UiComponent to edit Enso Modules or Text Files.

use crate::prelude::*;

use crate::view::temporary_panel::TemporaryPanel;
use crate::view::temporary_panel::TemporaryPadding;
use crate::controller::text::ControllerHandle;

use basegl::display::object::DisplayObjectOps;
use basegl::display::shape::text::glyph::font::FontRegistry;
use basegl::display::shape::text::text_field::TextField;
use basegl::display::shape::text::text_field::TextFieldProperties;
use basegl::display::world::*;

use nalgebra::Vector2;
use nalgebra::zero;
use crate::todo::executor::JsExecutor;
use basegl::control::EventLoop;


// ==================
// === TextEditor ===
// ==================

/// TextEditor allows us to edit text files or Enso Modules. Extensible code highlighting is
/// planned to be implemented for it.
#[derive(Clone,Debug)]
pub struct TextEditor {
    text_field : TextField,
    padding    : TemporaryPadding,
    position   : Vector2<f32>,
    size       : Vector2<f32>,
    controller : ControllerHandle
}

impl TextEditor {
    /// Creates a new TextEditor.
    pub fn new(world:&World, controller:ControllerHandle) -> Self {
        let scene        = world.scene();
        let camera       = scene.camera();
        let screen       = camera.screen();
        let mut fonts    = FontRegistry::new();
        let font         = fonts.get_or_load_embedded_font("DejaVuSansMono").unwrap();
        let padding      = default();
        let position     = zero();
        let size         = Vector2::new(screen.width, screen.height);
        let black        = Vector4::new(0.0,0.0,0.0,1.0);
        let base_color   = black;
        let text_size    = 16.0;
        let properties   = TextFieldProperties {font,text_size,base_color,size};
        let text_field   = TextField::new(&world,properties);
        let event_loop   = EventLoop::new();
        let executor     = JsExecutor::new(event_loop);
        let controller_clone = controller.clone_ref();
        let text_field_clone = text_field.clone();
        executor.spawn(async move {
            async {
                println!("Inside async");
            }.await;
            println!("After async");
            controller_clone.store_content("Hello".into()).await.expect("Couldn't store content.");
            println!("Reached?");
            let content = controller_clone.read_content().await.expect("Couldn't read content.");
            text_field_clone.write(&content);
        }).unwrap();
        std::mem::forget(executor);
        world.add_child(&text_field);

        Self {controller,text_field,padding,position,size}.initialize()
    }

    fn initialize(self) -> Self {
        self.update();
        self
    }

    /// Updates the underlying display object.
    pub fn update(&self) {
        let padding  = self.padding;
        let position = self.position;
        let position = Vector3::new(position.x + padding.left, position.y + padding.bottom, 0.0);
        self.text_field.set_position(position);
        // TODO: set text field size once the property change will be supported.
        // let padding  = Vector2::new(padding.left + padding.right, padding.top + padding.bottom);
        // self.text_field.set_size(self.dimensions - padding);
        self.text_field.update();
    }
}

impl TemporaryPanel for TextEditor {
    fn set_padding(&mut self, padding: TemporaryPadding) {
        self.padding = padding;
    }

    fn padding(&self) -> TemporaryPadding {
        self.padding
    }

    fn set_size(&mut self, size:Vector2<f32>) {
        self.size = size;
        self.update();
    }

    fn size(&self) -> Vector2<f32> {
        self.text_field.size()
    }

    fn set_position(&mut self, position:Vector2<f32>) {
        self.position = position;
        self.update();
    }

    fn position(&self) -> Vector2<f32> {
        let position = self.text_field.position();
        Vector2::new(position.x, position.y)
    }
}
