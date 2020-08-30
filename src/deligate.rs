use crate::commands;
use druid::*;

pub struct Deligate {}

impl Default for Deligate {
    fn default() -> Self {
        Self {}
    }
}

impl druid::AppDelegate<crate::AppState> for Deligate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut crate::AppState,
        _env: &Env,
    ) -> bool {
        match cmd {
            _ if cmd.is(commands::ARRANGEMENT_ADD_TRACK) => {
                data.arrangement.add_track();

                log::info!("Added Track");

                false
            }

            _ if cmd.is(commands::ARRANGEMENT_REMOVE_TRACK) => {
                let index = cmd.get_unchecked(commands::ARRANGEMENT_REMOVE_TRACK);
                data.arrangement.remove_track(*index);

                log::info!("Removed Track {}", index);

                false
            }

            _ if cmd.is(commands::AUDIO_ENGINE_SET_PLAY_TIME) => {
                let time = cmd.get_unchecked(commands::AUDIO_ENGINE_SET_PLAY_TIME);

                data.audio_engine_handle.set_play_time(*time);

                false
            }

            _ => true,
        }
    }
}
