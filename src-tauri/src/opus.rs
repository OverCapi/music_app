//! Ogg Opus playback: symphonia demuxes the Ogg container and libopus decodes.
//! rodio's built-in symphonia setup has no Opus codec and no way to register one,
//! so this is a small rodio `Source` of its own.

use std::fs::File;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder as _, DecoderOptions, CODEC_TYPE_OPUS};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use symphonia_adapter_libopus::OpusDecoder;

/// libopus always decodes at 48 kHz.
const OPUS_SAMPLE_RATE: u32 = 48_000;

pub struct OpusSource {
    format: Box<dyn FormatReader>,
    decoder: OpusDecoder,
    track_id: u32,
    channels: ChannelCount,
    sample_rate: SampleRate,
    total_duration: Option<Duration>,
    /// Interleaved samples of the last decoded packet.
    buffer: Vec<Sample>,
    pos: usize,
}

impl OpusSource {
    pub fn open(file: File) -> Result<Self, String> {
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("opus");
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("conteneur Ogg invalide ({e})"))?;
        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec == CODEC_TYPE_OPUS)
            .ok_or("aucune piste Opus dans le fichier")?;
        let params = track.codec_params.clone();
        let track_id = track.id;

        let decoder = OpusDecoder::try_new(&params, &DecoderOptions::default())
            .map_err(|e| format!("décodeur Opus indisponible ({e})"))?;

        let channels = params
            .channels
            .and_then(|c| u16::try_from(c.count()).ok())
            .and_then(NonZero::new)
            .ok_or("nombre de canaux inconnu")?;
        let sample_rate = NonZero::new(params.sample_rate.unwrap_or(OPUS_SAMPLE_RATE))
            .ok_or("fréquence d'échantillonnage invalide")?;
        let total_duration = params.time_base.zip(params.n_frames).map(|(tb, frames)| {
            let t = tb.calc_time(frames);
            Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac)
        });

        Ok(Self {
            format,
            decoder,
            track_id,
            channels,
            sample_rate,
            total_duration,
            buffer: Vec::new(),
            pos: 0,
        })
    }

    /// Decodes the next packet into `buffer`. Returns false at end of stream.
    fn decode_next(&mut self) -> bool {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(_) => return false,
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(audio) => {
                    if audio.frames() == 0 {
                        continue;
                    }
                    let mut samples =
                        SampleBuffer::<f32>::new(audio.capacity() as u64, *audio.spec());
                    samples.copy_interleaved_ref(audio);
                    self.buffer.clear();
                    self.buffer
                        .extend(samples.samples().iter().map(|&s| s as Sample));
                    self.pos = 0;
                    return true;
                }
                // Corrupt packet: skip it rather than ending the track.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(_) => return false,
            }
        }
    }
}

impl Iterator for OpusSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.pos >= self.buffer.len() && !self.decode_next() {
            return None;
        }
        let sample = self.buffer[self.pos];
        self.pos += 1;
        Some(sample)
    }
}

impl Source for OpusSource {
    fn current_span_len(&self) -> Option<usize> {
        // Channels and sample rate never change within an Opus stream.
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from(pos.as_secs_f64()),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| SeekError::Other(Arc::new(e)))?;
        self.decoder.reset();
        self.buffer.clear();
        self.pos = 0;
        Ok(())
    }
}
