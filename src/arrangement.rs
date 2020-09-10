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

    pub fn remove_track(&mut self, idx: usize) {
        Arc::make_mut(&mut self.tracks).remove(idx);
    }
}

#[derive(Clone)]
pub struct Track {
    blocks: Vec<(usize, Block)>,
}

impl Track {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            blocks: Vec::new(),
        }
    }

    pub fn get_idx(&self, beat: usize) -> Option<usize> {
        let mut place = 0;

        for (i, (space, block)) in self.blocks.iter().enumerate() {
            place += space;

            if beat < place {
                return None;
            }

            place += block.length;

            if beat < place {
                return Some(i);
            }
        }

        None
    }

    pub fn get_block(&self, beat: usize) -> Option<&Block> {
        self.get_idx(beat).map(|i| &self.blocks[i].1)
    }

    pub fn get_selection(&self, beat: usize) -> Option<Selection> {
        // FIXME: this is bad, very bad, please find a better way to do this. please.
        //      -Hjalte Nannestad 10-09-2020

        if let Some(i) = self.get_idx(beat) {
            log::info!("idx: {}", i);
            if beat == self.get_start(i) {
                log::info!("selected start of: {}", i);
                Some(Selection::Start(i))
            } else {
                None
            }
        } else if let Some(i) = self.get_idx(beat - 1) {
            if beat == self.get_end(i) {
                log::info!("selected end of: {}", i);
                Some(Selection::End(i))
            } else {
                None
            }
        } else {
            log::info!("selected new: {}", beat);
            Some(Selection::New(beat))
        }
    }

    pub fn get_start(&self, idx: usize) -> usize {
        let mut start = 0;

        for i in 0..idx {
            start += self.blocks[i].0;
            start += self.blocks[i].1.length;
        }

        start + self.blocks[idx].0
    }

    pub fn get_end(&self, idx: usize) -> usize {
        let mut end = 0;

        for i in 0..idx + 1 {
            end += self.blocks[i].0;
            end += self.blocks[i].1.length;
        }

        end
    }

    /// Adds a block or atleast it tries
    pub fn add_block(&mut self, audio_id: crate::audio::AudioID, a: usize, b: usize) -> Result<(), ()> {
        let start = a.min(b);
        let end = a.max(b);

        let block = Block::new(end-start, audio_id);

        for i in 0..self.blocks.len() {
            let b_end = self.get_end(i);
            
            if start > b_end && self.blocks.get(i+1).map(|_| end < self.get_start(i + 1)).unwrap_or(true) {
                self.blocks.insert(i + 1, (start - b_end, block));

                if i + 2 < self.blocks.len() {
                    self.blocks[i + 2].0 -= start - b_end + (end - start);
                }

                return Ok(())
            }
        }

        if self.blocks.len() > 0 {

        } else {
            self.blocks.push((start, block));
            return Ok(());
        }

        Err(())
    }

    /// Moves the start of a block
    pub fn move_start(&mut self, idx: usize, target: usize) -> Result<(), ()> {
        let end = if idx > 0 { self.get_end(idx - 1) } else { 0 };

        if target < end || target >= self.get_end(idx) {
            Err(())
        } else {
            let blocks = &mut self.blocks;
            let old_offset = blocks[idx].0;
            blocks[idx].0 = target - end;
            blocks[idx].1.length += old_offset - blocks[idx].0;

            Ok(())
        }
    }

    pub fn move_end(&mut self, idx: usize, target: usize) -> Result<(), ()> {
        let start = self.get_start(idx);

        if self
            .blocks
            .get(idx + 1)
            .map(|_| target > self.get_start(idx + 1))
            .unwrap_or(false)
            || target <= self.get_start(idx)
        {
            Err(())
        } else {
            let blocks = &mut self.blocks;
            let old_length = blocks[idx].1.length;
            blocks[idx].1.length = target - start;

            if blocks.len() > idx + 1 {
                blocks[idx + 1].0 += old_length - blocks[idx].1.length;
            }

            Ok(())
        }
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

pub enum Selection {
    Start(usize),
    End(usize),
    New(usize),
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
                let beat_size = env.get(settings::ARRANGEMENT_BEAT_SIZE);
                let beat = (mouse_event.pos.x / beat_size).round() as usize;

                self.selection = track.get_selection(beat);
            }

            Event::MouseUp(mouse_event) if mouse_event.button.is_left() => {
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

                if let Some(selection) = &self.selection {
                    let track = &mut Arc::make_mut(&mut data.arrangement.tracks)[self.idx];

                    match selection {
                        Selection::Start(i) => {
                            if track.move_start(*i, beat).is_ok() {
                                ctx.request_paint();
                            }
                        }

                        Selection::End(i) => {
                            if track.move_end(*i, beat).is_ok() {
                                ctx.request_paint();
                            }
                        }

                        Selection::New(i) if beat != *i && data.selected_audio_block.is_some() => {
                            if track.add_block(data.audio_blocks[&data.selected_audio_block.unwrap()].audio_id, *i, beat).is_ok() {
                                ctx.request_paint();
                                log::info!("added, {}, {}", *i, beat);
                            }
                        }

                        _ => (),
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

        let track = &data.arrangement.tracks[self.idx];

        while place < ctx.size().width {
            let block_idx = track.get_idx((place / beat_size).floor() as usize);
            let prev_block_idx = track.get_idx((place / beat_size).floor() as usize - 1);
            let block = block_idx.map(|i| &track.blocks[i].1);
            let prev_block = prev_block_idx.map(|i| &track.blocks[i].1);

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

            // small circle drawn at the end of each block
            if let Some(_prev_block) = prev_block {
                if block_idx != prev_block_idx {
                    let circle = kurbo::Circle::new((place, ctx.size().height / 2.0), 4.0);
                    ctx.fill(circle, &prev_color);
                }
            }

            // large circle drawn at the start of each block
            // TODO: fix white line overlap
            if let Some(_block) = block {
                if block_idx != prev_block_idx {
                    let circle = kurbo::Circle::new((place, ctx.size().height / 2.0), 6.0);
                    ctx.stroke(circle, &color, 1.0);
                }
            }

            place += beat_size;
        }
    }
}
