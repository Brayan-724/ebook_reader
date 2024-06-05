use log::{error, trace, warn};
use std::io::{self, Cursor};
use symphonia::core::audio::{RawSampleBuffer, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::AUDIO_LOG;

pub fn mp3_to_wav(hint: &Hint, a: Vec<u8>) -> Vec<u8> {
    let a = Cursor::new(a);

    let media_source = MediaSourceStream::new(Box::from(a), Default::default());

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed = symphonia::default::get_probe()
        .format(hint, media_source, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    let mut out = vec![];

    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => unimplemented!(),
            Err(Error::IoError(err)) if err.kind() == io::ErrorKind::UnexpectedEof => break,

            Err(err) => {
                error!(target: AUDIO_LOG, "{:?}", err);
                // A unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while let Some(_) = format.metadata().pop() {
            // Consume the new metadata at the head of the metadata queue.
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => match decoded {
                symphonia::core::audio::AudioBufferRef::F32(b) => {
                    let a = b.chan(0);
                    let a =
                        unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, a.len() * 4) };
                    let mut a = a.to_vec();
                    out.append(&mut a);
                }
                _ => unreachable!(),
            },
            Err(Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                warn!(target: AUDIO_LOG, "Cannot decode packet. Skipping");
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                warn!(target: AUDIO_LOG, "Cannot decode packet. Skipping");
                continue;
            }
            Err(err) => {
                // An unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        }
    }

    // trace!(target: AUDIO_LOG, "Out: {:?}", out);
    out
}
