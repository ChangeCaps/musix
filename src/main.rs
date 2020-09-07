use druid::{widget::*, *};
use std::{collections::HashMap, sync::Arc};

mod arrangement;
mod audio;
mod deligate;
mod widgets;

use arrangement::*;

mod commands {
    use druid::Selector;

    pub const AUDIO_ENGINE_SET_PLAY_TIME: Selector<f64> =
        Selector::new("audio-engine.set-play-time");

    pub const SELECT_AUDIO_BLOCK: Selector<super::AudioBlockID> =
        Selector::new("select-audio-block");

    pub const ARRANGEMENT_ADD_TRACK: Selector<()> = Selector::new("arrangement.add-track");
    pub const ARRANGEMENT_REMOVE_TRACK: Selector<usize> = Selector::new("arrangement.remove-track");
    pub const ARRANGEMENT_UPDATE_PLAY_LINE: Selector<f64> =
        Selector::new("arrangement.update-play-line");
    //pub const ARRANGEMENT_ADD_BLOCK: Selector<>
}

mod settings {
    use druid::Key;

    pub const ARRANGEMENT_SCROLL_SPEED: Key<f64> = Key::new("arrangement.scroll-speed");
    pub const ARRANGEMENT_BEAT_SIZE: Key<f64> = Key::new("arrangement.beat-size");
    pub const ARRANGEMENT_TRACK_HEIGHT: Key<f64> = Key::new("arrangement.track-height");

    pub fn default(env: &mut druid::Env) {
        env.set(ARRANGEMENT_SCROLL_SPEED, 0.1);
        env.set(ARRANGEMENT_BEAT_SIZE, 40.0);
        env.set(ARRANGEMENT_TRACK_HEIGHT, 20.0);
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

    pub fn default(env: &mut druid::Env) {
        env.set(BORDER_COLOR, Color::WHITE);
        env.set(BORDER_WIDTH, 2.0);

        env.set(ARRANGEMENT_BEAT_LINE_WIDTH, 1.0);
        env.set(ARRANGEMENT_BEAT_LINE_COLOR, Color::rgb(0.2, 0.2, 0.2));
        env.set(ARRANGEMENT_TACT_LINE_COLOR, Color::rgb(0.4, 0.4, 0.4));
        env.set(ARRANGEMENT_PLAY_LINE_WIDTH, 3.5);
        env.set(ARRANGEMENT_PLAY_LINE_COLOR, Color::rgb(0.5, 0.5, 0.5));

        env.set(
            druid::theme::WINDOW_BACKGROUND_COLOR,
            Color::rgb(0.05, 0.05, 0.06),
        );
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Data)]
pub struct AudioBlockID(pub usize);

#[derive(Clone, Data, Lens)]
pub struct AudioBlock {
    audio_id: audio::AudioID,
    pub name: String,
    len_beats: usize,
    color: Color,
}

impl AudioBlock {
    pub fn new(audio_id: audio::AudioID) -> Self {
        Self {
            audio_id,
            name: "New Block".into(),
            len_beats: 0,
            color: Color::rgb(0.7, 0.2, 0.2),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub arrangement: arrangement::Arrangement,
    pub audio_blocks: Arc<HashMap<AudioBlockID, AudioBlock>>,
    pub shown_audio_blocks: Arc<Vec<AudioBlockID>>,
    pub listed_audio_blocks: Arc<Vec<AudioBlockID>>,
    pub selected_audio_block: Option<AudioBlockID>,
    pub next_audio_block_id: AudioBlockID,
    pub playing: bool,
    pub recording: bool,
    pub audio_engine_handle: audio::AudioEngineHandle,
}

fn create_block_list() -> impl Widget<AppState> {
    Scroll::new(List::new(|| {
        Flex::column()
            .with_spacer(4.0)
            .with_child(
                Label::dynamic(
                    |data: &(Arc<HashMap<AudioBlockID, AudioBlock>>, AudioBlockID), _| {
                        data.0[&data.1].name.clone()
                    },
                )
                .with_text_size(16.0),
            )
            .fix_size(120.0, 80.0)
            .background(Painter::new(
                |ctx, data: &(Arc<HashMap<AudioBlockID, AudioBlock>>, AudioBlockID), _| {
                    let rect = Rect::from_origin_size((0.0, 0.0), ctx.size());

                    ctx.fill(rect, &data.0[&data.1].color);
                },
            ))
            .rounded(5.0)
            .on_click(
                |ctx, data: &mut (Arc<HashMap<AudioBlockID, AudioBlock>>, AudioBlockID), _env| {
                    ctx.submit_command(
                        Command::new(commands::SELECT_AUDIO_BLOCK, data.1),
                        Target::Global,
                    );
                },
            )
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

fn create_block_menu(selected: AudioBlockID) -> Box<impl Widget<AppState>> {
    const NUM_COLORS: u32 = 20;

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
            .on_click(move |_ctx, data: &mut AudioBlock, _env| {
                data.color = cloned_color.clone();
            }),
        );
        block_color_pick.add_spacer(2.0);
    }

    Box::new(
        Flex::row()
            .with_child(
                Scroll::new(block_color_pick)
                    .vertical()
                    .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
                    .rounded(5.0),
            )
            .align_left()
            .lens(AppState::audio_blocks.map(
                move |data: &Arc<HashMap<AudioBlockID, AudioBlock>>| data[&selected].clone(),
                move |data, val| {
                    Arc::make_mut(data).insert(selected, val);
                },
            )),
    )
}

fn create_top_bar() -> impl Widget<AppState> {
    Flex::row()
        .with_child(ViewSwitcher::new(
            |data: &AppState, _| data.playing,
            |selector, _, _| match selector {
                true => Box::new(Button::new("Stop").on_click(
                    |_ctx, data: &mut AppState, _env| {
                        data.playing = false;
                        data.audio_engine_handle.set_playing(false);

                        if let Some(id) = data.audio_engine_handle.stop_recording() {
                            Arc::make_mut(&mut data.audio_blocks)
                                .insert(data.next_audio_block_id, AudioBlock::new(id));
                            Arc::make_mut(&mut data.shown_audio_blocks)
                                .push(data.next_audio_block_id);
                            data.next_audio_block_id.0 += 1;
                        }
                    },
                )),
                false => Box::new(
                    Flex::row()
                        .with_child(Button::new("Play").on_click(
                            |_ctx, data: &mut AppState, _env| {
                                data.playing = true;
                                data.audio_engine_handle.set_playing(true);
                            },
                        ))
                        .with_child(Button::new("Record").on_click(
                            |_ctx, data: &mut AppState, _env| {
                                data.recording = true;
                                data.playing = true;
                                data.audio_engine_handle.set_playing(true);
                                data.audio_engine_handle.start_recording();
                            },
                        )),
                ),
            },
        ))
        .align_left()
}

fn create_menu() -> impl druid::Widget<AppState> {
    Flex::column().with_child(create_top_bar()).with_flex_child(
        Flex::row().with_child(create_block_list()).with_flex_child(
            Flex::column()
                .with_flex_child(
                    ViewSwitcher::new(
                        |data: &AppState, _| data.selected_audio_block,
                        |selector, _, _| match selector {
                            Some(selected) => create_block_menu(*selected),
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
                        .rounded(5.0),
                    1.0,
                ),
            1.0,
        ),
        1.0,
    )
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
        listed_audio_blocks: Arc::new(Vec::new()),
        selected_audio_block: None,
        next_audio_block_id: AudioBlockID(0),
        playing: false,
        recording: false,
        audio_engine_handle,
    };

    launcher.launch(app_data).expect("launch failed");
}
