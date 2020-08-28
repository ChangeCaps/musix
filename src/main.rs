use druid::{widget::*, *};

mod arrangement;
mod audio;
mod deligate;

use arrangement::*;

const ADD_TRACK: Selector<()> = Selector::new("track-add-track");
const REMOVE_TRACK: Selector<()> = Selector::new("track-remove-track");

mod theme {
    use druid::{Color, Key};

    pub const BORDER_COLOR: Key<Color> = Key::new("border-color");
    pub const BORDER_WIDTH: Key<f64> = Key::new("border-width");

    pub fn default(env: &mut druid::Env) {
        env.set(BORDER_COLOR, Color::WHITE);
        env.set(BORDER_WIDTH, 2.0);
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub arrangement: arrangement::Arrangement,
}

fn create_menu() -> impl druid::Widget<AppState> {
    Flex::column().with_child(
        List::new(|| TrackWidget::new(50.0))
            .border(theme::BORDER_COLOR, theme::BORDER_WIDTH)
            .rounded(10.0)
            .lens(Arrangement::tracks)
            .lens(AppState::arrangement),
    )
}

fn make_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::empty()
        .append(druid::platform_menus::win::file::default())
        .append(
            MenuDesc::new(LocalizedString::new("Track"))
                .append(MenuItem::new(LocalizedString::new("Add Track"), ADD_TRACK))
                .append(MenuItem::new(
                    LocalizedString::new("Remove Track"),
                    REMOVE_TRACK,
                )),
        )
}

fn main() {
    simple_logger::init().unwrap();

    let app_data = AppState {
        arrangement: arrangement::Arrangement::new(),
    };

    let window_desc = druid::WindowDesc::new(create_menu).menu(make_menu());

    druid::AppLauncher::with_window(window_desc)
        .configure_env(|env, _| theme::default(env))
        .delegate(deligate::Deligate::default())
        .launch(app_data)
        .expect("launch failed");
}
