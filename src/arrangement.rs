use crate::{commands, settings, theme, AppState};
use druid::{widget::*, *};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
pub struct Arrangement {
    pub tracks: Arc<Vec<Track>>,
    pub beats: usize,
}

impl Arrangement {
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(vec![Track::new()]),
            beats: 4,
        }
    }

    pub fn add_track(&mut self) {
        Arc::make_mut(&mut self.tracks).push(Track::empty())
    }

    pub fn remove_track(&mut self, index: usize) {
        Arc::make_mut(&mut self.tracks).remove(index);
    }
}

#[derive(Clone, Data)]
pub struct Track {
    blocks: Arc<Vec<(usize, Block)>>,
}

impl Track {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(Vec::new()),
        }
    }

    pub fn empty() -> Self {
        Self {
            blocks: Arc::new(Vec::new()),
        }
    }
}

#[derive(Data, Clone)]
pub struct Block {
    length: usize,
    id: crate::audio::AudioID,
}

impl Block {
    fn new(length: usize, id: crate::audio::AudioID) -> Self {
        Self { length, id }
    }
}

pub struct ArrangementWidget {
    children: Vec<WidgetPod<Track, TrackWidget>>,
    scroll: Vec2,
    play_line: f64,
}

impl ArrangementWidget {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            scroll: Vec2::new(0.0, 0.0),
            play_line: 0.0,
        }
    }

    pub fn update_children(&mut self, arrangement: &Arrangement) {
        let mut new_children = Vec::new();

        for (i, _track) in arrangement.tracks.iter().enumerate() {
            new_children.push(WidgetPod::new(TrackWidget::new(50.0, i)));
        }

        self.children = new_children;
    }
}

impl Widget<Arrangement> for ArrangementWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Arrangement, env: &Env) {
        let mut children = self.children.iter_mut();

        data.tracks.for_each_mut(|child_data, _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });

        match event {
            Event::Wheel(mouse_event) => {
                let scroll_speed = env.get(crate::settings::ARRANGEMENT_SCROLL_SPEED);

                if mouse_event.mods.ctrl {
                    self.scroll.y += mouse_event.wheel_delta.y * scroll_speed;

                    self.scroll.y = self.scroll.y.max(0.0);
                } else {
                    self.scroll.x += mouse_event.wheel_delta.y * scroll_speed;

                    self.scroll.x = self.scroll.x.max(0.0);
                }

                ctx.request_layout();
            }

            Event::MouseDown(mouse_event) if mouse_event.button.is_middle() => {
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let time = (mouse_event.pos.x + self.scroll.x) / beat_size;

                self.play_line = time;
                ctx.submit_command(
                    Command::new(commands::AUDIO_ENGINE_SET_PLAY_TIME, time),
                    Target::Global,
                );
                ctx.request_paint();
            }

            Event::Command(cmd) if cmd.is(commands::ARRANGEMENT_UPDATE_PLAY_LINE) => {
                let place = cmd.get_unchecked(commands::ARRANGEMENT_UPDATE_PLAY_LINE);

                self.play_line = *place;

                ctx.request_paint();
            }

            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Arrangement,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.update_children(data);
            ctx.children_changed();
        }

        for i in 0..self.children.len() {
            self.children[i].lifecycle(ctx, event, &data.tracks[i], env);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &Arrangement,
        data: &Arrangement,
        env: &Env,
    ) {
        let mut children = self.children.iter_mut();
        data.tracks.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.update(ctx, child_data, env);
            }
        });

        if !old_data.same(data) {
            self.update_children(data);
            ctx.children_changed();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Arrangement,
        env: &Env,
    ) -> Size {
        let mut size = Size::new(bc.max().width, 0.0);

        for i in 0..self.children.len() {
            let track = &data.tracks[i];

            let mut max = bc.max();
            max.width += self.scroll.x;
            let child_size =
                self.children[i].layout(ctx, &BoxConstraints::new(bc.min(), max), track, env);

            let rect = Rect::from_origin_size(
                (0.0 - self.scroll.x, size.height - self.scroll.y),
                child_size,
            );
            self.children[i].set_layout_rect(ctx, track, env, rect);

            size.height += child_size.height;
        }

        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Arrangement, env: &Env) {
        let viewport = ctx.size().to_rect().to_rounded_rect(5.0);
        ctx.with_save(|ctx| {
            ctx.clip(viewport);

            ctx.with_save(|ctx| {
                ctx.transform(Affine::translate(Vec2::new(-self.scroll.x, 0.0)));

                let mut beat = 0.0;
                let mut beat_num = 0;
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let beat_line_width = env.get(theme::ARRANGEMENT_BEAT_LINE_WIDTH);

                while beat <= ctx.size().width + self.scroll.x {
                    let rect = Rect::from_origin_size(
                        (beat - beat_line_width / 2.0, 0.0),
                        (beat_line_width, ctx.size().height),
                    );

                    let color = if beat_num % data.beats == 0 {
                        env.get(theme::ARRANGEMENT_TACT_LINE_COLOR)
                    } else {
                        env.get(theme::ARRANGEMENT_BEAT_LINE_COLOR)
                    };

                    ctx.fill(rect, &color);

                    beat += beat_size;
                    beat_num += 1;
                }
            });

            for i in 0..self.children.len() {
                self.children[i].paint(ctx, &data.tracks[i], env);
            }

            ctx.with_save(|ctx| {
                ctx.transform(Affine::translate(Vec2::new(-self.scroll.x, 0.0)));

                let width = env.get(theme::ARRANGEMENT_PLAY_LINE_WIDTH);
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let rect = Rect::from_origin_size(
                    (self.play_line * beat_size - width / 2.0, 0.0),
                    (width, ctx.size().height),
                );

                ctx.fill(rect, &env.get(theme::ARRANGEMENT_PLAY_LINE_COLOR));
            });
        });
    }
}

pub struct TrackWidget {
    children: Vec<WidgetPod<Block, Box<dyn Widget<Block> + 'static>>>,
    height: f64,
    index: usize,
}

impl TrackWidget {
    pub fn new(height: f64, index: usize) -> Self {
        Self {
            children: Vec::new(),
            height,
            index,
        }
    }

    pub fn update_children(&mut self, data: &Track) {
        let mut new_children = Vec::new();

        for (_space, _block) in &*data.blocks {
            let widget = Painter::new(|ctx, _block: &Block, _env| {
                let size = ctx.size();

                let rect = Rect::from_origin_size((0.0, 0.0), size).to_rounded_rect(10.0);

                ctx.fill(rect, &Color::BLACK);
            });

            new_children.push(WidgetPod::new(Box::new(widget) as Box<dyn Widget<_>>));
        }

        self.children = new_children;
    }
}

impl Widget<Track> for TrackWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Track, env: &Env) {
        let mut children = self.children.iter_mut();

        data.blocks.for_each_mut(|(_, child_data), _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });

        match event {
            Event::MouseDown(mouse_event) if mouse_event.button.is_right() => {
                let menu = ContextMenu::new(
                    MenuDesc::<AppState>::empty().append(MenuItem::new(
                        LocalizedString::new("Remove"),
                        Command::new(commands::ARRANGEMENT_REMOVE_TRACK, self.index),
                    )),
                    mouse_event.window_pos,
                );
                ctx.show_context_menu(menu);
            }

            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Track, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.update_children(data);
            ctx.children_changed();
        }

        for i in 0..self.children.len() {
            self.children[i].lifecycle(ctx, event, &data.blocks[i].1, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Track, data: &Track, env: &Env) {
        let mut children = self.children.iter_mut();
        data.blocks.for_each(|(_, child_data), _| {
            if let Some(child) = children.next() {
                child.update(ctx, child_data, env);
            }
        });

        if !old_data.same(data) {
            self.update_children(data);
            ctx.children_changed();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Track,
        env: &Env,
    ) -> Size {
        let mut size = Size::new(0.0, self.height);

        for i in 0..self.children.len() {
            let (space, block) = &data.blocks[i];
            let s = Size::new(block.length as f64 * 10.0, size.height);

            let bc = BoxConstraints::new(s, s);
            let block_size = self.children[i].layout(ctx, &bc, block, env);

            size.width += *space as f64 * 10.0;

            let rect = Rect::from_origin_size((size.width, 0.0), block_size);

            size.width += block_size.width;
            self.children[i].set_layout_rect(ctx, block, env, rect);
        }

        size.width = bc.max().width;

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Track, env: &Env) {
        for i in 0..self.children.len() {
            self.children[i].paint(ctx, &data.blocks[i].1, env);
        }

        let rect = Rect::from_origin_size(
            (0.0, ctx.size().height),
            (ctx.size().width, env.get(theme::BORDER_WIDTH)),
        );
        ctx.fill(rect, &env.get(theme::BORDER_COLOR));
    }
}
