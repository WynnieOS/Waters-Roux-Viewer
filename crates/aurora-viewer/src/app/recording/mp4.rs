//! H.264/MP4 recorder backend. Frames are encoded *during* capture rather than
//! first JPEG-compressed into an AVI; in-world audio is muxed as AAC on stop.
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
        let Ok(probe) = Command::new(&candidate)
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
        let child = Command::new(ffmpeg)
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
    out.write_all(&bytes).map_err(|e| format!("Cannot capture in-world audio: {e}"))
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

pub(super) fn encode(
    rx: Receiver<Packet>,
    path: PathBuf,
    options: Options,
    audio: Option<aurora_audio::RecordingTap>,
    queued_frames: Option<Arc<AtomicUsize>>,
    ffmpeg: PathBuf,
) -> Result<(PathBuf, u64, u64), String> {
    let video_work = path.with_extension("recording-video.mp4");
    let audio_work = path.with_extension("recording-audio.pcm");
    let mux_work = path.with_extension("recording-final.mp4");
    for work in [&video_work, &audio_work, &mux_work] {
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
    let mut pcm = if audio.is_some() {
        Some(File::create(&audio_work).map_err(|e| format!("Cannot prepare audio track: {e}"))?)
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

        // World audio uses an independent tap; no microphone data is consumed.
        if let (Some(tap), Some(file)) = (audio.as_ref(), pcm.as_mut()) {
            for _ in 0..256 {
                let Some(samples) = tap.try_pop() else { break };
                write_audio_packet(file, &samples)?;
                audio_bytes += 1920;
            }
        }
        if stop {
            break;
        }
    }

    // All PCM must be closed before FFmpeg attempts to mux it.
    if let Some(mut file) = pcm.take() {
        file.flush().map_err(|e| format!("Audio track flush: {e}"))?;
    }
    let Some(video) = encoder else {
        if audio_work.exists() {
            let _ = fs::remove_file(&audio_work);
        }
        return Ok((path, 0, 0));
    };
    let frames = video.finish()?;
    check_mp4(&video_work)?;

    if audio_bytes > 0 {
        let result = Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .arg("-i")
            .arg(&video_work)
            .args(["-f", "s16le", "-ar", &SAMPLE_RATE.to_string(), "-ac", &AUDIO_CHANNELS.to_string()])
            .arg("-i")
            .arg(&audio_work)
            .args(["-map", "0:v:0", "-map", "1:a:0", "-c:v", "copy"])
            .args(["-c:a", "aac", "-b:a", "192k", "-af", "aresample=async=1:first_pts=0,apad"])
            .args(["-shortest", "-movflags", "+faststart", "-f", "mp4"])
            .arg(&mux_work)
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
        check_mp4(&mux_work)?;
        fs::rename(&mux_work, &path).map_err(|e| format!("Cannot save completed MP4: {e}"))?;
        let _ = fs::remove_file(&video_work);
        let _ = fs::remove_file(&audio_work);
    } else {
        fs::rename(&video_work, &path).map_err(|e| format!("Cannot save completed MP4: {e}"))?;
        if audio_work.exists() {
            let _ = fs::remove_file(&audio_work);
        }
    }
    check_mp4(&path)?;
    Ok((path, frames, unique_frames))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            exe,
        )
        .expect("H.264/MP4 encoding");
        assert_eq!((total, unique), (30, 3));
        check_mp4(&output).expect("real MP4 container");
        if fixture_path.is_none() {
            fs::remove_file(&output).expect("cleanup fixture");
        }
    }
}
