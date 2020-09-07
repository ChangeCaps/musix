use crate::{commands, settings, theme, AppState, AudioBlock, AudioBlockID};
use druid::{widget::*, *};
use std::{collections::HashMap, sync::Arc};

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
            blocks: Arc::new(vec![
                (0, Block::new(4, crate::audio::AudioID(0))),
                (0, Block::new(6, crate::audio::AudioID(0))),
            ]),
        }
    }

    pub fn empty() -> Self {
        Self {
            blocks: Arc::new(Vec::new()),
        }
    }

    pub fn get_block(&self, index: usize) -> Option<&Block> {
        let mut place = 0;

        for (space, block) in self.blocks.iter() {
            place += space;

            if index < place {
                return None;
            }

            place += block.length;

            if index < place {
                return Some(block);
            }
        }

        None
    }
}

#[derive(Data, Clone, PartialEq)]
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
    children: Vec<WidgetPod<AppState, TrackWidget>>,
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
            new_children.push(WidgetPod::new(TrackWidget::new(i)));
        }

        self.children = new_children;
    }
}

impl Widget<AppState> for ArrangementWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        for child in &mut self.children {
            child.event(ctx, event, data, env);
        }

        match event {
            Event::Wheel(mouse_event) => {
                let scroll_speed = env.get(crate::settings::ARRANGEMENT_SCROLL_SPEED);

                if mouse_event.mods.ctrl {
                    self.scroll.y += mouse_event.wheel_delta.y * scroll_speed;

                    self.scroll.y = self
                        .scroll
                        .y
                        .max(-env.get(settings::ARRANGEMENT_TRACK_HEIGHT) / 2.0);
                } else {
                    self.scroll.x += mouse_event.wheel_delta.y * scroll_speed;

                    self.scroll.x = self.scroll.x.max(-env.get(settings::ARRANGEMENT_BEAT_SIZE));
                }

                ctx.request_layout();
            }

            Event::MouseDown(mouse_event) if mouse_event.button.is_middle() => {
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let mut time = (mouse_event.pos.x + self.scroll.x) / beat_size;
                time = time.max(0.0);

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

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.update_children(&data.arrangement);
            self.scroll.x = -env.get(settings::ARRANGEMENT_BEAT_SIZE);
            self.scroll.y = -env.get(settings::ARRANGEMENT_TRACK_HEIGHT) / 2.0;
            ctx.children_changed();
        }

        for child in &mut self.children {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        for child in &mut self.children {
            child.update(ctx, data, env);
        }

        if !old_data.arrangement.same(&data.arrangement) {
            self.update_children(&data.arrangement);
            ctx.children_changed();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        let mut size = Size::new(bc.max().width, 0.0);

        for child in &mut self.children {
            let mut max = bc.max();
            max.width += self.scroll.x;
            let child_size = child.layout(ctx, &BoxConstraints::new(bc.min(), max), data, env);

            let rect = Rect::from_origin_size(
                (0.0 - self.scroll.x, size.height - self.scroll.y),
                child_size,
            );
            child.set_layout_rect(ctx, data, env, rect);

            size.height += child_size.height;
        }

        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        let arrangement = &data.arrangement;

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

                    let color = if beat_num % arrangement.beats == 0 {
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
                self.children[i].paint(ctx, data, env);
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
    index: usize,
}

impl TrackWidget {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

impl Widget<AppState> for TrackWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut AppState, _env: &Env) {
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

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppState,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &AppState, _data: &AppState, _env: &Env) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        env: &Env,
    ) -> Size {
        Size::new(bc.max().width, env.get(settings::ARRANGEMENT_TRACK_HEIGHT))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        let mut place = 0.0;
        let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);

        let track = &data.arrangement.tracks[self.index];

        while place < ctx.size().width {
            let block = track.get_block((place / beat_size).floor() as usize); //.map(|i| &data.audio_blocks[i.id]);
            let prev_block = track.get_block((place / beat_size).floor() as usize - 1);

            let color = block
                .map(|b| Color::rgb(0.7, 0.2, 0.2))
                .unwrap_or(Color::WHITE);
            let prev_color = prev_block
                .map(|b| Color::rgb(0.7, 0.2, 0.2))
                .unwrap_or(Color::WHITE);

            let offset = if prev_block.is_some() || block.is_none() {
                0.0
            } else {
                6.0
            };

            let rect = Rect::from_origin_size(
                (place + offset, ctx.size().height / 2.0 - 2.0 / 2.0),
                (beat_size - offset, 2.0),
            );
            ctx.fill(rect, &color);

            if let Some(_prev_block) = prev_block {
                if block != prev_block {
                    let circle = kurbo::Circle::new((place, ctx.size().height / 2.0), 4.0);
                    ctx.fill(circle, &prev_color);
                }
            }

            if let Some(_block) = block {
                if block != prev_block {
                    let circle = kurbo::Circle::new((place, ctx.size().height / 2.0), 6.0);
                    ctx.stroke(circle, &color, 1.0);
                }
            }

            place += beat_size;
        }
    }
}
