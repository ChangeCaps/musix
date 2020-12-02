use crate::{arrangement::*, audio_clip::AudioClip, audio_source::*, commands::*};
use cpal::traits::*;
use druid::Target;
use log::*;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, druid::Data)]
pub struct AudioSourceID(pub usize);

pub enum Command {
    LogHistory,
    RevertHistory(crate::deligate::HistoryID),
    SetPlaying(bool),
    SetRecording(bool),
    SetPlayTime(f64),
    SetFeedback(bool),
    SetBeatsPerSecond(f64),
    SetVolume(f64),
    SetMetronome(bool),
    RemoveAudioSource(AudioSourceID),
    GetAudioSourceClone(AudioSourceID),
    DownloadAudioSources,
    SetArrangementAudioSourceIndex(ArrangementAudioSourceIndex),
}

pub enum CommandResponse {
    SetRecording(Option<(AudioSourceID, AudioSourceFormat)>),
    DownloadAudioSources(Arc<HashMap<AudioSourceID, AudioSource>>),
    GetAudioSourceClone(AudioSource),
}

#[derive(Clone, druid::Data)]
pub struct AudioEngineHandle {
    sender: std::sync::Arc<Sender<Command>>,
    receiver: std::sync::Arc<Receiver<CommandResponse>>,
}

impl AudioEngineHandle {
    pub fn log_history(&self) {
        self.sender.send(Command::LogHistory).unwrap();
    }

    pub fn revert_history(&self, history_id: crate::deligate::HistoryID) {
        self.sender
            .send(Command::RevertHistory(history_id))
            .unwrap();
    }

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
            _ => panic!("wrong response wtf"),
        }
    }

    pub fn set_feedback(&self, val: bool) {
        self.sender.send(Command::SetFeedback(val)).unwrap();
    }

    pub fn set_volume(&self, volume: f64) {
        self.sender.send(Command::SetVolume(volume)).unwrap();
    }

    pub fn set_beats_per_second(&self, beats_per_second: f64) {
        self.sender
            .send(Command::SetBeatsPerSecond(beats_per_second))
            .unwrap();
    }

    pub fn set_metronome(&self, metronome: bool) {
        self.sender.send(Command::SetMetronome(metronome)).unwrap();
    }

    pub fn remove_audio_source(&self, audio_source_id: AudioSourceID) {
        self.sender
            .send(Command::RemoveAudioSource(audio_source_id))
            .unwrap();
    }

    pub fn download_audio_sources(&self) -> Arc<HashMap<AudioSourceID, AudioSource>> {
        self.sender.send(Command::DownloadAudioSources).unwrap();

        match self.receiver.recv().unwrap() {
            CommandResponse::DownloadAudioSources(v) => v,
            _ => panic!("fucking wrong respose, how does this even happen, like it's probably a deta-race kinda thing, but like, you shouldn't see this error message")
        }
    }

    pub fn get_audio_source_clone(&self, audio_source_id: AudioSourceID) -> AudioSource {
        self.sender
            .send(Command::GetAudioSourceClone(audio_source_id))
            .unwrap();

        match self.receiver.recv().unwrap() {
            CommandResponse::GetAudioSourceClone(v) => v,
            _ => panic!("wrong response wtf"),
        }
    }

    pub fn set_arrangement_index(&self, index: ArrangementAudioSourceIndex) {
        self.sender
            .send(Command::SetArrangementAudioSourceIndex(index))
            .unwrap();
    }
}

pub struct AudioEngine {
    receiver: Receiver<Command>,
    sender: Sender<CommandResponse>,
    event_sink: druid::ExtEventSink,
    volume: f64,
    beats_per_second: f64,
    feedback: bool,
    sources: Arc<HashMap<AudioSourceID, AudioSource>>,
    next_audio_id: AudioSourceID,
    history: crate::deligate::History<AudioEngineHistory>,
}

#[derive(Clone, druid::Data)]
pub struct AudioEngineHistory {
    beats_per_second: f64,
    sources: Arc<HashMap<AudioSourceID, AudioSource>>,
    next_audio_id: AudioSourceID,
}

impl AudioEngineHistory {
    pub fn from_audio_engine(audio_engine: &AudioEngine) -> Self {
        Self {
            beats_per_second: audio_engine.beats_per_second.clone(),
            sources: audio_engine.sources.clone(),
            next_audio_id: audio_engine.next_audio_id.clone(),
        }
    }
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
                sources: Arc::new(HashMap::new()),
                next_audio_id: AudioSourceID(0),
                history: crate::deligate::History::new(),
            },
            AudioEngineHandle {
                sender: std::sync::Arc::new(h_sender),
                receiver: std::sync::Arc::new(h_receiver),
            },
        )
    }

    pub fn revert(&mut self, history: AudioEngineHistory) {
        self.beats_per_second = history.beats_per_second;
        self.sources = history.sources;
        self.next_audio_id = history.next_audio_id;
    }

    pub fn run(mut self) {
        self.history
            .update_current_data(&AudioEngineHistory::from_audio_engine(&self));

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

            let config: cpal::StreamConfig = output_device.default_output_config()?.into();

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

            let mut noise_level: f32 = 0.025;
            let mut noise_sample = 0;
            let mut channel = 0;
            let mut play_sample: u32 = 0;
            let mut play_frame: u32 = 0;
            let mut metronome = true;
            let mut wait_for_input = true;
            let mut waiting_for_input = false;
            let mut playing = false;
            let mut recording = false;
            let mut recording_clip: Option<AudioClip> = None;
            let mut arrangement_index = ArrangementAudioSourceIndex::default();

            let input_stream = input_device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for sample in data {
                        if let Err(e) = producer.push(*sample) {
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
                                Command::LogHistory => {
                                    if self
                                        .history
                                        .log_history(&AudioEngineHistory::from_audio_engine(&self))
                                        .is_some()
                                    {
                                        log::info!("Logged audio engine history");
                                    }
                                }
                                Command::RevertHistory(history_id) => {
                                    if let Some(state) = self.history.revert_to(history_id) {
                                        self.revert(state);
                                        log::info!("Reverted audio engine history");
                                    }
                                }
                                Command::SetPlaying(val) => {
                                    playing = val;
                                    recording &= val;
                                }
                                Command::SetRecording(val) => {
                                    if val {
                                        recording_clip =
                                            Some(AudioClip::empty(AudioSourceFormat {
                                                sample_rate,
                                                channels,
                                                len_frames: 0,
                                                beats_per_second: self.beats_per_second,
                                            }));

                                        if wait_for_input {
                                            waiting_for_input = true;
                                        }

                                        playing = true;
                                        recording = true;
                                    } else {
                                        recording = false;

                                        if let Some(mut recording_clip) =
                                            std::mem::replace(&mut recording_clip, None)
                                        {
                                            recording_clip.clean();

                                            let id = self.next_audio_id;
                                            self.next_audio_id.0 += 1;

                                            let format = recording_clip.format();

                                            Arc::make_mut(&mut self.sources).insert(
                                                id,
                                                AudioSource::AudioClip(Arc::new(recording_clip)),
                                            );

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
                                Command::SetVolume(volume) => self.volume = volume,
                                Command::SetMetronome(m) => metronome = m,
                                Command::RemoveAudioSource(audio_source_id) => {
                                    Arc::make_mut(&mut self.sources).remove(&audio_source_id);
                                }
                                Command::DownloadAudioSources => self.sender.send(CommandResponse::DownloadAudioSources(self.sources.clone())).unwrap(),
                                Command::GetAudioSourceClone(audio_source_id) => {
                                    self.sender
                                        .send(CommandResponse::GetAudioSourceClone(
                                            self.sources[&audio_source_id].clone().clone(),
                                        ))
                                        .unwrap();
                                }
                                Command::SetArrangementAudioSourceIndex(index) => {
                                    arrangement_index = index
                                }
                            }
                        }

                        match consumer.pop() {
                            Some(s) => {
                                if self.feedback {
                                    *sample = s * self.volume as f32;
                                } else {
                                    *sample = 0.0;
                                }
                            }
                            None => (), //error!("input stream fell behind, increase latency"),
                        }

                        channel += 1;
                        channel = channel % channels;

                        if noise_sample > 0 {
                            noise_sample -= 1;
                            noise_level = noise_level.max(*sample);

                            if noise_sample == 0 {
                                info!("recorded noise level: {}", noise_level);
                            }
                        }

                        if let Some(recording_clip) = &mut recording_clip {
                            if (channel % channels == 0
                                && (sample.abs() > noise_level * 1.2 || !waiting_for_input))
                                || recording_clip.len_samples() > 0
                            {
                                recording_clip.append_sample(*sample);
                            }
                        }

                        if playing {
                            if recording
                                && metronome
                                && (play_frame as f64 / sample_rate as f64)
                                    % (1.0 / self.beats_per_second)
                                    < 0.01
                            {
                                *sample += 0.3;
                            }

                            play_sample += 1;

                            play_frame = play_sample / channels;

                            let beat = (play_frame as f64 / sample_rate as f64
                                * self.beats_per_second)
                                .floor() as u32;
                            let beat_frame = play_frame
                                % (sample_rate as f64 / self.beats_per_second).floor() as u32;

                            if let Some(source_indices) =
                                arrangement_index.beats.get(&(beat as usize))
                            {
                                for source_index in source_indices {
                                    let offset = (source_index.beats_offset as f64
                                        * sample_rate as f64
                                        / self.beats_per_second)
                                        as i64;

                                    if beat_frame as i64 + offset < 0 {
                                        continue;
                                    }

                                    if let Some(source_sample) =
                                        self.sources[&source_index.audio_source_id].get_sample(
                                            beat_frame + offset as u32,
                                            channel,
                                            self.beats_per_second,
                                        )
                                    {
                                        *sample += source_sample;
                                    }
                                }
                            }

                            if play_frame % (sample_rate / 30) == 0 {
                                self.event_sink
                                    .submit_command(
                                        ARRANGEMENT_UPDATE_PLAY_LINE,
                                        play_frame as f64 / sample_rate as f64,
                                        Target::Widget(crate::ARRANGEMENT_WIDGET_ID),
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
