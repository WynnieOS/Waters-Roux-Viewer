<# Apply only MP4 app wiring to the already-integrated branded viewer.
   Never replace the entire branded app.rs with Aurora branch content. #>
$ErrorActionPreference = 'Stop'
$appPath = 'crates/aurora-viewer/src/app.rs'
if (-not (Test-Path -LiteralPath $appPath)) { throw "Branded app.rs missing" }
$text = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $appPath).Path)
function Replace-Exactly([string]$before, [string]$after) {
    $count = ([regex]::Matches($script:text, [regex]::Escape($before))).Count
    if ($count -ne 1) { throw "MP4 source integration expected 1 marker, got $count" }
    $script:text = $script:text.Replace($before, $after)
}
$old = @'
BarAction::StopRecording => {
                let result = self.recorder.stop();
                if let Some(previous) = self.recording_huds_restore.take() {
                    self.set_show_huds(previous);
                }
                match result {
                    Ok(Some(path)) => self.world.system_message(format!("Saved video: {}", path.display())),
                    Ok(None) => self.world.system_message("No recorded frames were saved.".to_owned()),
                    Err(error) => self.world.system_message(format!("Video finalize failed: {error}")),
                }
            }
'@
$new = @'
BarAction::StopRecording => {
                let result = self.recorder.stop_async();
                if let Some(previous) = self.recording_huds_restore.take() {
                    self.set_show_huds(previous);
                }
                match result {
                    Ok(true) => self.world.system_message("Finalizing H.264 MP4 in the background. Keep Waters Roux open until saved confirmation.".to_owned()),
                    Ok(false) => self.world.system_message("No recording is active.".to_owned()),
                    Err(error) => self.world.system_message(format!("MP4 finalize failed: {error}")),
                }
            }
'@
Replace-Exactly $old $new

$old = @'
            self.recorder.accept(w, h, pixels);
        }
        if let Some((files, exit)) = &mut self.capture {
'@
$new = @'
            self.recorder.accept(w, h, pixels);
        }
        if let Some(result) = self.recorder.poll_finalization() {
            match result {
                Ok(Some(path)) => self.world.system_message(format!("Saved H.264 MP4: {}", path.display())),
                Ok(None) => self.world.system_message("No recording frames were saved.".to_owned()),
                Err(error) => self.world.system_message(format!("H.264 MP4 recording failed: {error}")),
            }
        }
        if let Some((files, exit)) = &mut self.capture {
'@
Replace-Exactly $old $new

$old = @'
                    self.recorder.active(),
                    self.recorder.elapsed(),
'@
$new = @'
                    self.recorder.active(),
                    self.recorder.finalizing(),
                    self.recorder.elapsed(),
'@
Replace-Exactly $old $new
[IO.File]::WriteAllText((Resolve-Path -LiteralPath $appPath).Path, $text, [Text.UTF8Encoding]::new($false))
'MP4 UI integration applied safely to the branded Waters Roux app.'
