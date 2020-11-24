use crate::{audio::AudioSource, audio_clip::AudioClip, theme, AudioBlock};
use druid::*;

pub struct AudioClipEditor {
    scroll: f64,
}

impl AudioClipEditor {
    pub fn new() -> Self {
        Self { scroll: 0.5 }
    }
}

impl Widget<(AudioClip, AudioBlock)> for AudioClipEditor {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut (AudioClip, AudioBlock), _env: &Env) {
        match event {
            Event::Wheel(mouse_event) => {
                self.scroll += mouse_event.wheel_delta.y * 0.0001;

                self.scroll = self.scroll.max(0.0);
                self.scroll = self.scroll.min(1.0);

                ctx.request_paint();
            }

            _ => ()
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &(AudioClip, AudioBlock),
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: &(AudioClip, AudioBlock),
        _data: &(AudioClip, AudioBlock),
        _env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &(AudioClip, AudioBlock),
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, bc.max().height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, (audio_clip, audio_block): &(AudioClip, AudioBlock), env: &Env) {
        let size = ctx.size();

        let format = audio_clip.format();

        ctx.with_save(|ctx| {
            ctx.clip(size.to_rect());

            let clip_size = audio_clip.len_seconds() * env.get(theme::AUDIO_CLIP_EDITOR_SCALE);

            let scroll_offset = self.scroll * size.width - clip_size / 2.0;
            ctx.transform(Affine::translate(Vec2::new(scroll_offset, 0.0)));

            // draw beat lines
            let beat_size = env.get(theme::AUDIO_CLIP_EDITOR_SCALE) / 4.0 * format.beats_per_second;
            let beat_line_width = env.get(theme::ARRANGEMENT_BEAT_LINE_WIDTH);
            let starting_beat = (-scroll_offset / beat_size).floor() * beat_size;
            let starting_beat_num = (starting_beat / beat_size) as i32;

            let mut beat = starting_beat;
            let mut beat_num = starting_beat_num;

            while beat <= size.width - scroll_offset {
                let rect = Rect::from_origin_size(
                    (beat - beat_line_width / 2.0, 0.0),
                    (beat_line_width, ctx.size().height),
                );

                let color = if beat_num % 4 == 0 {
                    env.get(theme::ARRANGEMENT_TACT_LINE_COLOR)
                } else {
                    env.get(theme::ARRANGEMENT_BEAT_LINE_COLOR)
                };

                ctx.fill(rect, &color);

                beat += beat_size;
                beat_num += 1;
            }

            // draw the clip visulaization
            let num_bars =
                (audio_clip.len_seconds() / env.get(theme::AUDIO_CLIP_EDITOR_RESOLUTION)).ceil() as u32;
            let bar_width = env.get(theme::AUDIO_CLIP_EDITOR_SCALE)
                * env.get(theme::AUDIO_CLIP_EDITOR_RESOLUTION);
            let bar_frames =
                (env.get(theme::AUDIO_CLIP_EDITOR_RESOLUTION) * format.sample_rate as f64) as u32;

            for bar in 0..num_bars {
                let bar_height = audio_clip
                    .get_sample(bar * bar_frames, 0, format.beats_per_second)
                    .unwrap_or(0.0) as f64;

                let rect = Rect::from_center_size(
                    (bar as f64 * bar_width + bar_width / 2.0, size.height / 2.0),
                    (bar_width + 1.0, bar_height * size.height * 1.5),
                );

                ctx.fill(rect, &audio_block.color);
            }

            // draw block bounds
        });
    }
}
