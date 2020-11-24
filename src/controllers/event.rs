use druid::{widget::*, *};

pub struct EventController<T> {
    event_handler: fn(&mut EventCtx, &Event, &mut T, &Env),
}

impl<T> EventController<T> {
    pub fn new(event_handler: fn(&mut EventCtx, &Event, &mut T, &Env)) -> Self {
        Self { event_handler }
    }
}

impl<T, W: Widget<T>> Controller<T, W> for EventController<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        child.event(ctx, event, data, env);

        (self.event_handler)(ctx, event, data, env);
    }
}
