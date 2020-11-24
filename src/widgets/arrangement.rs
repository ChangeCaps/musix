use crate::{arrangement::*, commands, settings, theme, AppState};
use druid::{widget::*, *};
use std::sync::Arc;

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

    pub fn update_children(&mut self, arrangement: &Arrangement) -> bool {
        let changed = self.children.len() != arrangement.tracks.len();

        self.children.truncate(arrangement.tracks.len());

        for (i, _track) in arrangement.tracks.iter().enumerate() {
            if i >= self.children.len() {
                self.children.push(WidgetPod::new(TrackWidget::new(i)));
            } else {
                self.children[i].widget_mut().idx = i;
            }
        }

        changed
    }
}

impl Widget<AppState> for ArrangementWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        for child in &mut self.children {
            child.event(ctx, event, data, env);
        }

        match event {
            Event::Wheel(mouse_event) => {
                let scroll_speed = env.get(settings::ARRANGEMENT_SCROLL_SPEED);

                if mouse_event.mods.shift {
                    self.scroll.y += mouse_event.wheel_delta.y * scroll_speed;

                    self.scroll.y = self
                        .scroll
                        .y
                        .max(-env.get(settings::ARRANGEMENT_TRACK_HEIGHT) / 2.0);
                } else if mouse_event.mods.ctrl {
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
                    Command::new(
                        commands::AUDIO_ENGINE_SET_PLAY_TIME,
                        time / env.get(settings::ARRANGEMENT_BEATS_PER_SECOND),
                    ),
                    Target::Global,
                );
                ctx.request_paint();
            }

            Event::Command(cmd) if cmd.is(commands::ARRANGEMENT_UPDATE_PLAY_LINE) => {
                let place = cmd.get_unchecked(commands::ARRANGEMENT_UPDATE_PLAY_LINE);

                self.play_line = *place * env.get(settings::ARRANGEMENT_BEATS_PER_SECOND);

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
            if self.update_children(&data.arrangement) {
                ctx.children_changed();
            }
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

#[derive(Clone)]
pub enum Selection {
    Some(usize, usize),
    None(usize),
}

pub struct TrackWidget {
    idx: usize,
    selection: Option<Selection>,
}

impl TrackWidget {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            selection: None,
        }
    }
}

impl Widget<AppState> for TrackWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        let track = &data.arrangement.tracks[self.idx];

        match event {
            Event::MouseDown(mouse_event) if mouse_event.button.is_left() => {
                if mouse_event.mods.shift {
                    let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                    let beat = (mouse_event.pos.x / beat_size).round() as usize;

                    Arc::make_mut(&mut data.arrangement.tracks)[self.idx].remove_block(beat);
                } else {
                    let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                    let beat = (mouse_event.pos.x / beat_size).round() as usize;

                    self.selection = track.get_selection(beat);
                }
            }

            Event::Command(cmd) if cmd.is(commands::GLOBAL_MOUSE_UP) => {
                self.selection = None;
            }

            Event::MouseDown(mouse_event) if mouse_event.button.is_right() => {
                let menu = ContextMenu::new(
                    MenuDesc::<AppState>::empty().append(MenuItem::new(
                        LocalizedString::new("Remove"),
                        Command::new(commands::ARRANGEMENT_REMOVE_TRACK, self.idx),
                    )),
                    mouse_event.window_pos,
                );
                ctx.show_context_menu(menu);
            }

            Event::MouseMove(mouse_event) => {
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let beat = (mouse_event.pos.x / beat_size).round() as usize;

                if let Some(selection) = self.selection.clone() {
                    let track = &mut Arc::make_mut(&mut data.arrangement.tracks)[self.idx];

                    match selection {
                        Selection::Some(selected_beat, block_index) => {
                            if track.move_block_bound(block_index, selected_beat, beat) {
                                self.selection = Some(Selection::Some(beat, block_index));
                            }
                        }

                        Selection::None(selected_beat) => {
                            if let Some(selected_audio_block_id) = data.selected_audio_block {
                                if beat != selected_beat {
                                    let audio_block =
                                        data.audio_blocks[&selected_audio_block_id].clone();

                                    if let Some(index) = track.add_block(Block::new(
                                        beat.min(selected_beat)..beat.max(selected_beat),
                                        selected_audio_block_id,
                                        audio_block.format,
                                    )) {
                                        self.selection = Some(Selection::Some(beat, index));
                                    }
                                }
                            }
                        }
                    }
                }
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

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, _env: &Env) {
        if !data.same(old_data) {
            ctx.request_paint();
        }
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

        let track = &data.arrangement.tracks[self.idx];

        while place < ctx.size().width {
            let beat = (place / beat_size).floor() as usize;
            let block = track.get_block(beat);
            let prev_block = if beat > 0 {
                track.get_block(beat - 1)
            } else {
                None
            };
            let audio_block = block.map(|b| &data.audio_blocks[&b.audio_block_id]);
            let prev_audio_block = prev_block.map(|b| &data.audio_blocks[&b.audio_block_id]);

            let color = audio_block.map(|b| b.color.clone()).unwrap_or(Color::WHITE);
            let prev_color = prev_audio_block
                .map(|b| b.color.clone())
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

            if let Some(audio_block) = audio_block {
                if (beat - block.unwrap().bounds.start) % audio_block.len_beats == 0
                    && beat != block.unwrap().bounds.start
                {
                    let rect = Rect::from_origin_size(
                        (place - 2.0 / 2.0, ctx.size().height / 2.0 - 8.0 / 2.0),
                        (2.0, 8.0),
                    );
                    ctx.fill(rect, &color);
                }
            }

            // small circle drawn at the end of each block
            if let Some(_prev_block) = prev_block {
                if block != prev_block {
                    let circle = kurbo::Circle::new((place, ctx.size().height / 2.0), 4.0);
                    ctx.fill(circle, &prev_color);
                }
            }

            // large circle drawn at the start of each block
            // TODO: fix white line overlap
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
