//! H.264/MP4 recorder backend. Frames are encoded *during* capture rather than
//! first JPEG-compressed into an AVI; explicitly selected world and microphone
//! audio are mixed and muxed as AAC on stop.
//! FFmpeg is a separate executable, bundled beside the viewer by the Windows
//! packaging workflow. No executable is downloaded or installed at runtime.
//! The original AVI backend remains only for regression tests and recovery.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::Duration,
};

use super::{AUDIO_CHANNELS, Options, Packet, SAMPLE_RATE, dimensions, expected_frames};

/// FFmpeg is a console application on Windows, but Waters Roux is a GUI.
/// Without CREATE_NO_WINDOW, each probe/record/mux can flash a command prompt
/// and steal focus from the viewer. Background throttling can then freeze
/// the in-world scene that the recorder is supposed to capture.
fn ffmpeg_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Prefer the bundled, pinned build. A resident can override it explicitly
/// for development; finally allow an FFmpeg already installed on the PATH.
pub(super) fn locate_encoder() -> Result<PathBuf, String> {
    let mut paths = Vec::new();
    if let Some(override_path) = std::env::var_os("WATERS_ROUX_FFMPEG") {
        paths.push(PathBuf::from(override_path));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = exe.parent()
    {
        paths.push(root.join("tools").join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }));
        paths.push(root.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }));
    }
    paths.push(PathBuf::from("ffmpeg"));

    for candidate in paths {
        let Ok(probe) = ffmpeg_command(&candidate)
            .args(["-hide_banner", "-encoders"])
            .stdin(Stdio::null())
            .output()
        else {
            continue;
        };
        if !probe.status.success() {
            continue;
        }
        let available = String::from_utf8_lossy(&probe.stdout);
        if available.contains("libx264") && available.contains(" aac ") {
            return Ok(candidate);
        }
    }
    Err("H.264/MP4 cannot start: Waters Roux needs FFmpeg with libx264 and AAC. \
         The official Windows ZIP must include tools/ffmpeg.exe; do not use \
         an incomplete ZIP. If building from source, set WATERS_ROUX_FFMPEG \
         to an FFmpeg executable with these encoders."
        .into())
}

/// If encoding encounters an error, close and reap the child so a stale
/// FFmpeg process can never keep writing after a failed recording.
struct VideoEncoder {
    child: Child,
    width: u32,
    height: u32,
    frames: u64,
}

impl VideoEncoder {
    fn start(ffmpeg: &Path, output: &Path, width: u32, height: u32, options: Options) -> Result<Self, String> {
        let size = format!("{width}x{height}");
        let fps = options.fps.to_string();
        // The old persisted field name remains for backwards compatibility.
        // Lower x264 CRF means higher visual quality: range 16..26.
        let crf = (16 + (100 - u32::from(options.jpeg_quality)) / 2).to_string();
        let child = ffmpeg_command(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &size])
            .args(["-framerate", &fps, "-i", "pipe:0"])
            .args(["-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", &crf])
            .args(["-pix_fmt", "yuv420p", "-movflags", "+faststart", "-f", "mp4"])
            .arg(output)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot launch H.264 encoder: {e}"))?;
        Ok(Self {
            child,
            width,
            height,
            frames: 0,
        })
    }

    fn write(&mut self, rgba: &[u8]) -> Result<(), String> {
        self.child
            .stdin
            .as_mut()
            .ok_or_else(|| "H.264 encoder input closed unexpectedly".to_owned())?
            .write_all(rgba)
            .map_err(|e| format!("H.264 encoder stopped accepting frames: {e}"))?;
        self.frames += 1;
        Ok(())
    }

    fn finish(mut self) -> Result<u64, String> {
        // End-of-stream is required: the MP4 index (moov) is written on exit.
        self.child.stdin.take();
        let status = self.child.wait().map_err(|e| format!("H.264 encoder exit: {e}"))?;
        let mut details = String::new();
        if let Some(mut stderr) = self.child.stderr.take() {
            let _ = stderr.read_to_string(&mut details);
        }
        if !status.success() {
            return Err(format!(
                "H.264 encoder failed ({status}): {}",
                details.trim().chars().take(800).collect::<String>()
            ));
        }
        Ok(self.frames)
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        // A finished process is already reaped; killing it is harmless.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn write_audio_packet(out: &mut File, stereo: &[f32; 960]) -> Result<(), String> {
    let mut bytes = [0u8; 1920];
    for (i, sample) in stereo.iter().enumerate() {
        let v = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        bytes[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
    }
    out.write_all(&bytes).map_err(|e| format!("Cannot write recorder audio: {e}"))
}

/// Convert the user's own 48 kHz mono microphone into the recorder's 48 kHz
/// stereo PCM without consuming the voice client's separate input queue.
fn write_microphone_packet(out: &mut File, mono: &[f32; 480]) -> Result<(), String> {
    let mut stereo = [0.0_f32; 960];
    for (i, sample) in mono.iter().enumerate() {
        stereo[i * 2] = *sample;
        stereo[i * 2 + 1] = *sample;
    }
    write_audio_packet(out, &stereo)
}

/// Opting into world audio must not silently produce a video-only MP4 when
/// the output device has delivered no PCM blocks. One muted 10 ms frame is
/// enough for FFmpeg's existing apad filter to fill the recording timeline.
/// This never captures a microphone or unrelated desktop audio.
fn seed_empty_world_audio(pcm: Option<&mut File>, audio_bytes: &mut u64) -> Result<(), String> {
    if *audio_bytes == 0
        && let Some(file) = pcm
    {
        write_audio_packet(file, &[0.0; 960])?;
        *audio_bytes += 1920;
    }
    Ok(())
}

fn check_mp4(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot open finished MP4: {e}"))?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header).map_err(|e| format!("Truncated MP4: {e}"))?;
    if &header[4..8] != b"ftyp" {
        return Err("Encoder returned a file without a valid MP4 ftyp header".into());
    }
    if file.metadata().map_err(|e| e.to_string())?.len() < 1024 {
        return Err("Encoder returned an unexpectedly small MP4".into());
    }
    Ok(())
}

/// Use precisely the same AAC muxing path for live captures and the real
/// bundled-encoder regression fixture. Keep the original silent MP4 intact
/// until FFmpeg confirms that both output tracks were finalized.
fn mux_world_audio(ffmpeg: &Path, video_work: &Path, audio_work: &Path, mux_work: &Path) -> Result<(), String> {
    let result = ffmpeg_command(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        .arg("-i")
        .arg(video_work)
        .args(["-f", "s16le", "-ar", &SAMPLE_RATE.to_string(), "-ac", &AUDIO_CHANNELS.to_string()])
        .arg("-i")
        .arg(audio_work)
        .args(["-map", "0:v:0", "-map", "1:a:0", "-c:v", "copy"])
        .args(["-c:a", "aac", "-b:a", "192k", "-af", "aresample=async=1:first_pts=0,apad"])
        .args(["-shortest", "-movflags", "+faststart", "-f", "mp4"])
        .arg(mux_work)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Cannot mux MP4 audio track: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "MP4 audio mux failed; video retained at {}: {}",
            video_work.display(),
            String::from_utf8_lossy(&result.stderr).chars().take(800).collect::<String>()
        ));
    }
    check_mp4(mux_work)
}

/// Mix locally consented narration and the viewer's in-world sound into one
/// AAC track. Both are 48 kHz stereo sources (the mono mic is expanded when
/// written); timestamps begin at recording start and end at the video duration.
fn mux_world_and_microphone(
    ffmpeg: &Path,
    video_work: &Path,
    world_work: &Path,
    microphone_work: &Path,
    mux_work: &Path,
) -> Result<(), String> {
    let result = ffmpeg_command(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        .arg("-i")
        .arg(video_work)
        .args(["-f", "s16le", "-ar", &SAMPLE_RATE.to_string(), "-ac", &AUDIO_CHANNELS.to_string()])
        .arg("-i")
        .arg(world_work)
        .args(["-f", "s16le", "-ar", &SAMPLE_RATE.to_string(), "-ac", &AUDIO_CHANNELS.to_string()])
        .arg("-i")
        .arg(microphone_work)
        .args([
            "-filter_complex",
            "[1:a][2:a]amix=inputs=2:duration=longest:dropout_transition=0,aresample=async=1:first_pts=0,apad[mixed]",
            "-map",
            "0:v:0",
            "-map",
            "[mixed]",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ])
        .arg(mux_work)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Cannot combine microphone and world audio: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "MP4 microphone/world audio mix failed; unmixed video preserved at {}: {}",
            video_work.display(),
            String::from_utf8_lossy(&result.stderr).chars().take(800).collect::<String>()
        ));
    }
    check_mp4(mux_work)
}

pub(super) fn encode(
    rx: Receiver<Packet>,
    path: PathBuf,
    options: Options,
    audio: Option<aurora_audio::RecordingTap>,
    microphone: Option<aurora_audio::MicRecordingTap>,
    queued_frames: Option<Arc<AtomicUsize>>,
    ffmpeg: PathBuf,
) -> Result<(PathBuf, u64, u64), String> {
    let video_work = path.with_extension("recording-video.mp4");
    let audio_work = path.with_extension("recording-audio.pcm");
    let microphone_work = path.with_extension("recording-microphone.pcm");
    let mux_work = path.with_extension("recording-final.mp4");
    for work in [&video_work, &audio_work, &microphone_work, &mux_work] {
        if work.exists() {
            return Err(format!(
                "Previous unfinished recording exists: {}. Preserve it before recording again.",
                work.display()
            ));
        }
    }

    let mut encoder: Option<VideoEncoder> = None;
    let mut last_frame: Option<Vec<u8>> = None;
    let mut unique_frames = 0u64;
    let mut audio_bytes = 0u64;
    let mut microphone_bytes = 0u64;
    let mut pcm = if audio.is_some() {
        Some(File::create(&audio_work).map_err(|e| format!("Cannot prepare world audio track: {e}"))?)
    } else {
        None
    };
    let mut mic_pcm = if microphone.is_some() {
        Some(File::create(&microphone_work).map_err(|e| format!("Cannot prepare microphone track: {e}"))?)
    } else {
        None
    };

    loop {
        let mut stop = false;
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Packet::Frame(width, height, pixels, elapsed)) => {
                if let Some(backlog) = queued_frames.as_ref() {
                    backlog.fetch_sub(1, Ordering::Relaxed);
                }
                let target = expected_frames(elapsed, options.fps).max(1);
                if encoder.as_ref().is_some_and(|w| w.frames >= target) {
                    continue;
                }
                if encoder.is_none() {
                    let (w, h) = dimensions(width, height, options.height);
                    encoder = Some(VideoEncoder::start(&ffmpeg, &video_work, w, h, options)?);
                }
                let video = encoder.as_mut().expect("initialized on first frame");
                let rgba = image::RgbaImage::from_raw(width, height, pixels)
                    .ok_or_else(|| "Invalid GPU frame size for H.264 encoding".to_owned())?;
                let resized = if (width, height) == (video.width, video.height) {
                    rgba
                } else {
                    image::imageops::resize(&rgba, video.width, video.height, image::imageops::FilterType::Triangle)
                };
                let frame = resized.into_raw();
                let previous = last_frame.as_deref().unwrap_or(&frame);
                while video.frames + 1 < target {
                    video.write(previous)?;
                }
                video.write(&frame)?;
                unique_frames += 1;
                last_frame = Some(frame);
            }
            Ok(Packet::Stop(elapsed)) => {
                if let (Some(video), Some(previous)) = (encoder.as_mut(), last_frame.as_deref()) {
                    while video.frames < expected_frames(elapsed, options.fps) {
                        video.write(previous)?;
                    }
                }
                stop = true;
            }
            Err(RecvTimeoutError::Disconnected) => stop = true,
            Err(RecvTimeoutError::Timeout) => {}
        }

        // World and microphone taps have independent bounded queues. Reading
        // one cannot delay or consume the other's samples or affect WebRTC.
        if let (Some(tap), Some(file)) = (audio.as_ref(), pcm.as_mut()) {
            for _ in 0..256 {
                let Some(samples) = tap.try_pop() else { break };
                write_audio_packet(file, &samples)?;
                audio_bytes += 1920;
            }
        }
        if let (Some(tap), Some(file)) = (microphone.as_ref(), mic_pcm.as_mut()) {
            for _ in 0..256 {
                let Some(samples) = tap.try_pop() else { break };
                write_microphone_packet(file, &samples)?;
                microphone_bytes += 1920;
            }
        }
        if stop {
            break;
        }
    }

    // A quiet device must still produce the explicitly requested AAC track.
    // Reuse the real recorder's PCM->AAC path; do not silently drop to video-only.
    seed_empty_world_audio(pcm.as_mut(), &mut audio_bytes)?;
    seed_empty_world_audio(mic_pcm.as_mut(), &mut microphone_bytes)?;
    // Close temporary PCM before FFmpeg reads it. Neither PCM file is kept
    // when finalization succeeds; failed muxes preserve the source video.
    if let Some(mut file) = pcm.take() {
        file.flush().map_err(|e| format!("World audio track flush: {e}"))?;
    }
    if let Some(mut file) = mic_pcm.take() {
        file.flush().map_err(|e| format!("Microphone track flush: {e}"))?;
    }
    let Some(video) = encoder else {
        for file in [&audio_work, &microphone_work] {
            if file.exists() {
                let _ = fs::remove_file(file);
            }
        }
        return Ok((path, 0, 0));
    };
    let frames = video.finish()?;
    check_mp4(&video_work)?;

    match (audio_bytes > 0, microphone_bytes > 0) {
        (true, true) => mux_world_and_microphone(&ffmpeg, &video_work, &audio_work, &microphone_work, &mux_work)?,
        (true, false) => mux_world_audio(&ffmpeg, &video_work, &audio_work, &mux_work)?,
        (false, true) => mux_world_audio(&ffmpeg, &video_work, &microphone_work, &mux_work)?,
        (false, false) => {}
    }
    if audio_bytes > 0 || microphone_bytes > 0 {
        fs::rename(&mux_work, &path).map_err(|e| format!("Cannot save completed MP4: {e}"))?;
        let _ = fs::remove_file(&video_work);
    } else {
        fs::rename(&video_work, &path).map_err(|e| format!("Cannot save completed MP4: {e}"))?;
    }
    for work in [&audio_work, &microphone_work] {
        if work.exists() {
            let _ = fs::remove_file(work);
        }
    }
    check_mp4(&path)?;
    Ok((path, frames, unique_frames))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_microphone_pcm_is_stereo_without_stealing_world_samples() {
        let file = std::env::temp_dir().join(format!("waters-roux-mic-{}.pcm", uuid::Uuid::new_v4()));
        let mut output = File::create(&file).expect("create mic PCM");
        write_microphone_packet(&mut output, &[0.25; 480]).expect("record mic sample");
        output.flush().expect("flush mic PCM");
        drop(output);
        let bytes = fs::read(&file).expect("read mic PCM");
        assert_eq!(bytes.len(), 1920, "10 ms of 48 kHz stereo PCM");
        assert_eq!(&bytes[0..2], &bytes[2..4], "mono microphone must be duplicated to left and right");
        assert_ne!(&bytes[0..2], &[0u8; 2], "mic voice samples cannot be discarded");
        fs::remove_file(file).expect("clean mic fixture");
    }

    #[test]
    fn h264_mp4_recording_worker_captures_own_microphone_without_world_mix() {
        if std::env::var_os("WATERS_ROUX_TEST_MP4").is_none() {
            return;
        }
        let ffmpeg = locate_encoder().expect("bundled encoder");
        let ffprobe = ffmpeg.with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        let path = std::env::temp_dir().join(format!("waters-roux-mic-only-{}.mp4", uuid::Uuid::new_v4()));
        let (mut feed, mic) = aurora_audio::MicCapture::virtual_input();
        let mic_tap = mic.recording_tap();
        let samples: Vec<f32> = (0..48_000usize)
            .map(|n| 0.22 * (n as f32 * 660.0 * std::f32::consts::TAU / 48_000.0).sin())
            .collect();
        assert_eq!(feed.push(&samples), 48_000);
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let pixels = vec![128u8; 64 * 64 * 4];
        tx.send(Packet::Frame(64, 64, pixels.clone(), Duration::ZERO))
            .expect("initial frame");
        tx.send(Packet::Frame(64, 64, pixels.clone(), Duration::from_millis(500)))
            .expect("motion clock");
        tx.send(Packet::Frame(64, 64, pixels, Duration::from_millis(900)))
            .expect("last frame");
        tx.send(Packet::Stop(Duration::from_secs(1))).expect("stop at 1s");
        drop(tx);
        let (saved, total, unique) = encode(
            rx,
            path.clone(),
            Options {
                height: 0,
                ..Options::default()
            },
            None,
            Some(mic_tap),
            None,
            ffmpeg.clone(),
        )
        .expect("MP4 worker must record own microphone");
        assert_eq!((total, unique), (30, 3));
        assert_eq!(
            mic.available(),
            48_000,
            "microphone recording must not consume samples reserved for voice"
        );
        let probe = ffmpeg_command(&ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,codec_type",
                "-show_entries",
                "format=duration",
                "-of",
                "json",
            ])
            .arg(&saved)
            .output()
            .expect("probe microphone-only MP4");
        assert!(probe.status.success());
        let meta: serde_json::Value = serde_json::from_slice(&probe.stdout).expect("probe JSON");
        let streams = meta["streams"].as_array().expect("streams");
        assert!(streams.iter().any(|s| s["codec_type"] == "video" && s["codec_name"] == "h264"));
        assert!(
            streams.iter().any(|s| s["codec_type"] == "audio" && s["codec_name"] == "aac"),
            "MP4 microphone-only narration must have AAC: {meta}"
        );
        let duration: f64 = meta["format"]["duration"]
            .as_str()
            .expect("duration")
            .parse()
            .expect("duration numeric");
        assert!(
            (0.95..=1.1).contains(&duration),
            "video and narration must have matching timeline: {duration}"
        );
        fs::remove_file(saved).expect("cleanup mic-only MP4");
    }

    #[test]
    fn h264_mp4_mixes_own_microphone_and_world_audio_into_decodable_aac() {
        if std::env::var_os("WATERS_ROUX_TEST_MP4").is_none() {
            return;
        }
        let ffmpeg = locate_encoder().expect("bundled encoder for mic regression");
        let ffprobe = ffmpeg.with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        assert!(ffprobe.is_file(), "bundled ffprobe needed");
        let root = std::env::temp_dir().join(format!("waters-roux-mic-mix-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create mic fixture");
        let video = root.join("silent.mp4");
        let world = root.join("world.pcm");
        let microphone = root.join("microphone.pcm");
        let final_mp4 = root.join("narration.mp4");
        let mut encoder = VideoEncoder::start(&ffmpeg, &video, 64, 64, Options::default()).expect("start video");
        for _ in 0..30 {
            encoder.write(&vec![127u8; 64 * 64 * 4]).expect("encode frame");
        }
        assert_eq!(encoder.finish().expect("finish video"), 30);
        let mut world_file = File::create(&world).expect("world track");
        let mut mic_file = File::create(&microphone).expect("mic track");
        for chunk in 0..100 {
            let mut world_pcm = [0.0; 960];
            let mut mic_pcm = [0.0; 480];
            for i in 0..480 {
                let time = (chunk * 480 + i) as f32 / 48_000.0;
                let a = 0.12 * (time * 440.0 * std::f32::consts::TAU).sin();
                let b = 0.12 * (time * 880.0 * std::f32::consts::TAU).sin();
                world_pcm[i * 2] = a;
                world_pcm[i * 2 + 1] = a;
                mic_pcm[i] = b;
            }
            write_audio_packet(&mut world_file, &world_pcm).expect("world PCM");
            write_microphone_packet(&mut mic_file, &mic_pcm).expect("own microphone PCM");
        }
        drop(world_file);
        drop(mic_file);
        mux_world_and_microphone(&ffmpeg, &video, &world, &microphone, &final_mp4).expect("mix both opt-in sources into AAC");
        let probe = ffmpeg_command(&ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type,codec_name,channels,sample_rate",
                "-show_entries",
                "format=duration",
                "-of",
                "json",
            ])
            .arg(&final_mp4)
            .output()
            .expect("ffprobe mixed MP4");
        assert!(probe.status.success());
        let meta: serde_json::Value = serde_json::from_slice(&probe.stdout).expect("valid codec JSON");
        let streams = meta["streams"].as_array().expect("stream array");
        assert!(streams.iter().any(|s| s["codec_type"] == "video" && s["codec_name"] == "h264"));
        assert!(
            streams
                .iter()
                .any(|s| s["codec_type"] == "audio" && s["codec_name"] == "aac" && s["channels"] == 2 && s["sample_rate"] == "48000")
        );
        let duration: f64 = meta["format"]["duration"]
            .as_str()
            .expect("duration")
            .parse()
            .expect("duration numeric");
        assert!(
            (0.95..=1.10).contains(&duration),
            "own voice must not change video playback speed: {duration}"
        );
        let decoded = ffmpeg_command(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(&final_mp4)
            .args(["-map", "0:a:0", "-ar", "48000", "-ac", "1", "-f", "s16le", "-"])
            .output()
            .expect("decode narration");
        assert!(decoded.status.success(), "AAC must decode");
        let samples: Vec<f64> = decoded
            .stdout
            .chunks_exact(2)
            .map(|b| f64::from(i16::from_le_bytes([b[0], b[1]])))
            .collect();
        assert!(samples.len() >= 47_000, "missing microphone audio: {} samples", samples.len());
        let spectral = |hz: f64| {
            let coeff = 2.0 * (std::f64::consts::TAU * hz / 48_000.0).cos();
            let (mut s1, mut s2) = (0.0, 0.0);
            for &sample in samples.iter().take(48_000) {
                let current = sample + coeff * s1 - s2;
                s2 = s1;
                s1 = current;
            }
            (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0).sqrt()
        };
        assert!(spectral(440.0) > 100_000.0, "world audio was lost");
        assert!(spectral(880.0) > 100_000.0, "local microphone was lost");
        fs::remove_dir_all(root).expect("remove temporary microphone and video test files");
    }

    #[test]
    fn requested_audio_track_stays_present_without_world_audio_samples() {
        let path = std::env::temp_dir().join(format!("waters-roux-empty-audio-{}.pcm", uuid::Uuid::new_v4()));
        let mut file = File::create(&path).expect("create silence test file");
        let mut audio_bytes = 0;
        seed_empty_world_audio(Some(&mut file), &mut audio_bytes).expect("seed missing audio");
        assert_eq!(audio_bytes, 1920, "one 10ms stereo frame");
        seed_empty_world_audio(Some(&mut file), &mut audio_bytes).expect("idempotent");
        file.flush().expect("flush");
        drop(file);
        let bytes = fs::read(&path).expect("read seeded PCM");
        assert_eq!(bytes.len(), 1920);
        assert!(bytes.iter().all(|b| *b == 0), "audio seed must be silence");
        fs::remove_file(path).expect("remove seeded PCM");
    }

    #[test]
    fn h264_mp4_has_valid_atoms_and_cfr_timeline() {
        // CI opts in after staging a trusted, pinned FFmpeg binary.
        let Ok(_) = std::env::var("WATERS_ROUX_TEST_MP4") else { return };
        let exe = locate_encoder().expect("test must have bundled H.264 FFmpeg");
        // The Windows QA runner sets a known path so ffprobe can independently
        // inspect the *viewer encoder's* output, not only a synthetic FFmpeg sample.
        let fixture_path = std::env::var_os("WATERS_ROUX_TEST_MP4_OUTPUT");
        let path = fixture_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join(format!("waters-roux-mp4-{}.mp4", uuid::Uuid::new_v4())));
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let frames = [Duration::ZERO, Duration::from_millis(350), Duration::from_millis(700)];
        for elapsed in frames {
            tx.send(Packet::Frame(64, 64, vec![127; 64 * 64 * 4], elapsed)).expect("send frame");
        }
        tx.send(Packet::Stop(Duration::from_secs(1))).expect("stop");
        drop(tx);
        let (output, total, unique) = encode(
            rx,
            path.clone(),
            Options {
                height: 0,
                fps: 30,
                ..Options::default()
            },
            None,
            None,
            None,
            exe,
        )
        .expect("H.264/MP4 encoding");
        assert_eq!((total, unique), (30, 3));
        check_mp4(&output).expect("real MP4 container");
        if fixture_path.is_none() {
            fs::remove_file(&output).expect("cleanup fixture");
        }
    }

    #[test]
    fn h264_mp4_muxes_aac_world_audio_with_correct_duration() {
        // The Windows packaging gate explicitly opts in with pinned FFmpeg
        // and FFprobe. No microphone or in-world audio is recorded here.
        if std::env::var_os("WATERS_ROUX_TEST_MP4").is_none() {
            return;
        }
        let ffmpeg = locate_encoder().expect("trusted bundled encoder");
        let ffprobe = ffmpeg.with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        assert!(ffprobe.is_file(), "AAC regression must probe with bundled ffprobe");
        let root = std::env::temp_dir().join(format!("waters-roux-aac-fixture-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("fixture folder");
        let silent = root.join("silent-video.mp4");
        let world_audio = root.join("world-output.pcm");
        let final_mp4 = root.join("world-audio.mp4");

        let mut encoder = VideoEncoder::start(&ffmpeg, &silent, 64, 64, Options::default()).expect("H.264 fixture encoder");
        let frame = vec![127; 64 * 64 * 4];
        for _ in 0..30 {
            encoder.write(&frame).expect("encode fixture frame");
        }
        assert_eq!(encoder.finish().expect("finalize fixture video"), 30);
        let mut pcm = File::create(&world_audio).expect("create 48 kHz stereo test track");
        for chunk in 0..100usize {
            let mut samples = [0.0_f32; 960];
            for frame in 0..480usize {
                let sample_at = (chunk * 480 + frame) as f32 / 48_000.0;
                let sample = (sample_at * 440.0 * std::f32::consts::TAU).sin() * 0.12;
                samples[frame * 2] = sample;
                samples[frame * 2 + 1] = sample;
            }
            write_audio_packet(&mut pcm, &samples).expect("record PCM block");
        }
        pcm.flush().expect("flush PCM");
        drop(pcm);
        assert_eq!(fs::metadata(&world_audio).expect("PCM metadata").len(), 192_000);

        mux_world_audio(&ffmpeg, &silent, &world_audio, &final_mp4).expect("AAC audio mux");
        check_mp4(&final_mp4).expect("final AAC MP4 container");
        let result = ffmpeg_command(&ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,codec_type,channels,sample_rate",
                "-show_entries",
                "format=duration",
                "-of",
                "json",
            ])
            .arg(&final_mp4)
            .stdin(Stdio::null())
            .output()
            .expect("probe AAC MP4");
        assert!(
            result.status.success(),
            "ffprobe failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let metadata: serde_json::Value = serde_json::from_slice(&result.stdout).expect("ffprobe JSON");
        let streams = metadata["streams"].as_array().expect("streams");
        assert!(
            streams.iter().any(|s| s["codec_type"] == "video" && s["codec_name"] == "h264"),
            "H.264 stream required: {metadata}"
        );
        assert!(
            streams
                .iter()
                .any(|s| { s["codec_type"] == "audio" && s["codec_name"] == "aac" && s["channels"] == 2 && s["sample_rate"] == "48000" }),
            "48kHz stereo AAC must survive real muxing: {metadata}"
        );
        let duration: f64 = metadata["format"]["duration"]
            .as_str()
            .expect("duration")
            .parse()
            .expect("numeric duration");
        assert!(
            (0.95..=1.10).contains(&duration),
            "1s recording must not play at 2x speed: {duration}"
        );
        let decode = ffmpeg_command(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
            .arg(&final_mp4)
            .args(["-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-"])
            .stdin(Stdio::null())
            .output()
            .expect("decode audio+video together");
        assert!(
            decode.status.success(),
            "MP4 stream decode failed: {}",
            String::from_utf8_lossy(&decode.stderr)
        );
        fs::remove_dir_all(&root).expect("cleanup fixture folder");
    }
}
