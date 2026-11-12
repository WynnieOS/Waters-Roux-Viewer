//! User-initiated high-quality MJPEG/AVI video with optional in-world audio.
//!
//! The resident can explicitly opt into recording the viewer's OWN 48 kHz
//! stereo mix, including incoming WebRTC voice, sounds, parcel music and media.
//! It does not monitor other desktop applications or record the local microphone.
//! Both video and audio are captured to one indexed AVI. Get consent before
//! recording conversations. Encoding remains an experimental release blocker
//! until real in-world playback tests pass.

use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
    time::{Duration, Instant},
};

const LIMIT_SECONDS: u64 = 300;
const SAMPLE_RATE: u32 = 48_000;
const AUDIO_CHANNELS: u16 = 2;
const AUDIO_BITS: u16 = 16;
const AUDIO_FRAME_BYTES: u32 = AUDIO_CHANNELS as u32 * AUDIO_BITS as u32 / 8;

/// Configured from the Machinima Bar, persisted independently of PhotoTools.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Options {
    pub height: u32,
    pub fps: u32,
    pub jpeg_quality: u8,
    pub capture_world_audio: bool,
    /// Include the viewer's UI; otherwise capture only the rendered scene.
    pub capture_interface: bool,
    /// Include HUD attachments in the captured world view.
    pub capture_huds: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            height: 1080,
            fps: 30,
            jpeg_quality: 95,
            capture_world_audio: false,
            capture_interface: false,
            capture_huds: true,
        }
    }
}

impl Options {
    pub fn sanitize(&mut self) {
        self.height = match self.height {
            0 | 480 | 720 | 1080 | 1440 | 2160 => self.height,
            _ => 1080,
        };
        self.fps = match self.fps {
            24 | 30 | 60 => self.fps,
            _ => 30,
        };
        self.jpeg_quality = self.jpeg_quality.clamp(80, 100);
    }
}

pub const fn recording_label(options: Options) -> &'static str {
    match options.height {
        0 => "Native viewport",
        480 => "SD 480p",
        720 => "HD 720p",
        1440 => "QHD 1440p",
        2160 => "UHD 2160p",
        _ => "Full HD 1080p",
    }
}

enum Packet {
    Frame(u32, u32, Vec<u8>, Duration),
    Stop(Duration),
}

struct Session {
    sender: SyncSender<Packet>,
    worker: thread::JoinHandle<Result<(PathBuf, u64), String>>,
    output: PathBuf,
    options: Options,
    started: Instant,
    next_frame: Instant,
    requested: Option<Instant>,
    sent: u64,
    dropped: u64,
}

#[derive(Default)]
pub struct Recorder {
    session: Option<Session>,
    pub message: Option<String>,
}

impl Recorder {
    pub fn active(&self) -> bool {
        self.session.is_some()
    }

    /// A recorder-exclusive GPU readback is outstanding.
    pub fn capture_interface(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.options.capture_interface)
    }

    pub fn pending(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.requested.is_some())
    }

    pub fn elapsed(&self) -> Duration {
        self.session.as_ref().map_or(Duration::ZERO, |s| s.started.elapsed())
    }

    pub fn start(&mut self, mut options: Options, audio_engine: Option<&aurora_audio::AudioEngine>) -> Result<PathBuf, String> {
        options.sanitize();
        if self.active() {
            return Err("A recording is already running".into());
        }
        let root = directories::UserDirs::new()
            .and_then(|u| u.video_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| crate::settings::config_dir().join("videos"))
            .join("Waters Roux");
        fs::create_dir_all(&root).map_err(|e| format!("Cannot create video folder: {e}"))?;
        let base = crate::snapshot::local_stamp();
        let output = (0..1000u32)
            .map(|i| root.join(format!("Waters-Roux-{base}-{i:03}.avi")))
            .find(|p| !p.exists())
            .ok_or_else(|| "Could not choose an unused recording filename".to_owned())?;
        let (sender, rx) = mpsc::sync_channel(3);
        let path = output.clone();
        // A second, separate audio receiver: the microphone's echo-reference
        // samples are NEVER consumed by the recorder.
        let audio = if options.capture_world_audio {
            audio_engine.map(|engine| engine.recording_tap())
        } else {
            None
        };
        let has_audio = audio.is_some();
        let worker = thread::Builder::new()
            .name("waters-roux-video-encoder".into())
            .spawn(move || encode_stream(rx, path, options, audio))
            .map_err(|e| format!("Unable to start video encoder: {e}"))?;
        let now = Instant::now();
        self.session = Some(Session {
            sender,
            worker,
            output: output.clone(),
            options,
            started: now,
            next_frame: now,
            requested: None,
            sent: 0,
            dropped: 0,
        });
        self.message = Some(format!(
            "Recording started: {} ({} • {} fps • world audio {})",
            output.display(),
            recording_label(options),
            options.fps,
            if has_audio { "ON" } else { "OFF" },
        ));
        Ok(output)
    }

    /// Only ask the renderer for a frame after all higher priority still
    /// captures have cleared. Keep the GPU readback from multiplying.
    pub fn needs_frame(&mut self, now: Instant) -> bool {
        let Some(s) = self.session.as_mut() else { return false };
        if s.started.elapsed().as_secs() >= LIMIT_SECONDS {
            self.message = Some("Video reached 10-minute safety limit. Stop to save it.".into());
            return false;
        }
        // Recover from an interrupted GPU frame rather than hanging forever.
        if s.requested.is_some_and(|at| now.duration_since(at) > Duration::from_secs(3)) {
            s.requested = None;
            s.dropped += 1;
        }
        s.requested.is_none() && now >= s.next_frame
    }

    pub fn mark_requested(&mut self, now: Instant) {
        if let Some(s) = self.session.as_mut() {
            s.requested = Some(now);
        }
    }

    pub fn accept(&mut self, width: u32, height: u32, rgba: Vec<u8>) {
        let Some(s) = self.session.as_mut() else { return };
        // Timestamp the render request rather than the delayed GPU readback.
        // The AVI encoder uses these wall-clock timestamps to fill missed frame
        // slots; a 15-fps capture must not play back at double speed in 30-fps AVI.
        let captured_at = s.requested.take().unwrap_or_else(Instant::now);
        let elapsed = captured_at.saturating_duration_since(s.started);
        let now = Instant::now();
        let period = Duration::from_secs_f64(1.0 / f64::from(s.options.fps));
        s.next_frame += period;
        if s.next_frame < now {
            s.next_frame = now;
        }
        if width == 0 || height == 0 || rgba.len() != (width as usize).saturating_mul(height as usize).saturating_mul(4) {
            s.dropped += 1;
            return;
        }
        match s.sender.try_send(Packet::Frame(width, height, rgba, elapsed)) {
            Ok(()) => s.sent += 1,
            Err(TrySendError::Full(_)) => s.dropped += 1,
            Err(TrySendError::Disconnected(_)) => {
                s.dropped += 1;
                self.message = Some("Video encoder stopped unexpectedly; stop recording to inspect the error.".into());
            }
        }
    }

    pub fn stop(&mut self) -> Result<Option<PathBuf>, String> {
        let Some(s) = self.session.take() else { return Ok(None) };
        let _ = s.sender.send(Packet::Stop(s.started.elapsed()));
        let result = s.worker.join().map_err(|_| "Video encoder thread panicked".to_owned())?;
        match result {
            Ok((path, frames)) if frames > 0 => {
                self.message = Some(format!("Saved video: {} ({frames} frames; {} dropped)", path.display(), s.dropped));
                Ok(Some(path))
            }
            Ok((_path, _)) => {
                let _ = fs::remove_file(&s.output);
                self.message = Some("No frames were captured; the empty recording was discarded.".into());
                Ok(None)
            }
            Err(error) => {
                self.message = Some(format!("Video could not be finalized: {error}"));
                Err(error)
            }
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        // Best effort on ordinary shutdown: the owned encoder closes its AVI
        // headers even when the machinima bar was not explicitly stopped.
        let _ = self.stop();
    }
}

fn encode_stream(
    rx: Receiver<Packet>,
    path: PathBuf,
    options: Options,
    audio: Option<aurora_audio::RecordingTap>,
) -> Result<(PathBuf, u64), String> {
    let mut avi: Option<Avi> = None;
    let mut previous_jpeg: Option<Vec<u8>> = None;
    loop {
        let mut stop = false;
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Packet::Frame(w, h, pixels, elapsed)) => {
                if avi.is_none() {
                    let size = dimensions(w, h, options.height);
                    avi = Some(Avi::new(&path, size.0, size.1, options.fps, audio.is_some()).map_err(|e| e.to_string())?);
                }
                let writer = avi.as_mut().expect("created on first frame");
                let target = expected_frames(elapsed, options.fps).max(1);
                if writer.video_frames >= target {
                    continue;
                }
                let rgba = image::RgbaImage::from_raw(w, h, pixels).ok_or_else(|| "Invalid frame dimensions".to_owned())?;
                let scaled = image::imageops::resize(&rgba, writer.width, writer.height, image::imageops::FilterType::Lanczos3);
                let rgb = image::RgbImage::from_fn(writer.width, writer.height, |x, y| {
                    let p = scaled.get_pixel(x, y).0;
                    image::Rgb([p[0], p[1], p[2]])
                });
                let mut encoded = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, options.jpeg_quality)
                    .encode(rgb.as_raw(), writer.width, writer.height, image::ExtendedColorType::Rgb8)
                    .map_err(|e| format!("JPEG encode: {e}"))?;
                // AVI's MJPEG track is constant-frame-rate. Fill missed slots
                // with the last valid frame, never speed up the timeline.
                let previous = previous_jpeg.as_deref().unwrap_or(&encoded);
                while writer.video_frames + 1 < target {
                    writer.push_frame(previous).map_err(|e| format!("AVI frame repeat: {e}"))?;
                }
                writer.push_frame(&encoded).map_err(|e| format!("AVI frame write: {e}"))?;
                previous_jpeg = Some(encoded);
            }
            Ok(Packet::Stop(elapsed)) => {
                if let (Some(writer), Some(previous)) = (avi.as_mut(), previous_jpeg.as_ref()) {
                    while writer.video_frames < expected_frames(elapsed, options.fps) {
                        writer.push_frame(previous).map_err(|e| format!("AVI final frame repeat: {e}"))?;
                    }
                }
                stop = true;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => stop = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        // Audio is received on its OWN queue. A slow JPEG encoder cannot
        // consume the voice chat echo-cancellation samples or fill the video
        // frame queue with hundreds of tiny audio packets per second.
        if let (Some(writer), Some(tap)) = (avi.as_mut(), audio.as_ref()) {
            for _ in 0..256 {
                let Some(pcm) = tap.try_pop() else { break };
                writer.push_audio(&pcm).map_err(|e| format!("AVI audio write: {e}"))?;
            }
        }
        if stop {
            break;
        }
    }
    let Some(avi) = avi else { return Ok((path, 0)) };
    let frames = avi.video_frames;
    avi.finish().map_err(|e| format!("AVI finalize: {e}"))?;
    Ok((path, frames))
}

/// Number of constant-frame-rate AVI slots covering the elapsed real time.
fn expected_frames(elapsed: Duration, fps: u32) -> u64 {
    let nanos = elapsed.as_nanos().min(u128::from(LIMIT_SECONDS) * 1_000_000_000);
    ((nanos * u128::from(fps) + 999_999_999) / 1_000_000_000) as u64
}

fn dimensions(w: u32, h: u32, max_height: u32) -> (u32, u32) {
    let even = |value: u32| (value / 2 * 2).max(2);
    if max_height == 0 {
        return (even(w), even(h));
    }
    let max_width = max_height * 16 / 9;
    let scale = (max_width as f64 / w.max(1) as f64)
        .min(max_height as f64 / h.max(1) as f64)
        .min(1.0);
    (even((w as f64 * scale).round() as u32), even((h as f64 * scale).round() as u32))
}

/// RIFF AVI with indexed MJPEG video plus optional 48-kHz stereo PCM.
/// Sound is the consented in-viewer mix, not an operating-system screen tap.
struct Avi {
    file: File,
    width: u32,
    height: u32,
    movi_size_at: u64,
    movi_tag_at: u64,
    frames_at: u64,
    video_length_at: u64,
    audio_length_at: Option<u64>,
    video_frames: u64,
    audio_bytes: u64,
    index: Vec<([u8; 4], u32, u32, u32)>,
}

impl Avi {
    fn new(path: &PathBuf, width: u32, height: u32, fps: u32, with_audio: bool) -> std::io::Result<Self> {
        let mut f = OpenOptions::new().create_new(true).write(true).read(true).open(path)?;
        f.write_all(b"RIFF")?;
        u32le(&mut f, 0)?;
        f.write_all(b"AVI ")?;
        f.write_all(b"LIST")?;
        u32le(&mut f, if with_audio { 292 } else { 192 })?;
        f.write_all(b"hdrl")?;
        f.write_all(b"avih")?;
        u32le(&mut f, 56)?;
        u32le(&mut f, 1_000_000 / fps.max(1))?;
        u32le(&mut f, 40_000_000)?;
        u32le(&mut f, 0)?;
        u32le(&mut f, 0x10)?;
        let frames_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        for val in [0, if with_audio { 2 } else { 1 }, width * height * 3, width, height, 0, 0, 0, 0] {
            u32le(&mut f, val)?;
        }
        f.write_all(b"LIST")?;
        u32le(&mut f, 116)?;
        f.write_all(b"strl")?;
        f.write_all(b"strh")?;
        u32le(&mut f, 56)?;
        f.write_all(b"vidsMJPG")?;
        u32le(&mut f, 0)?;
        f.write_all(&0u16.to_le_bytes())?;
        f.write_all(&0u16.to_le_bytes())?;
        u32le(&mut f, 0)?;
        u32le(&mut f, 1)?;
        u32le(&mut f, fps)?;
        u32le(&mut f, 0)?;
        let video_length_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        u32le(&mut f, width * height * 3)?;
        u32le(&mut f, u32::MAX)?;
        u32le(&mut f, 0)?;
        for v in [0u16, 0, width as u16, height as u16] {
            f.write_all(&v.to_le_bytes())?;
        }
        f.write_all(b"strf")?;
        u32le(&mut f, 40)?;
        u32le(&mut f, 40)?;
        u32le(&mut f, width)?;
        u32le(&mut f, height)?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&24u16.to_le_bytes())?;
        f.write_all(b"MJPG")?;
        for v in [width * height * 3, 0, 0, 0, 0] {
            u32le(&mut f, v)?;
        }
        let audio_length_at = if with_audio {
            f.write_all(b"LIST")?;
            u32le(&mut f, 92)?;
            f.write_all(b"strl")?;
            f.write_all(b"strh")?;
            u32le(&mut f, 56)?;
            f.write_all(b"auds")?;
            u32le(&mut f, 0)?;
            u32le(&mut f, 0)?;
            f.write_all(&0u16.to_le_bytes())?;
            f.write_all(&0u16.to_le_bytes())?;
            u32le(&mut f, 0)?;
            u32le(&mut f, AUDIO_FRAME_BYTES)?;
            u32le(&mut f, SAMPLE_RATE * AUDIO_FRAME_BYTES)?;
            u32le(&mut f, 0)?;
            let at = f.stream_position()?;
            u32le(&mut f, 0)?;
            u32le(&mut f, SAMPLE_RATE / 10 * AUDIO_FRAME_BYTES)?;
            u32le(&mut f, u32::MAX)?;
            u32le(&mut f, AUDIO_FRAME_BYTES)?;
            for _ in 0..4 {
                f.write_all(&0u16.to_le_bytes())?;
            }
            f.write_all(b"strf")?;
            u32le(&mut f, 16)?;
            f.write_all(&1u16.to_le_bytes())?;
            f.write_all(&AUDIO_CHANNELS.to_le_bytes())?;
            u32le(&mut f, SAMPLE_RATE)?;
            u32le(&mut f, SAMPLE_RATE * AUDIO_FRAME_BYTES)?;
            f.write_all(&(AUDIO_FRAME_BYTES as u16).to_le_bytes())?;
            f.write_all(&AUDIO_BITS.to_le_bytes())?;
            Some(at)
        } else {
            None
        };
        f.write_all(b"LIST")?;
        let movi_size_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        let movi_tag_at = f.stream_position()?;
        f.write_all(b"movi")?;
        Ok(Self {
            file: f,
            width,
            height,
            movi_size_at,
            movi_tag_at,
            frames_at,
            video_length_at,
            audio_length_at,
            video_frames: 0,
            audio_bytes: 0,
            index: Vec::new(),
        })
    }

    fn chunk(&mut self, kind: [u8; 4], flags: u32, bytes: &[u8]) -> std::io::Result<()> {
        let pos = self.file.stream_position()?;
        let offset = pos - self.movi_tag_at;
        // Keep room for the final index, and below AVI 1.0's 4 GiB cap.
        if offset > 3_700_000_000 || self.index.len() >= 1_000_000 {
            return Err(std::io::Error::other("AVI file limit reached; stop and start a new recording"));
        }
        let len = bytes.len() as u32;
        self.file.write_all(&kind)?;
        u32le(&mut self.file, len)?;
        self.file.write_all(bytes)?;
        if len & 1 != 0 {
            self.file.write_all(&[0])?;
        }
        self.index.push((kind, flags, offset as u32, len));
        Ok(())
    }

    fn push_frame(&mut self, encoded: &[u8]) -> std::io::Result<()> {
        self.chunk(*b"00dc", 0x10, encoded)?;
        self.video_frames += 1;
        Ok(())
    }

    fn push_audio(&mut self, stereo: &[f32; 960]) -> std::io::Result<()> {
        if self.audio_length_at.is_none() {
            return Ok(());
        }
        let mut pcm = Vec::with_capacity(stereo.len() * 2);
        for sample in stereo {
            let quantized = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            pcm.extend_from_slice(&quantized.to_le_bytes());
        }
        self.chunk(*b"01wb", 0, &pcm)?;
        self.audio_bytes += pcm.len() as u64;
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        let end_movi = self.file.stream_position()?;
        self.file.write_all(b"idx1")?;
        u32le(&mut self.file, (self.index.len() as u32) * 16)?;
        for &(kind, flags, offset, size) in &self.index {
            self.file.write_all(&kind)?;
            u32le(&mut self.file, flags)?;
            u32le(&mut self.file, offset)?;
            u32le(&mut self.file, size)?;
        }
        let end = self.file.stream_position()?;
        if end > u32::MAX as u64 {
            return Err(std::io::Error::other("AVI file is too large"));
        }
        self.file.seek(SeekFrom::Start(4))?;
        u32le(&mut self.file, (end - 8) as u32)?;
        self.file.seek(SeekFrom::Start(self.movi_size_at))?;
        u32le(&mut self.file, (end_movi - self.movi_tag_at) as u32)?;
        self.file.seek(SeekFrom::Start(self.video_length_at))?;
        u32le(&mut self.file, self.video_frames as u32)?;
        self.file.seek(SeekFrom::Start(self.frames_at))?;
        u32le(&mut self.file, self.video_frames as u32)?;
        if let Some(at) = self.audio_length_at {
            self.file.seek(SeekFrom::Start(at))?;
            u32le(&mut self.file, (self.audio_bytes / u64::from(AUDIO_FRAME_BYTES)) as u32)?;
        }
        self.file.flush()?;
        self.file.sync_all()
    }
}

fn u32le(f: &mut File, value: u32) -> std::io::Result<()> {
    f.write_all(&value.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_dimensions_remain_even_and_within_budget() {
        assert_eq!(dimensions(1920, 1080, 1080), (1920, 1080));
        assert_eq!(dimensions(1920, 1200, 0), (1920, 1200));
        assert_eq!(dimensions(960, 540, 480), (852, 480));
        assert_eq!(dimensions(800, 600, 1080), (800, 600));
        assert_eq!(dimensions(3840, 2160, 1440), (2560, 1440));
        assert_eq!(dimensions(3840, 2160, 2160), (3840, 2160));
    }
    #[test]
    fn capture_timeline_preserves_duration_at_low_frame_rate() {
        assert_eq!(expected_frames(Duration::from_secs(2), 30), 60);
        assert_eq!(expected_frames(Duration::from_millis(1500), 30), 45);
        assert_eq!(expected_frames(Duration::ZERO, 30), 0);
        assert_eq!(expected_frames(Duration::from_secs(360), 30), 9000);
    }
    #[test]
    fn slow_capture_keeps_the_original_recording_duration() {
        let path = std::env::temp_dir().join(format!("waters-roux-cfr-test-{}.avi", uuid::Uuid::new_v4()));
        let (tx, rx) = mpsc::sync_channel(4);
        let pixels = vec![255; 4 * 4 * 4];
        tx.send(Packet::Frame(4, 4, pixels.clone(), Duration::ZERO)).expect("first frame");
        tx.send(Packet::Frame(4, 4, pixels.clone(), Duration::from_millis(500)))
            .expect("slow frame");
        tx.send(Packet::Frame(4, 4, pixels, Duration::from_millis(900)))
            .expect("last frame");
        tx.send(Packet::Stop(Duration::from_secs(1))).expect("stop");
        drop(tx);
        let options = Options {
            height: 0,
            fps: 30,
            jpeg_quality: 90,
            ..Options::default()
        };
        let (_, frames) = encode_stream(rx, path.clone(), options, None).expect("encode");
        assert_eq!(frames, 30, "one second of wall time must not become a tenth-second movie");
        let file = fs::read(&path).expect("AVI exists");
        assert!(file.starts_with(b"RIFF"));
        assert!(file.windows(4).any(|b| b == b"idx1"));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn avi_container_writes_index_and_frame_counts() {
        let path = std::env::temp_dir().join(format!("waters-roux-test-{}.avi", uuid::Uuid::new_v4()));
        let mut w = Avi::new(&path, 4, 4, 30, true).expect("AVI create");
        w.push_audio(&[0.25; 960]).expect("audio chunk");
        w.push_frame(&[0xff, 0xd8, 0xff, 0xd9]).expect("frame");
        w.finish().expect("finalize");
        let data = fs::read(&path).expect("read AVI");
        assert!(data.starts_with(b"RIFF"));
        assert!(data.windows(4).any(|b| b == b"movi"));
        assert!(data.windows(4).any(|b| b == b"auds"));
        assert!(data.windows(4).any(|b| b == b"01wb"));
        assert!(data.windows(4).any(|b| b == b"idx1"));
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize, data.len() - 8);
        fs::remove_file(path).expect("cleanup");
    }
}
