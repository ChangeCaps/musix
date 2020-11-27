use crate::{commands, AppState, AppStateHistory};
use druid::*;
use std::sync::Arc;

#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub struct HistoryID(u64);

pub struct History<T> {
    history: Vec<(T, HistoryID)>,
    current_data: Option<T>,
}

pub struct Deligate {
    history: History<AppStateHistory>,
}

impl Default for Deligate {
    fn default() -> Self {
        Self {
            history: History::new(),
        }
    }
}

impl<T: Data> History<T> {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_data: None,
        }
    }

    pub fn update_current_data(&mut self, data: &T) {
        if let None = self.current_data {
            self.current_data = Some(data.clone());
        }
    }

    pub fn log_history(&mut self, data: &T) -> Option<HistoryID> {
        if let Some(current_data) = &self.current_data {
            if current_data.same(&data) {
                return None;
            }
        }

        if let Some(current_data) = std::mem::replace(&mut self.current_data, Some(data.clone())) {
            let mut last_history_id = self
                .history
                .last()
                .map(|(_, id)| *id)
                .unwrap_or(HistoryID(0));

            last_history_id.0 += 1;

            self.history.push((current_data, last_history_id));

            Some(last_history_id)
        } else {
            None
        }
    }

    pub fn clear(&mut self, data: &T) {
        self.history.clear();
        self.current_data = Some(data.clone());
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn revert(&mut self) -> Option<(T, HistoryID)> {
        self.history.pop().map(|state| {
            self.current_data = Some(state.0.clone());
            state
        })
    }

    pub fn revert_to(&mut self, target_history_id: HistoryID) -> Option<T> {
        while let Some((_, history_id)) = self.history.last() {
            if history_id.0 > target_history_id.0 {
                self.history.pop();
            } else if history_id.0 < target_history_id.0 {
                return None;
            } else {
                return Some(self.history.pop().unwrap().0);
            }
        }

        return None;
    }
}

impl druid::AppDelegate<AppState> for Deligate {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        _window_id: WindowId,
        event: Event,
        data: &mut AppState,
        _env: &Env,
    ) -> Option<Event> {
        if self.history.current_data.is_none() {
            self.history.current_data = Some(AppStateHistory::from_app_state(data));
        }

        match event {
            Event::KeyDown(key_event)
                if key_event.key_code == KeyCode::KeyZ
                    && key_event.mods.ctrl
                    && key_event.mods.shift =>
            {
                ctx.submit_command(druid::commands::REDO, Target::Global);
            }

            Event::KeyDown(key_event)
                if key_event.key_code == KeyCode::KeyZ && key_event.mods.ctrl =>
            {
                ctx.submit_command(druid::commands::UNDO, Target::Global);
            }

            _ => (),
        };

        Some(event)
    }

    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut crate::AppState,
        _env: &Env,
    ) -> bool {
        match cmd {
            _ if cmd.is(commands::ARRANGEMENT_ADD_TRACK) => {
                data.arrangement.add_track();

                log::info!("Added Track");

                ctx.submit_command(commands::GLOBAL_LOG_HISTORY, Target::Global);

                false
            }

            _ if cmd.is(commands::ARRANGEMENT_REMOVE_TRACK) => {
                let index = cmd.get_unchecked(commands::ARRANGEMENT_REMOVE_TRACK);
                data.arrangement.remove_track(*index);

                log::info!("Removed Track {}", index);

                ctx.submit_command(commands::GLOBAL_LOG_HISTORY, Target::Global);

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

                ctx.submit_command(commands::GLOBAL_LOG_HISTORY, Target::Global);

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

                false
            }

            _ if cmd.is(commands::GLOBAL_LOG_HISTORY) => {
                self.history
                    .log_history(&AppStateHistory::from_app_state(data));

                data.audio_engine_handle.log_history();

                false
            }

            _ if cmd.is(druid::commands::UNDO) => {
                log::info!("Undo {}", self.history.len());

                if let Some((new_data, history_id)) = self.history.revert() {
                    data.revert(new_data);
                    data.audio_engine_handle.revert_history(history_id);
                }

                false
            }

            _ => true,
        }
    }
}
