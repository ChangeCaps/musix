use crate::{audio_clip::AudioClip, AppState};
use druid::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Clone, Debug, druid::Data, PartialEq, Serialize, Deserialize)]
pub struct AudioSourceFormat {
    pub sample_rate: u32,
    pub len_frames: u32,
    pub channels: u32,
    pub beats_per_second: f64,
}


// i actually use an enum now and i don't want to hurt myself anymore
#[derive(Clone, Data, Serialize, Deserialize)]
pub enum AudioSource {
    AudioClip(Arc<AudioClip>),
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
