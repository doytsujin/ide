//! This module contains TextEditor, an UiComponent to edit Enso Modules or Text Files.

use crate::prelude::*;

use crate::view::notification::NotificationService;
use crate::view::temporary_panel::TemporaryPadding;
use crate::view::temporary_panel::TemporaryPanel;

use basegl::display::object::DisplayObjectOps;
use basegl::display::shape::text::glyph::font::FontRegistry;
use basegl::display::shape::text::text_field::TextField;
use basegl::display::shape::text::text_field::TextFieldProperties;
use basegl::display::world::*;
use enso_frp::io::Key;
use enso_frp::io::KeyboardActions;
use enso_frp::io::KeyMask;
use nalgebra::Vector2;
use nalgebra::zero;



// ==================
// === TextEditor ===
// ==================

shared! { TextEditor

/// TextEditor allows us to edit text files or Enso Modules. Extensible code highlighting is
/// planned to be implemented for it.
#[derive(Debug)]
pub struct TextEditorData {
    text_field           : TextField,
    padding              : TemporaryPadding,
    position             : Vector2<f32>,
    size                 : Vector2<f32>,
    controller           : controller::text::Handle,
    notification_service : NotificationService
}

impl {
    /// Saves text editor's content to file.
    pub fn save(&self) {
        let controller   = self.controller.clone();
        let file_path    = controller.file_path();
        let text         = self.text_field.get_content();
        let store_fut    = controller.store_content(text);
        let notification = self.notification_service.clone();
        notification.info("Saving file");
        executor::global::spawn(async move {
            if store_fut.await.is_err() {
                let message = format!("Failed to save file: {}", file_path);
                notification.error(&message);
            } else {
                notification.info("File saved");
            }
        });
    }
}}

impl TextEditor {
    /// Creates a new TextEditor.
    pub fn new
    ( notification_service : &NotificationService
    , world                : &World
    , controller           : controller::text::Handle
    , keyboard_actions     : &mut KeyboardActions) -> Self {
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

        let content_future       = controller.read_content();
        let text_field_weak      = text_field.downgrade();
        let notification_service = notification_service.clone();
        let notification         = notification_service.clone();
        executor::global::spawn(async move {
            if let Ok(content) = content_future.await {
                if let Some(text_field) = text_field_weak.upgrade() {
                    text_field.set_content(&content);
                    notification.info("File loaded");
                }
            }
        });
        world.add_child(&text_field);

        let data = TextEditorData
            {controller,text_field,padding,position,size,notification_service};
        Self::new_from_data(data).initialize(keyboard_actions)
    }

    fn initialize(self, keyboard_actions:&mut KeyboardActions) -> Self {
        let save_keys:KeyMask = [Key::Control, Key::Character("s".to_string())].iter().collect();
        let text_editor       = Rc::downgrade(&self.rc);
        keyboard_actions.set_action(save_keys,move |_| {
            if let Some(text_editor) = text_editor.upgrade() {
                text_editor.borrow().save();
            }
        });
        self.update();
        self
    }

    /// Modify the underlying TextEditorData.
    pub fn modify_data<F:FnMut(&mut TextEditorData)>(&mut self, mut f:F) {
        f(&mut self.rc.borrow_mut());
        self.update();
    }

    /// Updates the underlying display object, should be called after setting size or position.
    fn update(&self) {
        let data     = self.rc.borrow_mut();
        let z_origin = 0.0;
        let padding  = data.padding;
        let position = data.position;
        let position = Vector3::new(position.x + padding.left,position.y + padding.bottom,z_origin);
        data.text_field.set_position(position);
        // TODO: Set text field size once the size change gets supported.
        // https://app.zenhub.com/workspaces/enso-5b57093c92e09f0d21193695/issues/luna/ide/217
        // let padding  = Vector2::new(padding.left + padding.right, padding.top + padding.bottom);
        // self.text_field.set_size(self.dimensions - padding);
        data.text_field.update();
    }
}

impl TemporaryPanel for TextEditor {
    fn set_padding(&mut self, padding: TemporaryPadding) {
        self.modify_data(|data| data.padding = padding);
    }

    fn padding(&self) -> TemporaryPadding {
        self.rc.borrow().padding
    }

    fn set_size(&mut self, size:Vector2<f32>) {
        self.modify_data(|data| data.size = size);
    }

    fn size(&self) -> Vector2<f32> {
        self.rc.borrow_mut().text_field.size()
    }

    fn set_position(&mut self, position:Vector2<f32>) {
        self.modify_data(|data| data.position = position);
    }

    fn position(&self) -> Vector2<f32> {
        let position = self.rc.borrow().text_field.position();
        Vector2::new(position.x, position.y)
    }
}