//! A small path drawing application.


use druid::kurbo::{BezPath, Circle, Line, Point, Rect, Size, Vec2};
use druid::piet::{Color, FillRule, RenderContext};

use druid::shell::{runloop, WindowBuilder};
use druid::widget::{MouseButton, MouseEvent, Widget};
use druid::{
    BoxConstraints, HandlerCtx, Id, KeyEvent, KeyCode, LayoutCtx, LayoutResult, PaintCtx, Ui, UiMain, UiState,
};

const BG_COLOR: Color = Color::rgb24(0xfb_fb_fb);
const PATH_COLOR: Color = Color::rgb24(0xbb_bb_bb);
const ACTIVE_PATH_COLOR: Color = Color::rgb24(0x02_7b_db);
const ON_CURVE_POINT_COLOR: Color = Color::rgb24(0x0b_2b_db);

const ON_CURVE_POINT_RADIUS: f64 = 2.5;
const OPEN_PATH_END_LINE_LENGTH: f64 = 8.0;
const MIN_POINT_DISTANCE: f64 = 3.0;

struct Canvas {
    paths: Vec<Path>,
    mouse: Point,
    state: State,
}

impl Canvas {
    fn new() -> Canvas {
        Canvas {
            paths: Vec::new(),
            mouse: Point::ZERO,
            state: State::Ready,
        }
    }

    fn check_state(&mut self) {
        if let Some(path) = self.state.done_path() {
            self.paths.push(path);
        }
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Path {
    points: Vec<Point>,
    closed: bool,
}

impl Path {
    fn start(pos: Point) -> Path {
        Path {
            points: vec![pos],
            closed: false,
        }
    }

    fn push(&mut self, point: Point) {
        self.points.push(point)
    }

    fn close(&mut self) {
        self.closed = true;
    }
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Ready,
    Drawing(Path),
    Done(Path),
}

impl State {
    fn is_drawing(&self) -> bool {
        match self {
            State::Drawing(_) => true,
            _ => false
        }
    }

    /// If state is `Done`, return the path and reset state
    fn done_path(&mut self) -> Option<Path> {
        let path = match self {
            State::Done(ref mut path) => Some(std::mem::replace(path, Path::default())),
            _ => None,
        };
        if path.is_some() {
            *self = State::Ready;
        }
        path
    }

    fn update_for_event(&mut self, event: &MouseEvent) -> bool {
        // this involves some unfortunate borrowck gymnastics
        let mut old = std::mem::replace(self, State::Ready);
        let next = match (&mut old, &event.button, &event.count) {
            (State::Ready, &MouseButton::Left, &1) => {
                Some(State::Drawing(Path::start(event.pos)))
            }
            (State::Drawing(ref mut path), MouseButton::Left, 1) => {
                let mut path = std::mem::replace(path, Path::default());
                let closes = path.points.iter().any(|p| p.distance(event.pos) < MIN_POINT_DISTANCE);
                path.push(event.pos);
                let next = if closes {
                    path.close();
                    State::Done(path)
                } else {
                    State::Drawing(path)
                };
                Some(next)
            }
            (State::Drawing(ref mut path), MouseButton::Left, 2) => {
                let mut path = std::mem::replace(path, Path::default());
                let num_points = path.points.len();
                let closes = path.points[..num_points-1].iter().any(|p| p.distance(event.pos) < MIN_POINT_DISTANCE);
                if closes {
                    path.close();
                }
                Some(State::Done(path))
            }
            _ => None,
        };

        if let Some(next) = next {
            *self = next;
            true
        } else {
            *self = old;
            false
        }
    }
}


impl Widget for Canvas {

    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        paint_ctx.render_ctx.clear(BG_COLOR);
        for path in &self.paths {
            draw_inactive_path(path, paint_ctx, geom);
        }
        match self.state {
            State::Ready => (),
            State::Drawing(ref path) => draw_active_path(path, self.mouse, paint_ctx, geom),
            State::Done(ref path) => draw_inactive_path(path, paint_ctx, geom),
        }
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.max())
    }

    fn mouse_moved(&mut self, pos: Point, ctx: &mut HandlerCtx) {
        self.mouse = pos;
        if self.state.is_drawing() {
            ctx.invalidate()
        }
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        let handled = self.state.update_for_event(event);
        self.check_state();
        if handled {
            ctx.invalidate();
        }
        handled
    }

    fn key_down(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        match event {
            event if event.key_code == KeyCode::Backspace => self.state = State::Ready,
            _ => return false,
        }

        ctx.invalidate();
        true
    }
}

fn draw_inactive_path(path: &Path, paint_ctx: &mut PaintCtx, _geom: &Rect) {
    if path.points.len() < 2 {
        return;
    }
    let mut bez = BezPath::new();
    bez.move_to(path.points[0]);
    for point in path.points.iter().skip(1) {
        bez.line_to(*point);
    }

    if path.closed {
        bez.close_path();
    }

    let path_brush = paint_ctx.render_ctx.solid_brush(PATH_COLOR);
    paint_ctx.render_ctx.stroke(bez, &path_brush, 1.0, None);
}

fn draw_active_path(path: &Path, mouse: Point, paint_ctx: &mut PaintCtx, _geom: &Rect) {
    if path.points.is_empty() {
        return;
    }

    let on_curve_point_brush = paint_ctx.render_ctx.solid_brush(ON_CURVE_POINT_COLOR);
    let path_brush = paint_ctx.render_ctx.solid_brush(PATH_COLOR);
    let active_path_brush = paint_ctx.render_ctx.solid_brush(ACTIVE_PATH_COLOR);
    let white_brush = paint_ctx.render_ctx.solid_brush(Color::WHITE);

    let node_count = path.points.len();

    let circ = Circle::new(path.points[0], ON_CURVE_POINT_RADIUS);
    paint_ctx.render_ctx.fill(circ, &on_curve_point_brush, FillRule::NonZero);

    // draw the path itself
    let mut bez = BezPath::new();
    bez.move_to(path.points[0]);
    for point in path.points.iter().skip(1) {
        bez.line_to(*point);
    }

    if path.closed {
        bez.close_path();
    }

    paint_ctx.render_ctx.stroke(bez, &path_brush, 1.0, None);

    // draw the 'active path', that is the path that would be added on the next click.

    let active_line = Line::new(path.points[node_count - 1], mouse);
    paint_ctx.render_ctx.stroke(active_line, &active_path_brush, 1.0, None);

    // draw the control points

    // if open path, draw start and end points as perpendicular lines
    //if !path.closed {
        //let open_node = perp(path.points[0], path.points[1], OPEN_PATH_END_LINE_LENGTH);
        //let close_node = perp(path.points[node_count-1], path.points[node_count-2], OPEN_PATH_END_LINE_LENGTH);
        //paint_ctx.render_ctx.stroke(open_node, &on_curve_point_brush, 1.0, None);
        //paint_ctx.render_ctx.stroke(close_node, &on_curve_point_brush, 1.0, None);
    //}

    //let points_to_draw = if path.closed {
        //&path.points
    //} else {
        //&path.points[1..node_count - 1]
    //};

    for point in &path.points {
        let circ = Circle::new(*point, ON_CURVE_POINT_RADIUS);
        paint_ctx.render_ctx.fill(circ, &on_curve_point_brush, FillRule::NonZero);
    }


    let active_circ = Circle::new(mouse, ON_CURVE_POINT_RADIUS);
    paint_ctx.render_ctx.fill(active_circ, &white_brush, FillRule::NonZero);
    paint_ctx.render_ctx.stroke(active_circ, &on_curve_point_brush, 1.0, None);

    if let Some(p) = path.points.iter().find(|p| p.distance(mouse) < 2. * ON_CURVE_POINT_RADIUS) {
    let close_circ = Circle::new(*p, ON_CURVE_POINT_RADIUS);
    paint_ctx.render_ctx.fill(close_circ, &white_brush, FillRule::NonZero);
    paint_ctx.render_ctx.stroke(close_circ, &on_curve_point_brush, 1.0, None);
    }
}

fn perp(p1: Point, p2: Point, len: f64) -> Line {
    let perp_vec = Vec2::new(p1.y - p2.y, p2.x - p1.x);
    let norm_perp = perp_vec / perp_vec.hypot();
    let p3 = p1 + (len * -0.5) * norm_perp;
    let p4 = p1 + (len * 0.5) * norm_perp;
    Line::new(p3, p4)
}

// what is our state for a path?
// - open or closed
// - points: on curve, off curve
// - on curve points: corner, tangent,

fn main() {
    druid_shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let foo = Canvas::new().ui(&mut state);
    state.set_root(foo);
    state.set_focus(Some(foo));
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Paint");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
