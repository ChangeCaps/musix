use druid::{widget::*, *};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
pub struct Arrangement {
    pub tracks: Arc<Vec<Track>>,
}

impl Arrangement {
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(vec![Track::new()]),
        }
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
                (0, Block::new(4, crate::audio::AudioID(4))),
                (4, Block::new(4, crate::audio::AudioID(4))),
            ]),
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

pub struct TrackWidget {
    blocks: Vec<WidgetPod<Block, Box<dyn Widget<Block> + 'static>>>,
    height: f64,
}

impl TrackWidget {
    pub fn new(height: f64) -> Self {
        Self {
            blocks: Vec::new(),
            height,
        }
    }

    pub fn update_children(&mut self, data: &Track) {
        let mut new_blocks = Vec::new();

        for (_space, _block) in &*data.blocks {
            let widget = Painter::new(|ctx, _block: &Block, _env| {
                let size = ctx.size();

                let rect = Rect::from_origin_size((0.0, 0.0), size).to_rounded_rect(10.0);

                ctx.fill(rect, &Color::BLACK);
            });

            new_blocks.push(WidgetPod::new(Box::new(widget) as Box<dyn Widget<_>>));
        }

        self.blocks = new_blocks;
    }
}

impl Widget<Track> for TrackWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Track, env: &Env) {
        let mut children = self.blocks.iter_mut();
        data.blocks.for_each_mut(|(_, child_data), _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Track, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.update_children(data);
            ctx.children_changed();
        }

        for i in 0..self.blocks.len() {
            self.blocks[i].lifecycle(ctx, event, &data.blocks[i].1, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Track, data: &Track, env: &Env) {
        for i in 0..self.blocks.len() {
            self.blocks[i].update(ctx, &data.blocks[i].1, env);
        }

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

        for i in 0..self.blocks.len() {
            let (space, block) = &data.blocks[i];
            let s = Size::new(block.length as f64 * 10.0, size.height);
            let bc = BoxConstraints::new(s, s);
            let block_size = self.blocks[i].layout(ctx, &bc, block, env);

            size.width += *space as f64 * 10.0;

            let rect = Rect::from_origin_size((size.width, 0.0), block_size);

            size.width += block_size.width;
            self.blocks[i].set_layout_rect(ctx, block, env, rect);
        }

        size.width = size.width.max(bc.max().width);

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Track, env: &Env) {
        for i in 0..self.blocks.len() {
            self.blocks[i].paint(ctx, &data.blocks[i].1, env);
        }
    }
}
