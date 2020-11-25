use crate::{commands, AppState};
use druid::*;
use std::sync::Arc;

pub struct Deligate {
    history: Vec<AppState>,
    current_data: Option<AppState>,
}

impl Default for Deligate {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            current_data: None,
        }
    }
}

impl Deligate {
    pub fn log_history(&mut self, data: &AppState) {
        if let Some(current_data) = &self.current_data {
            if current_data.same(data) {
                return;
            }
        }

        if let Some(current_data) = std::mem::replace(&mut self.current_data, Some(data.clone())) {
            self.history.push(current_data);
        }
    }
}

impl druid::AppDelegate<AppState> for Deligate {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        _window_id: WindowId,
        event: Event,
        data: &mut AppState,
        _env: &Env
    ) -> Option<Event> {
        if self.current_data.is_none() {
            self.current_data = Some(data.clone());
        }

        match event {
            Event::KeyDown(key_event) if key_event.key_code == KeyCode::KeyZ && key_event.mods.ctrl => {
                ctx.submit_command(druid::commands::UNDO, Target::Global);
            }

            _ => ()
        };

        Some(event)
    }

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

                self.log_history(data);

                false
            }

            _ if cmd.is(commands::ARRANGEMENT_REMOVE_TRACK) => {
                let index = cmd.get_unchecked(commands::ARRANGEMENT_REMOVE_TRACK);
                data.arrangement.remove_track(*index);

                log::info!("Removed Track {}", index);

                self.log_history(data);

                false
            }

            _ if cmd.is(commands::AUDIO_ENGINE_SET_PLAY_TIME) => {
                let time = cmd.get_unchecked(commands::AUDIO_ENGINE_SET_PLAY_TIME);

                data.audio_engine_handle.set_play_time(*time);

                false
            }

            _ if cmd.is(commands::SELECT_AUDIO_BLOCK) => {
                let id = cmd.get_unchecked(commands::SELECT_AUDIO_BLOCK);

                data.selected_audio_block = Some(*id);
                let audio_blocks = &data.audio_blocks[id];

                data.selected_audio_source_clone = Some(
                    data.audio_engine_handle
                        .get_audio_source_clone(audio_blocks.audio_id),
                );

                self.log_history(data);

                false
            }

            _ if cmd.is(commands::REMOVE_AUDIO_BLOCK) => {
                let id = cmd.get_unchecked(commands::REMOVE_AUDIO_BLOCK);

                if data.selected_audio_block == Some(*id) {
                    data.selected_audio_block = None;
                    data.selected_audio_source_clone = None;
                }

                Arc::make_mut(&mut data.shown_audio_blocks).retain(|x| x != id);
                Arc::make_mut(&mut data.audio_blocks).remove(id);
                data.arrangement.remove_audio_block(*id);

                self.history.clear();
                self.current_data = Some(data.clone());

                false
            }

            _ if cmd.is(commands::GLOBAL_LOG_HISTORY) => {
                self.log_history(data);

                false
            }

            _ if cmd.is(druid::commands::UNDO) => {
                log::info!("Undo {}", self.history.len());

                if let Some(new_data) = self.history.pop() {
                    *data = new_data;
                    self.current_data = Some(data.clone());
                }

                false
            }

            _ => true,
        }
    }
}
