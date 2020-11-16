use crate::{commands::*, arrangement::*};
use cpal::traits::*;
use druid::Target;
use log::*;
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, druid::Data)]
pub struct AudioSourceID(pub usize);

pub enum Command {
    SetPlaying(bool),
    SetRecording(bool),
    SetPlayTime(f64),
    SetFeedback(bool),
    SetBeatsPerSecond(f64),
    SetArrangementAudioSourceIndex(ArrangementAudioSourceIndex),
}

pub enum CommandResponse {
    SetRecording(Option<(AudioSourceID, AudioSourceFormat)>),
}

#[derive(Clone, druid::Data)]
pub struct AudioEngineHandle {
    sender: std::sync::Arc<Sender<Command>>,
    receiver: std::sync::Arc<Receiver<CommandResponse>>,
}

impl AudioEngineHandle {
    pub fn set_playing(&self, val: bool) {
        self.sender.send(Command::SetPlaying(val)).unwrap();
    }

    pub fn set_play_time(&self, val: f64) {
        self.sender.send(Command::SetPlayTime(val)).unwrap();
    }

    pub fn start_recording(&self) {
        self.sender.send(Command::SetRecording(true)).unwrap();
    }

    pub fn stop_recording(&self) -> Option<(AudioSourceID, AudioSourceFormat)> {
        self.sender.send(Command::SetRecording(false)).unwrap();

        match self.receiver.recv().unwrap() {
            CommandResponse::SetRecording(v) => v,
        }
    }

    pub fn set_feedback(&self, val: bool) {
        self.sender.send(Command::SetFeedback(val)).unwrap();
    }

    pub fn set_arrangement_index(&self, index: ArrangementAudioSourceIndex) {
        self.sender.send(Command::SetArrangementAudioSourceIndex(index)).unwrap();
    }
}

#[derive(Clone, Debug, druid::Data, PartialEq)]
pub struct AudioSourceFormat {
    pub sample_rate: u32,
    pub len_frames: u32,
    pub channels: u32,
    pub beats_per_second: f64,
}

pub trait AudioSource {
    fn get_sample(&self, frame: u32, channel: u32, beats_per_second: f64) -> Option<f32>;
    fn length_secs(&self) -> f64;
    fn format(&self) -> AudioSourceFormat;
}

pub struct AudioClip {
    sample_rate: u32,
    len_frames: u32,
    channels: u32,
    samples: Vec<f32>,
    beats_per_second: f64,
}

impl AudioClip {
    pub fn new(sample_rate: u32, channels: u32, samples: Vec<f32>, beats_per_second: f64) -> Self {
        Self {
            sample_rate,
            len_frames: samples.len() as u32 / channels,
            channels,
            samples,
            beats_per_second,
        }
    }

    pub fn empty(sample_rate: u32, channels: u32, beats_per_second: f64) -> Self {
        Self {
            sample_rate,
            channels,
            len_frames: 0,
            samples: Vec::new(),
            beats_per_second,
        }
    }

    pub fn append_sample(&mut self, sample: f32) {
        self.samples.push(sample);
        self.len_frames = self.samples.len() as u32 / self.channels;
    }

    pub fn clean(&mut self) {
        self.samples.truncate(self.samples.len() - self.samples.len() % self.channels as usize);
    }
}

impl AudioSource for AudioClip {
    fn get_sample(&self, frame: u32, channel: u32, beats_per_second: f64) -> Option<f32> {
        self.samples.get(((frame as f64 * self.channels as f64 + channel as f64) * self.beats_per_second / beats_per_second).round() as usize).map(|x| *x)
    }

    fn length_secs(&self) -> f64 {
        self.len_frames as f64 / self.sample_rate as f64
    }

    fn format(&self) -> AudioSourceFormat {
        AudioSourceFormat {
            sample_rate: self.sample_rate,
            len_frames: self.len_frames,
            channels: self.channels,
            beats_per_second: self.beats_per_second,
        }
    }
}

pub struct AudioEngine {
    receiver: Receiver<Command>,
    sender: Sender<CommandResponse>,
    event_sink: druid::ExtEventSink,
    volume: f64,
    beats_per_second: f64,
    feedback: bool,
    sources: HashMap<AudioSourceID, Box<dyn AudioSource + Send + Sync>>,
    next_audio_id: AudioSourceID,
}

impl AudioEngine {
    pub fn new(event_sink: druid::ExtEventSink) -> (Self, AudioEngineHandle) {
        let (h_sender, e_receiver) = channel();
        let (e_sender, h_receiver) = channel();

        (
            Self {
                event_sink,
                volume: 0.5,
                feedback: true,
                beats_per_second: 120.0 / 60.0,
                receiver: e_receiver,
                sender: e_sender,
                sources: HashMap::new(),
                next_audio_id: AudioSourceID(0),
            },
            AudioEngineHandle {
                sender: std::sync::Arc::new(h_sender),
                receiver: std::sync::Arc::new(h_receiver),
            },
        )
    }

    pub fn run(mut self) {
        std::thread::spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let host = cpal::default_host();

            let input_device = host
                .default_input_device()
                .expect("failed to get input device");
            let output_device = host
                .default_output_device()
                .expect("failed to get output device");

            info!(
                "Using default input device: {}, {:?}",
                input_device.name()?,
                input_device.default_input_config()?
            );
            info!(
                "Using default output device: {}, {:?}",
                output_device.name()?,
                output_device.default_output_config()?
            );

            let config: cpal::StreamConfig = input_device.default_input_config()?.into();

            const LATENCY_MS: f32 = 20.0;

            let sample_rate = config.sample_rate.0;
            let channels = config.channels as u32;
            let latency_frames = (LATENCY_MS / 1000.0) * sample_rate as f32;
            let latency_samples = latency_frames as usize * channels as usize;

            let ring = ringbuf::RingBuffer::new(latency_samples * 2);
            let (mut producer, mut consumer) = ring.split();

            for _ in 0..latency_samples {
                producer.push(0.0).unwrap();
            }

            let mut channel = 0;
            let mut play_sample: u32 = 0;
            let mut play_frame: u32 = 0;
            let mut playing = false;
            let mut volume = self.volume;
            let mut recording_clip: Option<AudioClip> = None;
            let mut arrangement_index = ArrangementAudioSourceIndex::default();

            let input_stream = input_device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for sample in data {
                        if let Err(_e) = producer.push(*sample) {
                            //error!("output stream fell behind '{}', increase latency", e);
                        }
                    }
                },
                |err| {
                    error!("{}", err);
                },
            )?;

            let output_stream = output_device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for sample in data {
                        if let Ok(cmd) = self.receiver.try_recv() {
                            match cmd {
                                Command::SetPlaying(val) => playing = val,
                                Command::SetRecording(val) => {
                                    if val {
                                        recording_clip =
                                            Some(AudioClip::empty(sample_rate, channels, self.beats_per_second));
                                    } else {
                                        if let Some(mut recording_clip) =
                                            std::mem::replace(&mut recording_clip, None)
                                        {
                                            recording_clip.clean();

                                            let id = self.next_audio_id;
                                            self.next_audio_id.0 += 1;

                                            let format = recording_clip.format();

                                            self.sources.insert(id, Box::new(recording_clip));

                                            self.sender
                                                .send(CommandResponse::SetRecording(Some((
                                                    id, format,
                                                ))))
                                                .unwrap();
                                        } else {
                                            self.sender
                                                .send(CommandResponse::SetRecording(None))
                                                .unwrap();
                                        }
                                    }
                                }
                                Command::SetPlayTime(time) => {
                                    play_sample =
                                        (time * sample_rate as f64 * channels as f64) as u32;
                                }
                                Command::SetBeatsPerSecond(bps) => self.beats_per_second = bps,
                                Command::SetFeedback(feedback) => self.feedback = feedback,
                                Command::SetArrangementAudioSourceIndex(index) => arrangement_index = index,
                            }
                        }

                        match consumer.pop() {
                            Some(s) => {
                                if self.feedback {
                                    *sample = s * volume as f32;
                                } else {
                                    *sample = 0.0;
                                }
                            }
                            None => (), //error!("input stream fell behind, increase latency"),
                        }

                        channel += 1;
                        channel = channel % channels;

                        if let Some(recording_clip) = &mut recording_clip {
                            if channel % channels == 0 || recording_clip.samples.len() > 0 {
                                recording_clip.append_sample(*sample);
                            }
                        }

                        if playing {
                            play_sample += 1;
                            play_frame = play_sample / channels;

                            let beat = (play_frame as f64 / sample_rate as f64 * self.beats_per_second).floor() as u32;
                            let beat_frame = play_frame % (sample_rate as f64 / self.beats_per_second).floor() as u32;

                            if let Some(source_indices) = arrangement_index.beats.get(&(beat as usize)) {
                                for source_index in source_indices {
                                    let offset = (source_index.beats_offset as f64 * sample_rate as f64 / self.beats_per_second) as u32;

                                    if let Some(source_sample) = self.sources[&source_index.audio_source_id].get_sample(beat_frame + offset, channel, self.beats_per_second) {
                                        *sample += source_sample;
                                    }
                                }
                            }

                            if play_frame % (sample_rate / 30) == 0 {
                                self.event_sink
                                    .submit_command(
                                        ARRANGEMENT_UPDATE_PLAY_LINE,
                                        play_frame as f64 / sample_rate as f64,
                                        Target::Global,
                                    )
                                    .unwrap();
                            }
                        }
                    }
                },
                |err| {
                    error!("{}", err);
                },
            )?;

            input_stream.play()?;
            output_stream.play()?;

            std::thread::park();

            Ok(())
        });
    }
}
