use crate::audio::*;
use druid::*;
use std::sync::Arc;

#[derive(Clone, Data)]
pub struct AudioClip {
    format: AudioSourceFormat,
    samples: Arc<Vec<f32>>,
}

impl AudioClip {
    pub fn new(samples: Vec<f32>, format: AudioSourceFormat) -> Self {
        Self {
            format,
            samples: Arc::new(samples),
        }
    }

    pub fn empty(format: AudioSourceFormat) -> Self {
        Self {
            format,
            samples: Arc::new(Vec::new()),
        }
    }

    pub fn append_sample(&mut self, sample: f32) {
        Arc::make_mut(&mut self.samples).push(sample);
        self.format.len_frames = self.samples.len() as u32 / self.format.channels;
    }

    pub fn clean(&mut self) {
        let len = self.samples.len();
        Arc::make_mut(&mut self.samples).truncate(len - len % self.format.channels as usize);
    }

    pub fn len_samples(&self) -> usize {
        self.samples.len()
    }
}

impl AudioSource for AudioClip {
    fn get_sample(&self, frame: u32, channel: u32, beats_per_second: f64) -> Option<f32> {
        self.samples
            .get(
                ((frame as f64 * self.format.channels as f64 + channel as f64)
                    * self.format.beats_per_second
                    / beats_per_second)
                    .round() as usize,
            )
            .map(|x| *x)
    }

    fn format(&self) -> AudioSourceFormat {
        self.format.clone()
    }

    fn widget(&self) -> Box<dyn druid::Widget<Arc<dyn AudioSource>>> {
        Box::new(druid::widget::Flex::row().lens(lens::Map::new(
            |data: &Arc<dyn AudioSource>| {
                // this is hell, but im also kinda proud of the solution, fuck me, why didn't i
                // just use a god dammed enum
                if (**data).type_id() == std::any::TypeId::of::<AudioClip>() {
                    unsafe { &*(&**data as *const dyn AudioSource as *const Self) }.clone()
                } else {
                    panic!("yeet");
                }
            },
            |data, val| {
                *data = Arc::new(val);
            },
        )))
    }
}
