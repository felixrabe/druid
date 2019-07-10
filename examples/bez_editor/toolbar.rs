use druid::kurbo::{Affine, BezPath, Line, Point, Rect, Shape, Size};
use druid::piet::{Color, FillRule, RenderContext};
use druid::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx,
    Widget,
};

pub struct Toolbar {
    items: Vec<ToolbarItem>,
    selected: usize,
    hot: Option<usize>,
}

struct ToolbarItem {
    name: String,
    hotkey: String,
    icon: BezPath,
}

impl ToolbarItem {
    fn new(name: impl Into<String>, hotkey: impl Into<String>, icon: BezPath) -> Self {
        let padding = TOOLBAR_ICON_PADDING * 2.;
        let icon = scale_path(
            &icon,
            (TOOLBAR_ITEM_WIDTH - padding, TOOLBAR_HEIGHT - padding),
        );
        ToolbarItem {
            name: name.into(),
            hotkey: hotkey.into(),
            icon,
        }
    }

    fn select() -> Self {
        let mut path = BezPath::new();
        path.move_to((45., 100.));
        path.line_to((55., 100.));
        path.line_to((55., 70.));
        path.line_to((80., 70.));
        path.line_to((50., 10.));
        path.line_to((20., 70.));
        path.line_to((45., 70.));
        path.close_path();
        path.apply_affine(Affine::rotate(-0.5));
        ToolbarItem::new("select", "v", path)
    }

    fn pen() -> Self {
        let mut path = BezPath::new();
        path.move_to((173., 0.));
        path.line_to((277., 0.));
        path.line_to((277., 93.));
        path.curve_to((277., 93.), (364., 186.), (364., 265.));
        path.curve_to((364., 344.), (255., 481.), (255., 481.));
        path.curve_to((255., 481.), (86., 344.), (86., 265.));
        path.curve_to((86., 186.), (173., 93.), (173., 93.));
        path.close_path();
        path.apply_affine(Affine::rotate(-3.5));
        ToolbarItem::new("pen", "p", path)
    }
}

impl Toolbar {
    pub fn basic() -> Self {
        Toolbar::new(vec![ToolbarItem::select(), ToolbarItem::pen()])
    }

    fn new(items: Vec<ToolbarItem>) -> Self {
        Toolbar { items, selected: 0, hot: None }
    }

    fn size(&self) -> Size {
        let width = self.items.len() as f64 * TOOLBAR_ITEM_WIDTH; // + (self.items.len().saturating_sub(1) as f64 * TOOLBAR_ITEM_PADDING);
        Size::new(width, TOOLBAR_HEIGHT)
    }

    fn tool_at_pos(&self, pos: Point) -> Option<usize> {
        let Size { width, height } = self.size();
        if pos.x > 0. && pos.y > 0. && pos.x < width && pos.y < height {
            let idx = (pos.x / TOOLBAR_ITEM_WIDTH).trunc() as usize;
            Some(idx)
        } else {
            None
        }
    }
}

const TOOLBAR_HEIGHT: f64 = 32.;
const TOOLBAR_ICON_PADDING: f64 = 4.;
const TOOLBAR_ITEM_WIDTH: f64 = 32.;
const TOOLBAR_COLOR: Color = Color::rgb24(0xaa_aa_aa);
const SELECTED_ITEM_COLOR: Color = Color::rgb24(0x1e_40_d8);
const HOVER_ITEM_COLOR: Color = Color::rgb24(0x9e_40_d8);
const NORMAL_ITEM_COLOR: Color = Color::rgb24(0x3e_3a_38);

impl<T: Data> Widget<T> for Toolbar {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, _data: &T, _env: &Env) {
        let rect = Rect::from_origin_size((0., 0.), self.size());
        let bg_brush = paint_ctx.render_ctx.solid_brush(TOOLBAR_COLOR);
        let item_brush = paint_ctx.render_ctx.solid_brush(NORMAL_ITEM_COLOR);
        let hot_brush = paint_ctx.render_ctx.solid_brush(HOVER_ITEM_COLOR);
        let selected_brush = paint_ctx.render_ctx.solid_brush(SELECTED_ITEM_COLOR);
        paint_ctx
            .render_ctx
            .fill(rect, &bg_brush, FillRule::NonZero);

        let mut last = None;
        for (i, tool) in self.items.iter().enumerate() {
            let tool_size = tool.icon.bounding_box().size();
            let x_pad = TOOLBAR_ICON_PADDING.max((TOOLBAR_ITEM_WIDTH - tool_size.width) * 0.5);
            let y_pad = TOOLBAR_ICON_PADDING.max((TOOLBAR_HEIGHT - tool_size.height) * 0.5);
            let tool_pos = Affine::translate((x_pad + i as f64 * TOOLBAR_ITEM_WIDTH, y_pad));
            let brush = if i == self.selected {
                &selected_brush
            } else if Some(i) == self.hot {
                &hot_brush
            } else {
                &item_brush
            };
            paint_ctx
                .render_ctx
                .fill(tool_pos * &tool.icon, &brush, FillRule::NonZero);
            if let Some(last) = last {
                let line = Line::new((last, 0.), (last, TOOLBAR_HEIGHT));
                paint_ctx.render_ctx.stroke(line, &item_brush, 0.5, None);
            }
            last = Some((i + 1) as f64 * TOOLBAR_ITEM_WIDTH);
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.constrain(self.size())
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        _data: &mut T,
        _env: &Env,
    ) -> Option<Action> {
        match event {
            Event::KeyUp(key) => {
                let text = key.unmod_text().unwrap_or("");
                if let Some((i, item)) = self
                    .items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| item.hotkey == text)
                {
                    self.selected = i;
                    return Some(Action::from_str(item.name.as_str()));
                }
            }
            Event::MouseDown(mouse) => {
                if let Some(idx) = self.tool_at_pos(mouse.pos) {
                    self.hot = Some(idx);
                    ctx.set_handled();
                    ctx.set_active(true);
                    ctx.invalidate();
                }
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() {
                    self.hot = None;
                    ctx.set_active(false);
                    ctx.set_handled();
                    ctx.invalidate();
                    if let Some(idx) = self.tool_at_pos(mouse.pos) {
                        self.selected = idx;
                        return Some(Action::from_str(self.items[idx].name.as_str()));
                    }
                }
            }

            Event::MouseMoved(mouse) => {
                let hot = self.tool_at_pos(mouse.pos);
                if hot != self.hot {
                    self.hot = hot;
                    ctx.invalidate();
                    ctx.set_handled();
                }
            }
            _ => (),
        }
        None
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}

fn scale_path(path: &BezPath, fitting_size: impl Into<Size>) -> BezPath {
    let mut out = path.clone();
    let fitting_size = fitting_size.into();
    let path_size = path.bounding_box().size();
    let scale_factor =
        (fitting_size.width / path_size.width).min(fitting_size.height / path_size.height);
    out.apply_affine(Affine::scale(scale_factor));
    let translation = Point::ZERO - out.bounding_box().origin();
    out.apply_affine(Affine::translate(translation));
    out
}
