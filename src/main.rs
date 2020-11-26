use druid::{widget::*, *};
use std::{collections::HashMap, sync::Arc};

mod arrangement;
mod audio;
mod audio_clip;
mod audio_source;
mod controllers;
mod deligate;
mod widgets;

use widgets::arrangement::*;

pub const ARRANGEMENT_WIDGET_ID: WidgetId = WidgetId::reserved(0);

mod commands {
    use druid::MouseEvent;
    use druid::Selector;

    pub const GLOBAL_MOUSE_UP: Selector<MouseEvent> = Selector::new("global.mouse_up");

    pub const AUDIO_ENGINE_SET_PLAY_TIME: Selector<f64> =
        Selector::new("audio-engine.set-play-time");

    pub const SELECT_AUDIO_BLOCK: Selector<super::AudioBlockID> =
        Selector::new("global.select-audio-block");

    pub const REMOVE_AUDIO_BLOCK: Selector<super::AudioBlockID> =
        Selector::new("global.remove-audio-block");

    pub const ARRANGEMENT_ADD_TRACK: Selector<()> = Selector::new("arrangement.add-track");
    pub const ARRANGEMENT_REMOVE_TRACK: Selector<usize> = Selector::new("arrangement.remove-track");
    pub const ARRANGEMENT_UPDATE_PLAY_LINE: Selector<f64> =
        Selector::new("arrangement.update-play-line");

    pub const GLOBAL_LOG_HISTORY: Selector<()> = Selector::new("global.log-history");
}

mod settings {
    use druid::Key;

    pub const ARRANGEMENT_SCROLL_SPEED: Key<f64> = Key::new("arrangement.scroll-speed");
    pub const ARRANGEMENT_BEAT_SIZE: Key<f64> = Key::new("arrangement.beat-size");
    pub const ARRANGEMENT_TRACK_HEIGHT: Key<f64> = Key::new("arrangement.track-height");
    pub const ARRANGEMENT_BEATS_PER_SECOND: Key<f64> = Key::new("arrangement.beats-per-second");

    pub fn default(env: &mut druid::Env) {
        env.set(ARRANGEMENT_SCROLL_SPEED, 0.1);
        env.set(ARRANGEMENT_BEAT_SIZE, 40.0);
        env.set(ARRANGEMENT_TRACK_HEIGHT, 30.0);
        env.set(ARRANGEMENT_BEATS_PER_SECOND, 120.0 / 60.0);
    }
}

mod theme {
    use druid::{Color, Key};

    pub const BORDER_COLOR: Key<Color> = Key::new("general.border-color");
    pub const BORDER_WIDTH: Key<f64> = Key::new("general.border-width");

    pub const ARRANGEMENT_BEAT_LINE_WIDTH: Key<f64> = Key::new("arrangement.beat-line-width");
    pub const ARRANGEMENT_BEAT_LINE_COLOR: Key<Color> = Key::new("arrangement.beat-line-color");
    pub const ARRANGEMENT_TACT_LINE_COLOR: Key<Color> = Key::new("arrangement.tact-line-color");
    pub const ARRANGEMENT_PLAY_LINE_WIDTH: Key<f64> = Key::new("arrangement.play-line-width");
    pub const ARRANGEMENT_PLAY_LINE_COLOR: Key<Color> = Key::new("arrangement.play-line-color");

    pub const AUDIO_CLIP_EDITOR_RESOLUTION: Key<f64> = Key::new("audio-clip-editor.resolution");
    pub const AUDIO_CLIP_EDITOR_SCALE: Key<f64> = Key::new("audio-clip-editor.scale");
    pub const AUDIO_CLIP_EDITOR_BAR_COLOR: Key<Color> = Key::new("audio-clip-editor.bar-color");

    pub fn default(env: &mut druid::Env) {
        env.set(BORDER_COLOR, Color::WHITE);
        env.set(BORDER_WIDTH, 2.0);

        env.set(ARRANGEMENT_BEAT_LINE_WIDTH, 1.0);
        env.set(ARRANGEMENT_BEAT_LINE_COLOR, Color::rgb(0.2, 0.2, 0.2));
        env.set(ARRANGEMENT_TACT_LINE_COLOR, Color::rgb(0.4, 0.4, 0.4));
        env.set(ARRANGEMENT_PLAY_LINE_WIDTH, 3.5);
        env.set(ARRANGEMENT_PLAY_LINE_COLOR, Color::rgb(0.5, 0.5, 0.5));

        env.set(AUDIO_CLIP_EDITOR_RESOLUTION, 1.0 / 80.0);
        env.set(AUDIO_CLIP_EDITOR_SCALE, 200.0);
        env.set(AUDIO_CLIP_EDITOR_BAR_COLOR, Color::rgb(0.6, 0.6, 0.6));

        env.set(
            druid::theme::WINDOW_BACKGROUND_COLOR,
            Color::rgb(0.05, 0.05, 0.055),
        );
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Data)]
pub struct AudioBlockID(pub usize);

#[derive(Clone, Data, Lens)]
pub struct AudioBlock {
    audio_id: audio::AudioSourceID,
    format: audio::AudioSourceFormat,
    offset: f32,
    len_beats: usize,
    true_len_beats: usize,
    color: Color,
}

impl AudioBlock {
    pub fn new(
        audio_id: audio::AudioSourceID,
        format: audio::AudioSourceFormat,
        beats_per_second: f64,
    ) -> Self {
        let true_len_beats = (format.len_frames as f64 / format.sample_rate as f64
            * beats_per_second)
            .ceil() as usize;

        Self {
            audio_id,
            format,
            offset: 0.0,
            len_beats: true_len_beats,
            true_len_beats,
            color: Color::rgb(0.7, 0.2, 0.2),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub arrangement: arrangement::Arrangement,
    pub audio_blocks: Arc<HashMap<AudioBlockID, AudioBlock>>,
    pub shown_audio_blocks: Arc<Vec<AudioBlockID>>,
    pub next_audio_block_id: AudioBlockID,
    pub audio_engine_handle: audio::AudioEngineHandle,

    pub selected_audio_block: Option<AudioBlockID>,
    pub selected_audio_source_clone: Option<audio_source::AudioSource>,
    pub beats_per_minute: f64,
    pub playing: bool,
    pub recording: bool,
    pub feedback: bool,
    pub metronome: bool,
    pub volume: f64,
}

impl AppState {
    pub fn history_changed(&self, other: &Self) -> bool {
        !(self.arrangement.same(&other.arrangement)
            && self.audio_blocks.same(&other.audio_blocks)
            && self.shown_audio_blocks.same(&other.shown_audio_blocks)
            && self.next_audio_block_id.same(&other.next_audio_block_id)
            && self.audio_engine_handle.same(&other.audio_engine_handle))
    }

    pub fn revert(&mut self, other: Self) {
        self.arrangement = other.arrangement;
        self.audio_blocks = other.audio_blocks;
        self.shown_audio_blocks = other.shown_audio_blocks;
        self.next_audio_block_id = other.next_audio_block_id;
        self.audio_engine_handle = other.audio_engine_handle;
    }
}

fn create_block_list() -> impl Widget<AppState> {
    Scroll::new(List::new(|| {
        Flex::column()
            .fix_size(120.0, 80.0)
            .background(Painter::new(
                |ctx, data: &(Arc<HashMap<AudioBlockID, AudioBlock>>, AudioBlockID), _| {
                    let rect = Rect::from_origin_size((0.0, 0.0), ctx.size());

                    ctx.fill(rect, &data.0[&data.1].color);
                },
            ))
            .rounded(5.0)
            .controller(controllers::EventController::new(
                |ctx,
                 event,
                 data: &mut (Arc<HashMap<AudioBlockID, AudioBlock>>, AudioBlockID),
                 _env| {
                    match event {
                        // on left click select block
                        Event::MouseDown(mouse_event) if mouse_event.button.is_left() => {
                            ctx.submit_command(
                                Command::new(commands::SELECT_AUDIO_BLOCK, data.1),
                                Target::Global,
                            );
                        }

                        // on right click, offer option to remove block
                        Event::MouseDown(mouse_event) if mouse_event.button.is_right() => {
                            let menu = ContextMenu::<AppState>::new(
                                MenuDesc::empty().append(MenuItem::new(
                                    LocalizedString::new("Remove"),
                                    Command::new(commands::REMOVE_AUDIO_BLOCK, data.1),
                                )),
                                mouse_event.window_pos,
                            );

                            ctx.show_context_menu(menu);
                        }

                        _ => (),
                    }
                },
            ))
    }))
    .expand_height()
    .fix_width(120.0)
    .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
    .rounded(5.0)
    .lens(lens::Id.map(
        |data: &AppState| (data.audio_blocks.clone(), data.shown_audio_blocks.clone()),
        |data, val| data.audio_blocks = val.0,
    ))
}

fn create_block_menu(selected: AudioBlockID) -> impl Widget<AppState> {
    const NUM_COLORS: u32 = 30;

    let mut block_color_pick = Flex::column();

    for i in 0..NUM_COLORS {
        let color = Color::hlc(i as f64 / NUM_COLORS as f64 * 360.0, 70.0, 127.0);
        let cloned_color = color.clone();

        block_color_pick.add_child(
            Painter::new(move |ctx, _, _| {
                let rect = Rect::from_origin_size((0.0, 0.0), ctx.size()).to_rounded_rect(5.0);
                ctx.fill(rect, &color);
            })
            .fix_size(30.0, 20.0)
            .on_click(move |ctx, data: &mut AudioBlock, _env| {
                data.color = cloned_color.clone();
                ctx.window().invalidate();
            }),
        );
        block_color_pick.add_spacer(2.0);
    }

    Scroll::new(block_color_pick)
        .vertical()
        .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
        .rounded(5.0)
        .align_left()
        .lens(AppState::audio_blocks.map(
            move |data: &Arc<HashMap<AudioBlockID, AudioBlock>>| data[&selected].clone(),
            move |data, val| {
                Arc::make_mut(data).insert(selected, val);
            },
        ))
}

fn create_top_bar() -> impl Widget<AppState> {
    Flex::row()
        .with_child(ViewSwitcher::new(
            |data: &AppState, _| data.playing,
            |selector, _, _| match selector {
                true => Box::new(
                    Button::new("Stop").on_click(|_ctx, data: &mut AppState, env| {
                        data.playing = false;
                        data.audio_engine_handle.set_playing(false);

                        if let Some((id, format)) = data.audio_engine_handle.stop_recording() {
                            log::info!("{:?}", format);

                            Arc::make_mut(&mut data.audio_blocks).insert(
                                data.next_audio_block_id,
                                AudioBlock::new(
                                    id,
                                    format,
                                    env.get(settings::ARRANGEMENT_BEATS_PER_SECOND),
                                ),
                            );
                            Arc::make_mut(&mut data.shown_audio_blocks)
                                .push(data.next_audio_block_id);
                            data.next_audio_block_id.0 += 1;
                        }
                    }),
                ),
                false => Box::new(
                    Flex::row()
                        .with_child(Button::new("Play").on_click(
                            |_ctx, data: &mut AppState, _env| {
                                data.playing = true;
                                data.audio_engine_handle.set_playing(true);

                                let arrangement_index =
                                    data.arrangement.compile_index(&data.audio_blocks);

                                data.audio_engine_handle
                                    .set_arrangement_index(arrangement_index);
                            },
                        ))
                        .with_child(Button::new("Record").on_click(
                            |_ctx, data: &mut AppState, _env| {
                                data.recording = true;
                                data.playing = true;
                                data.audio_engine_handle.start_recording();

                                let arrangement_index =
                                    data.arrangement.compile_index(&data.audio_blocks);

                                data.audio_engine_handle
                                    .set_arrangement_index(arrangement_index);
                            },
                        )),
                ),
            },
        ))
        .with_spacer(5.0)
        .with_child(Checkbox::new("Feedback").lens(lens::Id.map(
            |data: &AppState| data.feedback,
            |data, val| {
                data.feedback = val;
                data.audio_engine_handle.set_feedback(data.feedback);
            },
        )))
        .with_spacer(15.0)
        .with_child(Label::new("Volume"))
        .with_child(Slider::new().with_range(0.0, 5.0).lens(lens::Map::new(
            |data: &AppState| data.volume,
            |data, val| {
                data.volume = val;
                data.audio_engine_handle.set_volume(data.volume);
            },
        )))
        .with_spacer(15.0)
        .with_child(Label::new("bpm"))
        .with_child(
            TextBox::new()
                .with_placeholder("0")
                .lens(lens::Map::new(
                    |data: &AppState| {
                        if data.beats_per_minute == 0.0 {
                            "".to_owned()
                        } else {
                            data.beats_per_minute.to_string()
                        }
                    },
                    |data, val| {
                        if let Ok(val) = val.parse() {
                            data.beats_per_minute = val;
                            data.audio_engine_handle
                                .set_beats_per_second(data.beats_per_minute / 60.0);
                        } else if val == "" {
                            data.beats_per_minute = 0.0;
                        }
                    },
                ))
                .fix_width(35.0),
        )
        .with_spacer(15.0)
        .with_child(Checkbox::new("Metronome").lens(lens::Map::new(
            |data: &AppState| data.metronome,
            |data, val| {
                data.metronome = val;
                data.audio_engine_handle.set_metronome(val);
            },
        )))
        .align_left()
}

fn create_menu() -> impl druid::Widget<AppState> {
    Flex::column()
        .with_child(create_top_bar())
        .with_flex_child(
            Flex::row().with_child(create_block_list()).with_flex_child(
                Flex::column()
                    .with_flex_child(
                        ViewSwitcher::new(
                            |data: &AppState, _| data.selected_audio_block,
                            |selector, data, _| match selector {
                                Some(selected) => {
                                    let mut row =
                                        Flex::row().with_child(create_block_menu(*selected));

                                    if let Some(source) = &data.selected_audio_source_clone {
                                        row.add_flex_child(source.editor_widget(), 1.0);
                                    }

                                    Box::new(row)
                                }
                                None => Box::new(Flex::row().align_left()),
                            },
                        )
                        .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
                        .rounded(5.0),
                        1.2,
                    )
                    .with_flex_child(
                        ArrangementWidget::new()
                            .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
                            .rounded(5.0)
                            .with_id(ARRANGEMENT_WIDGET_ID),
                        1.0,
                    ),
                1.0,
            ),
            1.0,
        )
        .controller(GlobalController)
        .env_scope(|env, data: &AppState| {
            env.set(
                settings::ARRANGEMENT_BEATS_PER_SECOND,
                data.beats_per_minute / 60.0,
            );
        })
}

struct GlobalController;

impl<T, W: Widget<T>> Controller<T, W> for GlobalController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseUp(mouse_event)
                if mouse_event.button.is_left() || mouse_event.button.is_right() =>
            {
                ctx.submit_command(
                    Command::new(commands::GLOBAL_MOUSE_UP, mouse_event.clone()),
                    None,
                );
            }
            _ => (),
        }

        child.event(ctx, event, data, env);
    }
}

fn make_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::empty()
        .append(druid::platform_menus::win::file::default())
        .append(
            MenuDesc::new(LocalizedString::new("Track")).append(MenuItem::new(
                LocalizedString::new("Add Track"),
                commands::ARRANGEMENT_ADD_TRACK,
            )),
        )
}

fn main() {
    simple_logger::init().unwrap();

    let window_desc = druid::WindowDesc::new(create_menu)
        .window_size((1000.0, 500.0))
        .menu(make_menu())
        .title("Musix");

    let launcher = druid::AppLauncher::with_window(window_desc)
        .configure_env(|env, _| {
            theme::default(env);
            settings::default(env);
        })
        .delegate(deligate::Deligate::default());

    let (audio_engine, audio_engine_handle) =
        audio::AudioEngine::new(launcher.get_external_handle());
    audio_engine.run();

    let app_data = AppState {
        arrangement: arrangement::Arrangement::new(),
        audio_blocks: Arc::new(HashMap::new()),
        shown_audio_blocks: Arc::new(Vec::new()),
        selected_audio_block: None,
        selected_audio_source_clone: None,
        next_audio_block_id: AudioBlockID(0),
        playing: false,
        recording: false,
        feedback: true,
        metronome: true,
        audio_engine_handle,
        volume: 2.5,
        beats_per_minute: 120.0,
    };

    launcher.launch(app_data).expect("launch failed");
}
