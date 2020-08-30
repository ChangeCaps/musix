use crate::commands::*;
use cpal::traits::*;
use druid::Target;
use log::*;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, Hash, PartialEq, Debug, druid::Data)]
pub struct AudioID(pub usize);

pub enum Command {
    SetPlaying(bool),
    SetPlayTime(f64),
}

#[derive(Clone, druid::Data)]
pub struct AudioEngineHandle {
    sender: std::sync::Arc<Sender<Command>>,
}

impl AudioEngineHandle {
    pub fn set_playing(&self, val: bool) {
        self.sender.send(Command::SetPlaying(val)).unwrap();
    }

    pub fn set_play_time(&self, val: f64) {
        self.sender.send(Command::SetPlayTime(val)).unwrap();
    }
}

pub struct AudioEngine {
    receiver: Receiver<Command>,
    event_sink: druid::ExtEventSink,
    volume: f64,
}

impl AudioEngine {
    pub fn new(event_sink: druid::ExtEventSink) -> (Self, AudioEngineHandle) {
        let (sender, receiver) = channel();

        (
            Self {
                event_sink,
                volume: 2.0,
                receiver,
            },
            AudioEngineHandle {
                sender: std::sync::Arc::new(sender),
            },
        )
    }

    pub fn run(self) {
        std::thread::spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let host = cpal::default_host();

            let input_device = host
                .default_input_device()
                .expect("failed to get input device");
            let output_device = host
                .default_output_device()
                .expect("failed to get output device");

            info!("Using default input device: {}, {:?}", input_device.name()?, input_device.default_input_config()?);
            info!("Using default output device: {}, {:?}", output_device.name()?, output_device.default_output_config()?);

            let config: cpal::StreamConfig = input_device.default_input_config()?.into();

            const LATENCY_MS: f32 = 40.0;

            let sample_rate = config.sample_rate.0;
            let latency_frames = (LATENCY_MS / 1000.0) * sample_rate as f32;
            let latency_samples = latency_frames as usize * config.channels as usize;

            let ring = ringbuf::RingBuffer::new(latency_samples * 2);
            let (mut producer, mut consumer) = ring.split();

            for _ in 0..latency_samples {
                producer.push(0.0).unwrap();
            }

            let mut play_time: u32 = 0;
            let mut playing = false;
            let mut volume = self.volume;

            let input_stream = input_device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for sample in data {
                        if let Ok(cmd) = self.receiver.try_recv() {
                            match cmd {
                                Command::SetPlaying(val) => playing = val,
                                Command::SetPlayTime(time) => {
                                    play_time = (time * sample_rate as f64) as u32
                                }
                            }
                        }

                        if let Err(e) = producer.push(*sample) {
                            error!("output stream fell behind '{}', increase latency", e);
                        }

                        if playing {
                            play_time += 1;

                            if play_time % (sample_rate / 30) == 0 {
                                self.event_sink
                                    .submit_command(
                                        ARRANGEMENT_UPDATE_PLAY_LINE,
                                        play_time as f64 / sample_rate as f64,
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

            let output_stream = output_device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for sample in data {
                        match consumer.pop() {
                            Some(s) => *sample = s * volume as f32,
                            None => error!("input stream fell behind, increase latency"),

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
