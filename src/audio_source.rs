use crate::{audio_clip::AudioClip, AppState};
use druid::*;

// i actually use an enum now and i don't want to hurt myself anymore
#[derive(Clone, Data)]
pub enum AudioSource {
    AudioClip(AudioClip),
}

impl AudioSource {
    pub fn editor_widget(&self) -> Box<dyn Widget<AppState>> {
        match self {
            Self::AudioClip(audio_clip) => Box::new(audio_clip.editor_widget()),
        }
    }

    pub fn get_sample(&self, frame: u32, channel: u32, beats_per_second: f64) -> Option<f32> {
        match self {
            Self::AudioClip(audio_clip) => audio_clip.get_sample(frame, channel, beats_per_second),
        }
    }
}
