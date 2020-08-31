use druid::*;

pub struct MaxBox<T, W: Widget<T>> {
    child: WidgetPod<T, W>,
    max_width: Option<f64>,
    max_height: Option<f64>,
}

impl<T, W: Widget<T>> MaxBox<T, W> {
    pub fn new(widget: W) -> Self {
        Self {
            child: WidgetPod::new(widget),
            max_width: None,
            max_height: None,
        }
    }

    pub fn width(mut self, width: f64) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn height(mut self, height: f64) -> Self {
        self.max_height = Some(height);
        self
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for MaxBox<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env);

        if let LifeCycle::WidgetAdded = event {
            ctx.children_changed();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let mut max = bc.max();

        if let Some(max_width) = self.max_width {
            max.width = max.width.min(max_width);
        }

        if let Some(max_height) = self.max_height {
            max.height = max.height.min(max_height);
        }

        let bc = BoxConstraints::new(bc.min(), max);

        let size = self.child.layout(ctx, &bc, data, env);
        self.child
            .set_layout_rect(ctx, data, env, Rect::from_origin_size((0.0, 0.0), size));

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }
}
