//! This module defines the `Container` struct and related functionality.

use crate::prelude::*;

use crate::component::operator::NativeComponent;
use crate::component::operator::Networked;
use crate::component::operator::SymbolType;
use crate::component::operator;
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
use ensogl::gui::component::animation;
use ensogl::gui::component;



// =============
// === Shape ===
// =============

/// Container frame shape definition.
///
/// Provides a backdrop and outline for visualisations. Can indicate the selection status of the
/// container.
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

/// Container overlay shape definition. Used to capture events over the visualisation within the
/// container.
pub mod overlay {
    use super::*;

    ensogl::define_shape_system! {
        (width:f32,height:f32,selected:f32,padding:f32) {
            let width_bg       = width.clone();
            let height_bg      = height.clone();
            let width_bg  : Var<Distance<Pixels>> = width_bg.into();
            let height_bg : Var<Distance<Pixels>> = height_bg.into();
            let radius    : Var<Distance<Pixels>> = padding.clone().into();
            let color_overlay = color::Rgba::new(1.0,0.0,0.0,0.000_000_1);
            let overlay    = Rect((&width_bg,&height_bg)).corners_radius(&radius);
            let overlay       = overlay.fill(color_overlay);

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
    pub toggle_fullscreen : frp::Source,
    pub set_size          : frp::Source<Option<Vector2<f32>>>,
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
            def toggle_fullscreen = source();
            def set_size          = source();
        };
        let network = visualization_events;
        Self {network,set_visibility,set_visualization,toggle_visibility,set_data,select,deselect,
              clicked,toggle_fullscreen,set_size}
    }
}



// ================================
// === Visualizations Container ===
// ================================

/// Container that wraps a `Visualization` for rendering and interaction in the GUI.
///
/// The API to interact with the visualization is exposed through the `ContainerFrp`.
#[derive(Clone,CloneRef,Debug,Derivative,Shrinkwrap)]
#[derivative(PartialEq)]
#[allow(missing_docs)]
pub struct Container {
    // The internals are split into two structs: `ContainerData` and `ContainerFrp`. The
    // `ContainerData` contains the actual data and logic for the `Container`. The `ContainerFrp`
    // contains the FRP api and network. This split is required to avoid creating cycles in the FRP
    // network: the FRP network holds `Rc`s to the `ContainerData` and thus must not live in the
    // same struct.
    #[derivative(PartialEq(compare_with="ref_eq"))]
    #[shrinkwrap(main_field)]
    pub data              : Rc<ContainerData>,
    #[derivative(PartialEq="ignore")]
    pub frp               : ContainerFrp,
}

fn ref_eq(obj_a:&Rc<ContainerData>, obj_b:&Rc<ContainerData>) -> bool {
    let ptr_a:*const _ = obj_a.as_ref();
    let ptr_b:*const _ = obj_b.as_ref();
    ptr_a == ptr_b
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
    shape_frame             : component::ShapeView<frame::Shape>,
    shape_overlay           : component::ShapeView<overlay::Shape>,
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
    pub fn set_visualization(&self, visualization:Visualization) {
        let vis_parent = &self.display_object_visualisation;
        visualization.display_object().set_parent(&vis_parent);

        self.visualization.replace(Some(visualization));
        self.init_visualization_properties();
    }

    fn update_shape_sizes(&self) {
        let overlay_shape = &self.shape_overlay.shape;
        let frame_shape   = &self.shape_frame.shape;
        let padding       = self.padding.get();
        let width         = self.size.get().x;
        let height        = self.size.get().y;
        frame_shape.width.set(width);
        frame_shape.height.set(height);
        frame_shape.padding.set(padding);
        frame_shape.sprite.size().set(Vector2::new(width, height));
        overlay_shape.width.set(width);
        overlay_shape.height.set(height);
        overlay_shape.padding.set(padding);
        overlay_shape.sprite.size().set(Vector2::new(width, height));

        let pos = Vector3::new(width/2.0,height/2.0, 0.0);
        frame_shape.set_position(pos);
        overlay_shape.set_position(pos);
    }

    fn set_size(&self, size:Vector3<f32>) {
        if let Some(vis) = self.visualization.borrow().as_ref() {
            vis.set_size(size.xy());
        }

        self.size.set(size.xy());
        self.update_shape_sizes();
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
        let shape_frame             = component::ShapeView::<frame::Shape>::new(&logger,scene);
        let shape_overlay           = component::ShapeView::<overlay::Shape>::new(&logger,scene);
        let scene                   = scene.clone_ref();
        let data                    = ContainerData {logger,visualization,size,display_object,shape_frame,
                                                     display_object_internal,padding,scene,shape_overlay,
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
        self.update_shape_sizes();
        self.display_object_internal.add_child(&self.data.shape_overlay);
        self.display_object_internal.add_child(&self.data.shape_frame);
        self.display_object_internal.add_child(&self.data.display_object_visualisation);
        self.display_object.add_child(&self.display_object_internal);
        self
    }

    fn init_frp(self) -> Self {
        let frp                 = &self.frp;
        let network             = &self.frp.network;
        let container_data      = &self.data;

        let frame_shape_data = container_data.shape_frame.shape.clone_ref();
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

            def _output_hide = container_data.shape_overlay.events.mouse_down.map(f!([frp](_) {
                frp.clicked.emit(())
            }));
        }
        self
    }
}

impl operator::Resizable for Container {
    fn set_size(&self, size:Vector3<f32>) {
        self.data.set_size(size);
    }

    fn size(&self) -> Vector3<f32>{
        Vector3::new(self.data.size.get().x,self.data.size.get().y, 0.0)
    }

    fn set_size_frp(&self) -> &frp::Source<Option<Vector2<f32>>> {
        &self.frp.set_size
    }
}

impl NativeComponent for Container {
    fn symbols(&self) -> Vec<SymbolType> {
        let shape_system_frame   = self.scene.shapes.shape_system(PhantomData::<frame::Shape>);
        let shape_system_overlay = self.scene.shapes.shape_system(PhantomData::<overlay::Shape>);
        let mut shapes =  vec![
            SymbolType::Main(shape_system_frame.shape_system.symbol.clone_ref()),
            SymbolType::Main(shape_system_overlay.shape_system.symbol.clone_ref())
        ];

        if let Some(vis) = self.data.visualization.borrow().as_ref() {
            shapes.extend(vis.symbols());
        };
        shapes
    }
}

impl Networked for Container {
    fn network(&self) -> &frp::Network {
        &self.frp.network
    }
}

impl display::Object for Container {
    fn display_object(&self) -> &display::object::Instance {
        &self.data.display_object
    }
}
