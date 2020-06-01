//! This module defines the `Container` struct and related functionality.

use crate::prelude::*;

use crate::frp;
use crate::visualization::*;

use ensogl::data::color;
use ensogl::display::Attribute;
use ensogl::display::Buffer;
use ensogl::display::Sprite;
use ensogl::display::scene::Scene;
use ensogl::display::shape::*;
use ensogl::display::traits::*;
use ensogl::display;
use ensogl::gui::component;
use ensogl::display::layout::alignment;
use ensogl::gui::component::animation;


// =============
// === Shape ===
// =============

/// Canvas node shape definition.
pub mod frame {
    use super::*;

    ensogl::define_shape_system! {
        (width:f32,height:f32,selected:f32,padding:f32) {
            // TODO use style

            let width_bg       = width.clone();
            let height_bg      = height.clone();
            let width_bg  : Var<Distance<Pixels>> = width_bg.into();
            let height_bg : Var<Distance<Pixels>> = height_bg.into();
            let radius    : Var<Distance<Pixels>> = padding.clone().into();
            let color_bg      = color::Lcha::new(0.2,0.013,0.18,1.0);
            let background    = Rect((&width_bg,&height_bg)).corners_radius(&radius);
            let background    = background.fill(color::Rgba::from(color_bg));

            let frame_outer = Rect((&width_bg,&height_bg)).corners_radius(&radius);

            // +1 at the end to avoid aliasing artifacts.
            let width_frame_inner       =  &width  - &padding * Var::<f32>::from(2.0) * &selected + Var::<f32>::from(1.0);
            let height_frame_inner      =  &height - &padding * Var::<f32>::from(2.0) * &selected + Var::<f32>::from(1.0);
            let width_frame_inner  : Var<Distance<Pixels>> = width_frame_inner.into();
            let height_frame_inner : Var<Distance<Pixels>> = height_frame_inner.into();
            let inner_radius = &radius * (Var::<f32>::from(1.0) - &selected);
            let frame_inner = Rect((&width_frame_inner,&height_frame_inner)).corners_radius(&inner_radius);

            let frame = frame_outer.difference(frame_inner);
            let color_frame    = color::Lcha::new(0.72,0.5,0.22,1.0);
            let frame = frame.fill(color::Rgba::from(color_frame));

             let out = background + frame;

             out.into()
        }
    }
}

pub mod overlay {
    use super::*;

    ensogl::define_shape_system! {
        (width:f32,height:f32,selected:f32,padding:f32) {
            let width_bg       = width.clone();
            let height_bg      = height.clone();
            let width_bg  : Var<Distance<Pixels>> = width_bg.into();
            let height_bg : Var<Distance<Pixels>> = height_bg.into();
            let radius    : Var<Distance<Pixels>> = padding.clone().into();
            let color_overlay = color::Rgba::new(1.0,0.0,0.0,0.0000001);
            let background    = Rect((&width_bg,&height_bg)).corners_radius(&radius);
            let overlay       = background.clone();
            let overlay       = overlay.fill(color::Rgba::from(color_overlay));

            let out = overlay;

            out.into()
        }
    }
}



// ===========
// === FRP ===
// ===========

/// Event system of the `Container`.
#[derive(Clone,CloneRef,Debug)]
#[allow(missing_docs)]
pub struct ContainerFrp {
    pub network           : frp::Network,
    pub set_visibility    : frp::Source<bool>,
    pub toggle_visibility : frp::Source,
    pub set_visualization : frp::Source<Option<Visualization>>,
    pub set_data          : frp::Source<Option<Data>>,
    pub select            : frp::Source,
    pub deselect          : frp::Source,
    // TODO this should be a stream
    pub clicked           : frp::Source,
}

impl Default for ContainerFrp {
    fn default() -> Self {
        frp::new_network! { visualization_events
            def set_visibility    = source();
            def toggle_visibility = source();
            def set_visualization = source();
            def set_data          = source();
            def select            = source();
            def deselect          = source();
            def clicked           = source();
        };
        let network = visualization_events;
        Self {network,set_visibility,set_visualization,toggle_visibility,set_data,select,deselect,
              clicked}
    }
}



// ================================
// === Visualizations Container ===
// ================================

/// Container that wraps a `Visualization` for rendering and interaction in the GUI.
///
/// The API to interact with the visualization is exposed through the `ContainerFrp`.
#[derive(Clone,CloneRef,Debug,Shrinkwrap)]
#[allow(missing_docs)]
pub struct Container {
    // The internals are split into two structs: `ContainerData` and `ContainerFrp`. The
    // `ContainerData` contains the actual data and logic for the `Container`. The `ContainerFrp`
    // contains the FRP api and network. This split is required to avoid creating cycles in the FRP
    // network: the FRP network holds `Rc`s to the `ContainerData` and thus must not live in the
    // same struct.

    #[shrinkwrap(main_field)]
        data              : Rc<ContainerData>,
    pub frp               : ContainerFrp,
}

/// Internal data of a `Container`.
#[derive(Debug,Clone)]
#[allow(missing_docs)]
pub struct ContainerData {
    logger                       : Logger,
    size                         : Cell<Vector2<f32>>,
    padding                      : Cell<f32>,
    /// Topmost display object in the hierarchy. Used for global positioning.
    display_object               : display::object::Instance,
    /// Internal display object that will be sole child of `display_object` and can be attached and
    /// detached from its parent for showing/hiding all child shapes.
    display_object_internal      : display::object::Instance,
    /// Parent of the visualisation. Allows adding/removing of visualisations without affecting
    /// the order of other container shapes.
    display_object_visualisation : display::object::Instance,

    visualization           : RefCell<Option<Visualization>>,
    frame                   : component::ShapeView<frame::Shape>,
    overlay                 : component::ShapeView<overlay::Shape>,

    scene                   : Scene

}

impl ContainerData {
    /// Set whether the visualization should be visible or not.
    pub fn set_visibility(&self, is_visible:bool) {
        if is_visible {
            self.display_object_internal.set_parent(&self.display_object);
        } else {
            self.display_object_internal.unset_parent();
        }
    }

    /// Indicates whether the visualization is visible.
    pub fn is_visible(&self) -> bool {
        self.display_object_internal.has_parent()
    }

    /// Toggle visibility.
    fn toggle_visibility(&self) {
        self.set_visibility(!self.is_visible())
    }

    /// Update the content properties with the values from the `ContainerData`.
    ///
    /// Needs to called when a visualization has been set.
    fn init_visualization_properties(&self) {
        let size         = self.size.get();
        if let Some(vis) = self.visualization.borrow().as_ref() {
            vis.set_size(size);
        };
        self.set_visibility(true);
    }

    /// Set the visualization shown in this container.
    fn set_visualisation(&self, visualization:Visualization) {
        let vis_parent = &self.display_object_visualisation;
        visualization.display_object().set_parent(&vis_parent);

        self.visualization.replace(Some(visualization));
        self.init_visualization_properties();
    }
}

impl display::Object for ContainerData {
    fn display_object(&self) -> &display::object::Instance {
        &self.display_object
    }
}


impl Container {
    /// Constructor.
    pub fn new(scene:&Scene) -> Self {
        let logger                  = Logger::new("visualization");
        let visualization           = default();
        let size                    = Cell::new(Vector2::new(200.0, 200.0));
        let display_object          = display::object::Instance::new(&logger);
        let display_object_internal = display::object::Instance::new(&logger);
        let display_object_visualisation = display::object::Instance::new(&logger);

        let padding                 = Cell::new(10.0);
        let frame                   = component::ShapeView::<frame::Shape>::new(&logger,scene);
        let overlay                 = component::ShapeView::<overlay::Shape>::new(&logger,scene);
        let scene                   = scene.clone_ref();
        let data                    = ContainerData {logger,visualization,size,display_object,frame,
                                                     display_object_internal,padding,scene,overlay,
                                                     display_object_visualisation};
        let data                    = Rc::new(data);
        data.set_visualization(Registry::default_visualisation(&scene));
        data.set_visibility(false);
        let frp                     = default();
        Self {data,frp} . init() . init_frp()
    }

    fn init(self) ->  Self {
        self.init_shape()
    }

    fn init_shape(self) -> Self {
        // TODO avoid duplication
        let shape_system = self.scene.shapes.shape_system(PhantomData::<frame::Shape>);
        shape_system.shape_system.set_alignment(
            alignment::HorizontalAlignment::Center,
            alignment::VerticalAlignment::Center
        );

        let shape_system = self.scene.shapes.shape_system(PhantomData::<overlay::Shape>);
        shape_system.shape_system.set_alignment(
            alignment::HorizontalAlignment::Center,
            alignment::VerticalAlignment::Center
        );

        let overlay_shape = &self.data.overlay.shape;
        let frame_shape = &self.data.frame.shape;
        let padding     = self.data.padding.get();
        let width       = self.data.size.get().x;
        let height      = self.data.size.get().y;
        frame_shape.width.set(width + 2.0 * padding);
        frame_shape.height.set(height + 2.0 * padding);
        frame_shape.padding.set(padding);
        frame_shape.sprite.size().set(Vector2::new(width + 2.0 * padding, height + 2.0 * padding));
        frame_shape.selected.set(0.0);
        overlay_shape.width.set(width + 2.0 * padding);
        overlay_shape.height.set(height + 2.0 * padding);
        overlay_shape.padding.set(padding);
        overlay_shape.sprite.size().set(Vector2::new(width + 2.0 * padding, height + 2.0 * padding));
        overlay_shape.selected.set(0.0);

        frame_shape.mod_position(|t| t.x += width/2.0);
        frame_shape.mod_position(|t| t.y += height/2.0);
        overlay_shape.mod_position(|t| t.x += width/2.0);
        overlay_shape.mod_position(|t| t.y += height/2.0);

        self.display_object_internal.add_child(&self.data.overlay);
        self.display_object_internal.add_child(&self.data.frame);
        self.display_object_internal.add_child(&self.data.display_object_visualisation);
        self
    }

    fn set_selected(&self, value:bool) {
        if value {
            self.data.frame.shape.selected.set(1.0);
        } else{
            self.data.frame.shape.selected.set(0.0);
        }
    }

    fn init_frp(self) -> Self {
        let frp     = &self.frp;
        let network = &self.frp.network;
        let container_data = &self.data;

        let frame_shape_data = container_data.frame.shape.clone_ref();
        let selection = animation(network, move |value| {
            frame_shape_data.selected.set(value)
        });

        frp::extend! { network

            def _f_hide = frp.set_visibility.map(f!([container_data](is_visible) {
                container_data.set_visibility(*is_visible);
            }));

            def _f_toggle = frp.toggle_visibility.map(f!([container_data](_) {
                container_data.toggle_visibility()
            }));

            def _f_set_vis = frp.set_visualization.map(f!([container_data](visualization) {
                if let Some(visualization) = visualization.as_ref() {
                    container_data.set_visualization(visualization.clone());
                }
            }));

            def _f_set_data = frp.set_data.map(f!([container_data](data) {
                 container_data.visualization.borrow()
                    .for_each_ref(|vis| vis.frp.set_data.emit(data));
            }));

            def _select = frp.select.map(f!([selection](_) {
                 selection.set_target_position(1.0);
            }));

            def _deselect = frp.deselect.map(f!([selection](_) {
                 selection.set_target_position(0.0);
            }));

            def _output_hide = container_data.overlay.events.mouse_down.map(f!([frp](_) {
                frp.clicked.emit(())
            }));
        }
        self
    }
}

impl display::Object for Container {
    fn display_object(&self) -> &display::object::Instance {
        &self.data.display_object
    }
}
