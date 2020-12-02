use crate::{audio_source::*, widgets, AppState};
use druid::{widget::*, *};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioClip {
    format: AudioSourceFormat,
    samples: Vec<f32>,
}

impl AudioClip {
    pub fn new(samples: Vec<f32>, format: AudioSourceFormat) -> Self {
        Self {
            format,
            samples,
        }
    }

    pub fn empty(format: AudioSourceFormat) -> Self {
        Self {
            format,
            samples: Vec::new(),
        }
    }

    pub fn append_sample(&mut self, sample: f32) {
        self.samples.push(sample);
        self.format.len_frames = self.samples.len() as u32 / self.format.channels;
    }

    pub fn clean(&mut self) {
        let len = self.samples.len();
        self.samples.truncate(len - len % self.format.channels as usize);
        let len = self.samples.len();
    
        let sample_fraction = self.format.sample_rate as usize / 100;

        for i in 0..sample_fraction {
            let modulate = i as f32 / sample_fraction as f32;

            self.samples[i] *= modulate;
            self.samples[len - i - 1] *= modulate;
        }
    }

    pub fn len_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn get_sample(&self, frame: u32, channel: u32, beats_per_second: f64) -> Option<f32> {
        self.samples
            .get(
                ((frame as f64 * self.format.channels as f64 + channel as f64)
                    * (beats_per_second / self.format.beats_per_second))
                    .round() as usize,
            )
            .map(|x| *x)
    }

    pub fn len_seconds(&self) -> f64 {
        self.format.len_frames as f64 / self.format.sample_rate as f64
    }

    pub fn format(&self) -> AudioSourceFormat {
        self.format.clone()
    }

    pub fn editor_widget(&self) -> impl Widget<AppState> {
        druid::widget::Flex::row()
            .with_flex_child(widgets::audio_clip_editor::AudioClipEditor::new(), 1.0)
            .lens(lens::Map::new(
                |data: &AppState| {
                    if let AudioSource::AudioClip(audio_clip) =
                        data.selected_audio_source_clone.as_ref().unwrap()
                    {
                        (
                            audio_clip.clone(),
                            data.audio_blocks[&data.selected_audio_block.unwrap()].clone(),
                        )
                    } else {
                        panic!("yeet");
                    }
                },
                |data, val| {
                    if let AudioSource::AudioClip(audio_clip) =
                        data.selected_audio_source_clone.as_mut().unwrap()
                    {
                        if !audio_clip.same(&val.0) {
                            *audio_clip = val.0;
                        }
                    }
                    
                    if !data.audio_blocks[&data.selected_audio_block.unwrap()].same(&val.1) {
                        *Arc::make_mut(&mut data.audio_blocks)
                            .get_mut(&data.selected_audio_block.unwrap())
                            .unwrap() = val.1;
                    }
                },
            ))
    }
}
