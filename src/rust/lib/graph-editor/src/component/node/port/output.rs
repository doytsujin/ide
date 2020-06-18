//! Implements the segmented output port area.
use crate::prelude::*;

use ensogl::display::traits::*;

use enso_frp as frp;
use enso_frp;
use ensogl::data::color;
use ensogl::display::Attribute;
use ensogl::display::Buffer;
use ensogl::display::Sprite;
use ensogl::display::scene::Scene;
use ensogl::display;
use ensogl::gui::component::Animation;
use ensogl::gui::component::Tween;
use ensogl::gui::component;
use ensogl::math::algebra::Clamp;

use crate::node;



// =================
// === Constants ===
// =================
// TODO: These values should be in some IDE configuration.

const BASE_SIZE           : f32 = 0.5;
const HIGHLIGHT_SIZE      : f32 = 1.0;
const SEGMENT_GAP_WIDTH   : f32 = 2.0;

const SHOW_DELAY_DURATION : f32 = 150.0;
const HIDE_DELAY_DURATION : f32 = 150.0;



// ==============
// === Shapes ===
// ==============

/// Helper trait that allows us to abstract the API of the `multi_port_area::Shape` and the
/// `single_port_area::Shape`. This is needed to avoid code duplication for functionality that can
/// work with either shape.
#[allow(missing_docs)]
trait PortShapeApi {
    fn set_grow(&self, grow_value:f32);
    fn set_opacity(&self, opacity:f32);
}

/// The port area shape is based on a single shape that gets cropped to show the appropriate
/// segment.
///
/// The base shape looks roughly like this:
/// ```text
///      .                            .
///      .                            .
///       .                          .
///          .                    .
///              """""""""""""
///     |  r    |              |   r   |
///     |            width             |
/// ```
/// where r is the radius of the left and right quarter circle shapes.
///
pub mod multi_port_area {
    use super::*;
    use ensogl::display::shape::*;
    use std::f32::consts::PI;

    /// Return 1.0 if `lower_bound` < `value` <= `upper_bound`, 0.0 otherwise.
    fn in_range(value:&Var<f32>, lower_bound:&Var<f32>, upper_bound:&Var<f32>) -> Var<f32> {
        Var::<f32>::from(format!("(step(float({1}),float({0})) - step(float({2}),float({0})))", value, lower_bound, upper_bound))
    }

    /// Return 1.0 if `lower_bound` < `value` <, `upper_bound`, 0.0 otherwise.
    fn in_range_inclusive(value:&Var<f32>, lower_bound:&Var<f32>, upper_bound:&Var<f32>) -> Var<f32> {
        Var::<f32>::from(format!("(step(float({1}),float({0})) * step(float({0}),float({2})))", value, lower_bound, upper_bound))
    }

    /// Compute the rotation of the plane along the shape border. The plane needs to be rotated
    /// to be perpendicular with the outer shape border. That means, it needs to be rotate in the
    /// segments that have a curved path, and needs to be perpendicular in the inner segment.
    fn compute_crop_plane_angle(full_shape_border_length:&Var<f32>, corner_segment_length:&Var<f32>, position:&Var<f32>) ->Var<f32> {
        // TODO implement proper abstraction for non-branching "if/then/else" or "case" in shaderland
        // Here we use a trick to use a pseudo-boolean float that is either 0 or 1 to multiply a
        // value that should be returned, iff it's case is true. That way we can add return values
        // of different "branches" of which exactly one will be non-zero.

        let start                      = 0.0.into();
        let middle_segment_start_point = corner_segment_length;
        let middle_segment_end_point   = full_shape_border_length - corner_segment_length;
        let end                        = full_shape_border_length;

        let default_rotation           = Var::<f32>::from(90.0_f32.to_radians());

        let case_1_pseudo_bool      = in_range(position, &start, &middle_segment_start_point);
        let case_1_value_normalised = Var::<f32>::from(1.0) - (position / corner_segment_length).clamp(0.0.into(), 1.0.into());
        let case_1_scale            = 90.0_f32.to_radians();
        let case_1_value            = case_1_pseudo_bool * (case_1_value_normalised * case_1_scale + &default_rotation);

        let case_2_pseudo_bool = in_range_inclusive(position, &middle_segment_start_point, &middle_segment_end_point);
        let case_2_value_base  = &default_rotation;
        let case_2_value       = case_2_pseudo_bool * case_2_value_base;

        // Case 3
        let case_3_pseudo_bool      = in_range_inclusive(position, &middle_segment_end_point, &end);
        let case_3_value_normalised = ((position - middle_segment_end_point) / corner_segment_length).clamp(0.0.into(), 1.0.into());
        let case_3_scale            = (-90.0_f32).to_radians();
        let case_3_value            = case_3_pseudo_bool * (case_3_value_normalised * case_3_scale + &default_rotation);

        case_1_value + case_2_value + case_3_value
    }

    /// Returns a value between 0 and 1 that indicates the position along the straight center segment.
    fn calculate_crop_plane_position_relative_to_center_segment(full_shape_border_length:&Var<f32>, corner_segment_length:&Var<f32>, position:&Var<f32>) -> Var<f32> {
        // TODO implement proper abstraction for non-branching "if/then/else" or "case" in shaderland
        // See function above for explanation of the branching.

        let middle_segment_start_point = corner_segment_length;
        let middle_segment_end_point   = full_shape_border_length - corner_segment_length;
        let end                        = full_shape_border_length;

        // Case 1: always zero can be ignored

        let case_2_pseudo_bool      = in_range_inclusive(position, &middle_segment_start_point, &middle_segment_end_point);
        let case_2_value_normalised = (position - middle_segment_start_point) / (&middle_segment_end_point - middle_segment_start_point);
        let case_2_value            = case_2_pseudo_bool * case_2_value_normalised;

        let case_3_pseudo_bool = in_range_inclusive(position, &middle_segment_end_point, &end);
        let case_3_value       = case_3_pseudo_bool * 1.0;

        case_2_value + case_3_value

    }

    /// Compute the plane at the location of the given port index.
    fn compute_crop_plane(index:&Var<f32>, port_num: &Var<f32>, width: &Var<f32>, corner_radius:&Var<f32>, position_offset:&Var<f32>) -> AnyShape {
        let corner_circumference     = corner_radius * 2.0 * PI;
        let corner_segment_length    = &corner_circumference * 0.25;
        let center_segment_length    = width - corner_radius * 2.0;
        let full_shape_border_length = &center_segment_length + &corner_segment_length * 2.0;


        let position_relative = index / port_num;
        let crop_segment_pos  = &position_relative * &full_shape_border_length + position_offset;

        let crop_plane_pos_relative = calculate_crop_plane_position_relative_to_center_segment(&full_shape_border_length, &corner_segment_length, &crop_segment_pos);
        let crop_plane_pos          = crop_plane_pos_relative * &center_segment_length + corner_radius;

        let plane_rotation_angle = compute_crop_plane_angle(&full_shape_border_length, &corner_segment_length, &crop_segment_pos);
        let plane_shape_offset   = Var::<Distance<Pixels>>::from(&crop_plane_pos - width * 0.5);
        let crop_shape           = HalfPlane().rotate(plane_rotation_angle).translate_x(plane_shape_offset);
        crop_shape.into()
    }

    ensogl::define_shape_system! {
        (style:Style, grow:f32, shape_width:f32, offset_x:f32, padding:f32, opacity:f32) {
            let canvas_width : Var<Distance<Pixels>> = "input_size.x".into();
            let width        : Var<Distance<Pixels>> = shape_width.clone().into();
            let height       : Var<Distance<Pixels>> = "input_size.y".into();
            let width  = &width - node::NODE_SHAPE_PADDING.px() * 2.0;
            let height = height - node::NODE_SHAPE_PADDING.px() * 2.0;

            let hover_area_size   = 20.0.px();
            let hover_area_width  = &width  + &hover_area_size * 2.0;
            let hover_area_height = &height / 2.0 + &hover_area_size;
            let hover_area        = Rect((&hover_area_width,&hover_area_height));
            let hover_area        = hover_area.translate_y(-hover_area_height/2.0);

            let shrink           = 1.px() - 1.px() * &grow;
            let radius           = node::NODE_SHAPE_RADIUS.px();
            let port_area_size   = PORT_AREA_WIDTH.px() * &grow;
            let port_area_width  = width.clone()  + (&port_area_size - &shrink) * 2.0;
            let port_area_height = height.clone() + (&port_area_size - &shrink) * 2.0;
            let bottom_radius    = &radius + &port_area_size;
            let port_area        = Rect((&port_area_width,&port_area_height));
            let port_area        = port_area.corners_radius(&bottom_radius);
            let port_area        = port_area - BottomHalfPlane();
            let corner_radius    = &port_area_size / 2.0;
            let corner_offset    = &port_area_width / 2.0 - &corner_radius;
            let corner           = Circle(&corner_radius);
            let left_corner      = corner.translate_x(-&corner_offset);
            let right_corner     = corner.translate_x(&corner_offset);
            let port_area        = port_area + left_corner + right_corner;

            // Move the shape so it shows the correct slice, as indicated by `offset_x`.
            let offset_x          = Var::<Distance<Pixels>>::from(offset_x);
            let offset_x          = width/2.0 - offset_x;
            let port_area_aligned = port_area.translate_x(offset_x);

            // Crop the sides of the visible area to show a gap between segments.
            let crop_base_pos     = Var::<f32>::from("input_offset_x + input_size.x * 0.5") ;
            let padding           = Var::<Distance<Pixels>>::from(&padding * 1.0);
            let crop_window_base  = Rect((&padding,&(&height * 2.0)));

            let crop_window_center = crop_base_pos.clone();
            let crop_window_angle  = compute_angle(&width.into(), &radius.clone().into(), &crop_window_center.into());

            let crop_window_right   = crop_window_base.rotate(&crop_window_angle);
            let crop_window_right   = crop_window_right.translate_y(&height * -0.5);
            let crop_window_right   = crop_window_right.translate_x(-&canvas_width * 0.5);

            let crop_window_left   = crop_window_base.rotate(&crop_window_angle);
            let crop_window_left   = crop_window_left.translate_y(&height * -0.5);
            let crop_window_left   = crop_window_left.translate_x(&canvas_width * 0.5);

            let port_area_cropped = port_area_aligned.difference(crop_window_right);
            let port_area_cropped = port_area_cropped.difference(crop_window_left);

            // FIXME: Use colour from style and apply transparency there.
            let color             = Var::<color::Rgba>::from("srgba(0.25,0.58,0.91,input_opacity)");
            let port_area_colored = port_area_cropped.fill(color);

            (port_area + hover_area).into()
        }
    }

    impl PortShapeApi for Shape {
        fn set_grow(&self, grow_value: f32) {
            self.grow.set(grow_value)
        }

        fn set_opacity(&self, opacity: f32) {
            self.opacity.set(opacity)
        }
    }
}

/// Implements an simplified version of the multi_port_area::Shape shape for the case where there is
/// only a single output port.
pub mod single_port_area {
    use super::*;
    use ensogl::display::shape::*;

    ensogl::define_shape_system! {
        (style:Style, grow:f32, opacity:f32) {
            let overall_width  : Var<Distance<Pixels>> = "input_size.x".into();
            let overall_height : Var<Distance<Pixels>> = "input_size.y".into();
            let width  = &overall_width  - node::NODE_SHAPE_PADDING.px() * 2.0;
            let height = &overall_height - node::NODE_SHAPE_PADDING.px() * 2.0;

            let hover_area_size   = 20.0.px();
            let hover_area_width  = &width  + &hover_area_size * 2.0;
            let hover_area_height = &height / 2.0 + &hover_area_size;
            let hover_area        = Rect((&hover_area_width,&hover_area_height));
            let hover_area        = hover_area.translate_y(-hover_area_height/2.0);

            let shrink           = 1.px() - 1.px() * &grow;
            let radius           = 14.px();
            let port_area_size   = 4.0.px() * &grow;
            let port_area_width  = &width  + (&port_area_size - &shrink) * 2.0;
            let port_area_height = &height + (&port_area_size - &shrink) * 2.0;
            let bottom_radius    = &radius + &port_area_size;
            let port_area        = Rect((&port_area_width,&port_area_height));
            let port_area        = port_area.corners_radius(&bottom_radius);
            let port_area        = port_area - BottomHalfPlane();
            let corner_radius    = &port_area_size / 2.0;
            let corner_offset    = &port_area_width / 2.0 - &corner_radius;
            let corner           = Circle(&corner_radius);
            let left_corner      = corner.translate_x(-&corner_offset);
            let right_corner     = corner.translate_x(&corner_offset);
            let port_area        = port_area + left_corner + right_corner;
            let port_area        = port_area.fill(color::Rgba::from(color::Lcha::new(0.6,0.5,0.76,1.0)));

            // FIXME: Use colour from style and apply transparency there.
            let color     = Var::<color::Rgba>::from("srgba(0.25,0.58,0.91,input_opacity)");
            let port_area = port_area.fill(color);

            (port_area + hover_area).into()
        }
    }

    impl PortShapeApi for Shape {
        fn set_grow(&self, grow_value: f32) {
            self.grow.set(grow_value)
        }

        fn set_opacity(&self, opacity: f32) {
            self.opacity.set(opacity)
        }
    }
}

/// Helper enum that handles the distinction between a single shape output area and a multi port
/// output area.
#[derive(Clone,Debug)]
enum ShapeView {
    Single { view  : component::ShapeView<multi_port_area::Shape>      },
    Multi  { views : Vec<component::ShapeView<multi_port_area::Shape>> },
}

impl ShapeView {

    /// Constructor.
    fn new(number_of_ports:u32, logger:&Logger, scene:&Scene) -> Self {
        if number_of_ports == 1 {
            ShapeView::Single { view: component::ShapeView::new(&logger,&scene) }
        } else {
            let mut views = Vec::default();
            views.resize_with(number_of_ports as usize,|| component::ShapeView::new(&logger,&scene));
            ShapeView::Multi { views }
        }
    }

    /// Set up the frp for all ports.
    fn init_frp(&self, port_frp:PortFrp) {
        match self {
            ShapeView::Single {view}  => init_port_frp(&view, PortId{index:0},port_frp),
            ShapeView::Multi  {views} => {
                views.iter().enumerate().for_each(|(index,view)| {
                    init_port_frp(&view, PortId{index},port_frp.clone_ref())
                } )
            }
        }
    }

    /// Resize all the port output shapes to fit the new layout requirements for thr given
    /// parameters.
    fn update_shape_layout_based_on_size_and_gap(&self, size:Vector2<f32>, gap_width:f32) {
        match self {
            ShapeView::Single { view }   => {
                let shape = &view.shape;
                shape.sprite.size.set(size);
            }
            ShapeView::Multi { views }   => {
                let port_num  = views.len() as f32;
                // Align shapes along width.
                for (index, view) in views.iter().enumerate(){
                    let shape = &view.shape;
                    shape.sprite.size.set(size);
                    shape.index.set(index as f32);
                    shape.port_num.set(port_num);
                    shape.padding.set(gap_width);
                }
            }
        }
    }

    fn set_parent<T:display::object::Object>(&self, parent:&T) {
        match self {
            ShapeView::Single { view }   => {
                view.display_object().set_parent(parent);
            }
            ShapeView::Multi { views }   => {
                views.iter().for_each(|view|  view.display_object().set_parent(parent))
            }
        }
    }
}

// =============================
// === Port Frp Setup Helper ===
// =============================

/// Helper struct to pass the required FRP endpoints to set up the FRP of a port shape view.
#[derive(Clone,CloneRef,Debug)]
struct PortFrp {
    network                      : frp::Network,

    port_mouse_over              : frp::Source<PortId>,
    port_mouse_out               : frp::Source<PortId>,
    port_mouse_down              : frp::Source<PortId>,

    hide_all                     : frp::Stream<()>,
    activate_ports_with_selected : frp::Stream<PortId>
}

/// Set up the FRP system for a ShapeView of a shape that implements the PortShapeApi.
///
/// This allows us to use the same setup code for bot the `multi_port_area::Shape` and the
/// `single_port_area::Shape`.
fn init_port_frp<Shape:PortShapeApi+CloneRef+'static>
(view:&component::ShapeView<Shape>, port_id:PortId,frp:PortFrp) {
    let PortFrp { network, port_mouse_over, port_mouse_out, port_mouse_down,
        hide_all, activate_ports_with_selected } = frp;

    let shape        = &view.shape;
    let port_size    = Animation::<f32>::new(&network);
    let port_opacity = Animation::<f32>::new(&network);

    frp::extend! { network

            // === Mouse Event Handling == ///

            eval_ view.events.mouse_over(port_mouse_over.emit(port_id));
            eval_ view.events.mouse_out(port_mouse_out.emit(port_id));
            eval_ view.events.mouse_down(port_mouse_down.emit(port_id));


             // === Animation Handling == ///

             eval port_size.value    ((size) shape.set_grow(*size));
             eval port_opacity.value ((size) shape.set_opacity(*size));


            // === Visibility and Highlight Handling == ///

             def _hide_all = hide_all.map(f_!([port_size,port_opacity]{
                 port_size.set_target_value(0.0);
                 port_opacity.set_target_value(0.0);
             }));

            // Through the provided ID we can infer whether this port should be highlighted.
            is_selected      <- activate_ports_with_selected.map(move |id| *id == port_id);
            show_normal      <- activate_ports_with_selected.gate_not(&is_selected);
            show_highlighted <- activate_ports_with_selected.gate(&is_selected);

            eval_ show_highlighted ([port_opacity,port_size]{
                port_opacity.set_target_value(1.0);
                port_size.set_target_value(HIGHLIGHT_SIZE);
            });

            eval_ show_normal ([port_opacity,port_size]
                port_opacity.set_target_value(0.5);
                port_size.set_target_value(BASE_SIZE);
            );
        }
}

// ===========
// === Frp ===
// ===========

/// Id of a specific port inside of `OutPutPortsData`.
#[derive(Clone,Copy,Default,Debug,Eq,PartialEq)]
pub struct PortId {
    index: usize,
}

/// Frp API of the `OutPutPorts`.
#[derive(Clone,CloneRef,Debug)]
pub struct Frp {
    /// Update the size of the `OutPutPorts`. Should match the size of the parent node for visual correctness.
    pub set_size        : frp::Source<V2<f32>>,
    /// Emitted whenever one of the ports receives a `MouseDown` event. The `PortId` indicates the source port.
    pub port_mouse_down : frp::Stream<PortId>,

    on_port_mouse_down  : frp::Source<PortId>,
}

impl Frp {
    fn new(network: &frp::Network) -> Self {
        frp::extend! { network
            def set_size           = source();
            def on_port_mouse_down = source();

            let port_mouse_down = on_port_mouse_down.clone_ref().into();
        }
        Self{set_size,port_mouse_down,on_port_mouse_down}
    }
}



// =======================
// === OutPutPortsData ===
// =======================

/// Internal data of the `OutPutPorts`.
#[derive(Debug)]
pub struct OutputPortsData {
    display_object : display::object::Instance,
    logger         : Logger,
    size           : Cell<Vector2<f32>>,
    gap_width      : Cell<f32>,
    ports          : RefCell<ShapeView>,
}

impl OutputPortsData {

    fn new(scene:Scene, number_of_ports:u32) -> Self {
        let logger         = Logger::new("OutPutPorts");
        let display_object = display::object::Instance::new(&logger);
        let size           = Cell::new(Vector2::zero());
        let gap_width      = Cell::new(SEGMENT_GAP_WIDTH);
        let ports          = ShapeView::new(number_of_ports, &logger, &scene);
        let ports          = RefCell::new(ports);

        OutputPortsData {display_object,logger,size,ports,gap_width}.init()
    }

    fn init(self) -> Self {
        self.ports.borrow().set_parent(&self.display_object);
        self.update_shape_layout_based_on_size_and_gap();
        self
    }

    fn update_shape_layout_based_on_size(&self) {
        let port_num    = self.ports.borrow().len() as f32;
        let width       = self.size.get().x;
        let width_inner = width - 2.0 * node::NODE_SHAPE_PADDING ;
        let height      = self.size.get().y;
        let port_width  = (width_inner) / port_num;
        let port_size   = Vector2::new(port_width, height);
        let gap_width   = self.gap_width.get();
        // Align shapes along width.
        let x_start = -width / 2.0 + node::NODE_SHAPE_PADDING + 0.5 * port_width;
        let x_delta = port_width;
        for (index, view) in self.ports.borrow().iter().enumerate(){
            view.display_object().set_parent(&self.display_object);

            let pos_x = x_start + x_delta * index as f32;
            let pos_y = 0.0;
            let pos   = Vector2::new(pos_x,pos_y);
            view.set_position_xy(pos);

            let shape = &view.shape;
            shape.sprite.size.set(port_size);
            shape.shape_width.set(width);
            shape.padding.set(gap_width);
            shape.offset_x.set(x_delta * index as f32);
        }
    }

    fn set_size(&self, size:Vector2<f32>) {
        self.size.set(size);
        self.update_shape_layout_based_on_size_and_gap();
    }
}



// ===================
// === OutPutPorts ===
// ===================

/// Implements the segmented output port area. Provides shapes that can be attached to a `Node` to
/// add an interactive area with output ports.
///
/// The `OutputPorts` facilitate the falling behaviour:
///  * when one of the output ports is hovered, after a set time, all ports are show and the hovered
///    port is highlighted.
///  * when a different port is hovered, it is highlighted immediately.
///  * when none of the ports is hovered all of the `OutputPorts` disappear. Note: there is a very
///    small delay for disappearing to allow for smooth switching between ports.
///
#[derive(Debug,Clone,CloneRef)]
pub struct OutputPorts {
    /// The FRP api of the `OutPutPorts`.
    pub frp     : Frp,
        network : frp::Network,
        data    : Rc<OutputPortsData>,
}

impl OutputPorts {
    /// Constructor.
    pub fn new(scene:&Scene, number_of_ports:u32) -> Self {
        let network = default();
        let frp     = Frp::new(&network);
        let data    = OutputPortsData::new(scene.clone_ref(), number_of_ports);
        let data    = Rc::new(data);
        OutputPorts {data,network,frp}.init()
    }

    fn init(mut self) -> Self {
        self.init_frp();
        self
    }

    fn init_frp(&mut self) {
        let network = &self.network;
        let frp     = &self.frp;
        let data    = &self.data;

        // Used to set and detect the end of the tweens. The actual value is irrelevant, only the
        // duration of the tween matters and that this value is reached after that time.
        const TWEEN_END_VALUE:f32 = 1.0;

        // Timer used to measure whether the hover has been long enough to show the ports.
        let delay_show = Tween::new(&network);
        delay_show.set_duration(SHOW_DELAY_DURATION);

        // Timer used to measure whether the mouse has been gone long enough to hide all ports.
        let delay_hide = Tween::new(&network);
        delay_hide.set_duration(HIDE_DELAY_DURATION);

        frp::extend! { network

            // === Size Change Handling == ///

            eval frp.set_size ((size) data.set_size(size.into()));


            // === Hover Event Handling == ///

            port_mouse_over      <- source::<PortId>();
            port_mouse_out       <- source::<PortId>();

            delay_show_finished    <- delay_show.value.map(|t| *t>=TWEEN_END_VALUE );
            delay_hide_finished    <- delay_hide.value.map(|t| *t>=TWEEN_END_VALUE );
            on_delay_show_finished <- delay_show_finished.gate(&delay_show_finished).constant(());
            on_delay_hide_finished <- delay_hide_finished.gate(&delay_hide_finished).constant(());

            visible                <- delay_show_finished.map(|v| *v);

            mouse_over_while_inactive  <- port_mouse_over.gate_not(&visible).constant(());
            mouse_over_while_active    <- port_mouse_over.gate(&visible).constant(());

            eval mouse_over_while_inactive ([delay_show,delay_hide](_){
                delay_hide.stop();
                delay_show.rewind();
                delay_show.set_end_value(TWEEN_END_VALUE);
            });
            eval port_mouse_out ([delay_hide,delay_show](_){
                delay_show.stop();
                delay_hide.rewind();
                delay_hide.set_end_value(TWEEN_END_VALUE);
            });

            activate_ports <- any(mouse_over_while_active,on_delay_show_finished);
            eval_ activate_ports (delay_hide.rewind());

            activate_ports_with_selected <- port_mouse_over.sample(&activate_ports);

            hide_all <- on_delay_hide_finished.map(f_!(delay_show.rewind()));

        }

        let port_frp = PortFrp {network         : network.clone_ref(),
                                port_mouse_down : frp.on_port_mouse_down.clone_ref(),
                                port_mouse_over,port_mouse_out,hide_all,
                                activate_ports_with_selected};

        data.ports.borrow().init_frp(port_frp);

        // FIXME this is a hack to ensure the ports are invisible at startup.
        // Right now we get some of FRP mouse events on startup that leave the
        // ports visible by default.
        // Once that is fixed, remove this line.
        delay_hide.finish();
    }

    // TODO: Implement proper sorting and remove.
    /// Hack function used to register the elements for the sorting purposes. To be removed.
    pub(crate) fn order_hack(scene:&Scene) {
        let logger = Logger::new("hack");
        component::ShapeView::<multi_port_area::Shape>::new(&logger, scene);
    }
}

impl display::Object for OutputPorts {
    fn display_object(&self) -> &display::object::Instance {
        &self.data.display_object
    }
}
