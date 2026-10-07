//! Windows implementations of the Nook's local utility sources.
//! All WinRT objects live on the single utilities worker thread.

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use windows::{
    core::{Interface, RuntimeType, HRESULT},
    Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapSize, BitmapTransform,
        ColorManagementMode, ExifOrientationMode, SoftwareBitmap,
    },
    Media::{
        Capture::{
            Frames::{
                MediaFrameReader, MediaFrameReaderStartStatus, MediaFrameSourceGroup,
                MediaFrameSourceKind,
            },
            MediaCapture, MediaCaptureInitializationSettings, MediaCaptureMemoryPreference,
            MediaCaptureSharingMode, StreamingCaptureMode,
        },
        Control::{
            GlobalSystemMediaTransportControlsSession,
            GlobalSystemMediaTransportControlsSessionManager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus,
        },
        MediaProperties::MediaEncodingSubtypes,
    },
    Storage::Streams::{Buffer, DataReader},
    Win32::System::{
        Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
    },
};

use super::{Availability, Calendar, CameraFrame, Media, MediaCommand, Mirror, Power};

const MAX_WIDTH: u32 = 640;
const MAX_HEIGHT: u32 = 480;
const FRAME_PERIOD: Duration = Duration::from_millis(100);
const MEDIA_WAIT: Duration = Duration::from_millis(900);
const CAMERA_WAIT: Duration = Duration::from_secs(3);
const STOP_WAIT: Duration = Duration::from_millis(350);
const ARTWORK_WAIT: Duration = Duration::from_millis(700);
const MAX_ARTWORK_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ARTWORK_DIMENSION: u32 = 256;

pub(super) struct Platform {
    com_initialized: bool,
    media_manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
    capture: Option<MediaCapture>,
    reader: Option<MediaFrameReader>,
    last_frame: Option<Instant>,
}

impl Platform {
    pub(super) fn new() -> Self {
        // Service creates a dedicated worker; balance a successful CoInitializeEx on stop.
        let com_initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
        Self {
            com_initialized,
            media_manager: None,
            capture: None,
            reader: None,
            last_frame: None,
        }
    }

    fn manager(
        &mut self,
    ) -> windows::core::Result<&GlobalSystemMediaTransportControlsSessionManager> {
        if self.media_manager.is_none() {
            self.media_manager = Some(wait(
                GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?,
                MEDIA_WAIT,
                "media manager",
            )?);
        }
        Ok(self
            .media_manager
            .as_ref()
            .expect("manager was initialized"))
    }

    fn session(
        &mut self,
    ) -> windows::core::Result<Option<GlobalSystemMediaTransportControlsSession>> {
        let session = self.manager()?.GetCurrentSession()?;
        Ok((!session.as_raw().is_null()).then_some(session))
    }

    pub(super) fn refresh_media(&mut self) -> Media {
        if !self.com_initialized {
            return Media {
                status: Availability::Error("Windows COM initialization failed".into()),
                ..Media::default()
            };
        }
        let result = (|| -> windows::core::Result<Option<Media>> {
            let Some(session) = self.session()? else {
                return Ok(None);
            };
            let properties = wait(
                session.TryGetMediaPropertiesAsync()?,
                MEDIA_WAIT,
                "media properties",
            )?;
            let info = session.GetPlaybackInfo()?;
            let controls = info.Controls()?;
            let timeline = session.GetTimelineProperties()?;
            let start = timeline.StartTime()?.Duration;
            let end = timeline.EndTime()?.Duration;
            let position = timeline.Position()?.Duration;
            Ok(Some(Media {
                status: Availability::Ready,
                title: properties.Title()?.to_string(),
                artist: properties.Artist()?.to_string(),
                source: session.SourceAppUserModelId()?.to_string(),
                playing: info.PlaybackStatus()?
                    == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing,
                position_seconds: (position - start).max(0) as f64 / 10_000_000.0,
                duration_seconds: (end - start).max(0) as f64 / 10_000_000.0,
                can_seek: controls.IsPlaybackPositionEnabled()?,
                can_toggle: controls.IsPlayPauseToggleEnabled()?,
                can_next: controls.IsNextEnabled()?,
                can_previous: controls.IsPreviousEnabled()?,
                artwork: Self::artwork(&properties),
            }))
        })();
        match result {
            Ok(Some(media)) => media,
            Ok(None) => Media {
                status: Availability::Disconnected,
                ..Media::default()
            },
            Err(error) => {
                self.media_manager = None; // A service restart must be discoverable.
                Media {
                    status: api_status("Media", &error),
                    ..Media::default()
                }
            }
        }
    }

    pub(super) fn media_command(&mut self, command: MediaCommand) -> Result<(), String> {
        if !self.com_initialized {
            return Err("Windows COM initialization failed".into());
        }
        let session = self
            .session()
            .map_err(|e| format!("Media: {e}"))?
            .ok_or_else(|| "No active media session".to_string())?;
        let accepted = match command {
            MediaCommand::Toggle => session.TryTogglePlayPauseAsync(),
            MediaCommand::Next => session.TrySkipNextAsync(),
            MediaCommand::Previous => session.TrySkipPreviousAsync(),
            MediaCommand::Seek(seconds) => {
                if !seconds.is_finite()
                    || seconds < 0.0
                    || seconds > (i64::MAX as f64 / 10_000_000.0)
                {
                    return Err("Invalid seek position".into());
                }
                let timeline = session.GetTimelineProperties().map_err(|e| e.to_string())?;
                let start = timeline.StartTime().map_err(|e| e.to_string())?.Duration;
                let end = timeline.EndTime().map_err(|e| e.to_string())?.Duration;
                let offset = (seconds * 10_000_000.0) as i64;
                let target = start.checked_add(offset).ok_or("Invalid seek position")?;
                if target > end {
                    return Err("Seek position exceeds media duration".into());
                }
                session.TryChangePlaybackPositionAsync(target)
            }
        }
        .map_err(|e| format!("Media: {e}"))?;
        let accepted =
            wait(accepted, MEDIA_WAIT, "media command").map_err(|e| format!("Media: {e}"))?;
        if accepted {
            Ok(())
        } else {
            Err("Media session rejected command".into())
        }
    }

    fn artwork(
        properties: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<Arc<CameraFrame>> {
        // Thumbnail failures must not turn a valid media session into an error.
        let thumbnail = properties.Thumbnail().ok()?;
        if thumbnail.as_raw().is_null() {
            return None;
        }
        let stream = wait(
            thumbnail.OpenReadAsync().ok()?,
            ARTWORK_WAIT,
            "artwork stream",
        )
        .ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > MAX_ARTWORK_BYTES {
            return None;
        }
        let decoder = wait(
            BitmapDecoder::CreateAsync(&stream).ok()?,
            ARTWORK_WAIT,
            "artwork decoder",
        )
        .ok()?;
        let (source_width, source_height) =
            (decoder.PixelWidth().ok()?, decoder.PixelHeight().ok()?);
        if source_width == 0 || source_height == 0 || source_width > 4096 || source_height > 4096 {
            return None;
        }
        let scale =
            (MAX_ARTWORK_DIMENSION as f64 / source_width.max(source_height) as f64).min(1.0);
        let width = (source_width as f64 * scale).round().max(1.0) as u32;
        let height = (source_height as f64 * scale).round().max(1.0) as u32;
        let transform = BitmapTransform::new().ok()?;
        transform.SetScaledWidth(width).ok()?;
        transform.SetScaledHeight(height).ok()?;
        let pixels = wait(
            decoder
                .GetPixelDataTransformedAsync(
                    BitmapPixelFormat::Rgba8,
                    BitmapAlphaMode::Straight,
                    &transform,
                    ExifOrientationMode::IgnoreExifOrientation,
                    ColorManagementMode::DoNotColorManage,
                )
                .ok()?,
            ARTWORK_WAIT,
            "artwork pixels",
        )
        .ok()?;
        let rgba = pixels.DetachPixelData().ok()?;
        if rgba.len() != (width as usize * height as usize * 4) {
            return None;
        }
        Some(Arc::new(CameraFrame {
            width: width as usize,
            height: height as usize,
            rgba: rgba.to_vec(),
        }))
    }

    pub(super) fn power(&self) -> Power {
        let mut raw = SYSTEM_POWER_STATUS::default();
        if let Err(error) = unsafe { GetSystemPowerStatus(&mut raw) } {
            return Power {
                status: api_status("Power", &error),
                ..Power::default()
            };
        }
        if raw.BatteryFlag != 255 && raw.BatteryFlag & 128 != 0 {
            return Power {
                status: Availability::Unavailable("No system battery".into()),
                on_ac: (raw.ACLineStatus != 255).then_some(raw.ACLineStatus == 1),
                ..Power::default()
            };
        }
        Power {
            status: Availability::Ready,
            charge_percent: (raw.BatteryLifePercent <= 100).then_some(raw.BatteryLifePercent),
            charging: (raw.BatteryFlag != 255).then_some(raw.BatteryFlag & 8 != 0),
            on_ac: (raw.ACLineStatus != 255).then_some(raw.ACLineStatus == 1),
            remaining_seconds: (raw.BatteryLifeTime != u32::MAX && raw.ACLineStatus == 0)
                .then_some(raw.BatteryLifeTime as u64),
        }
    }

    pub(super) fn calendar(
        &mut self,
        _start_ms: i64,
        _end_ms: i64,
        _request_permission: bool,
    ) -> Calendar {
        // AppointmentStore requires a packaged identity and the appointmentsSystem
        // manifest capability; this unpackaged desktop executable has neither.
        Calendar {
            status: Availability::Unavailable("Windows Calendar needs a packaged app with appointments permission; import a local .ics file".into()),
            source: "Windows Calendar".into(),
            ..Calendar::default()
        }
    }

    pub(super) fn set_mirror(&mut self, enabled: bool) -> Mirror {
        if !enabled {
            self.stop_camera();
            return Mirror {
                status: Availability::Disconnected,
                frame: None,
            };
        }
        if self.reader.is_some() {
            return Mirror {
                status: Availability::Ready,
                frame: None,
            };
        }
        if !self.com_initialized {
            return Mirror {
                status: Availability::Error("Windows COM initialization failed".into()),
                frame: None,
            };
        }
        match self.start_camera() {
            Ok(()) => Mirror {
                status: Availability::Ready,
                frame: None,
            },
            Err(error) => {
                self.stop_camera();
                Mirror {
                    status: api_status("Camera", &error),
                    frame: None,
                }
            }
        }
    }

    fn start_camera(&mut self) -> windows::core::Result<()> {
        let groups = wait(
            MediaFrameSourceGroup::FindAllAsync()?,
            CAMERA_WAIT,
            "camera discovery",
        )?;
        let mut selected = None;
        for index in 0..groups.Size()? {
            let group = groups.GetAt(index)?;
            let infos = group.SourceInfos()?;
            for info_index in 0..infos.Size()? {
                let info = infos.GetAt(info_index)?;
                if info.SourceKind()? == MediaFrameSourceKind::Color {
                    selected = Some((group.clone(), info.Id()?));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let (group, source_id) = selected.ok_or_else(|| {
            windows::core::Error::new(HRESULT(0x80070490u32 as i32), "No color camera found")
        })?;
        let settings = MediaCaptureInitializationSettings::new()?;
        settings.SetSourceGroup(&group)?;
        settings.SetStreamingCaptureMode(StreamingCaptureMode::Video)?;
        settings.SetMemoryPreference(MediaCaptureMemoryPreference::Cpu)?;
        settings.SetSharingMode(MediaCaptureSharingMode::SharedReadOnly)?;
        let capture = MediaCapture::new()?;
        wait(
            capture.InitializeWithSettingsAsync(&settings)?,
            CAMERA_WAIT,
            "camera initialization",
        )?;
        let source = capture.FrameSources()?.Lookup(&source_id)?;
        let reader = wait(
            capture.CreateFrameReaderWithSubtypeAndSizeAsync(
                &source,
                &MediaEncodingSubtypes::Argb32()?,
                BitmapSize {
                    Width: MAX_WIDTH,
                    Height: MAX_HEIGHT,
                },
            )?,
            CAMERA_WAIT,
            "camera frame reader",
        )?;
        let started = wait(reader.StartAsync()?, CAMERA_WAIT, "camera start")?;
        if started != MediaFrameReaderStartStatus::Success {
            return Err(windows::core::Error::new(
                HRESULT(0x80004005u32 as i32),
                format!("Camera reader failed to start ({})", started.0),
            ));
        }
        self.capture = Some(capture);
        self.reader = Some(reader);
        self.last_frame = None;
        Ok(())
    }

    pub(super) fn next_frame(&mut self) -> Option<Result<CameraFrame, String>> {
        let reader = self.reader.as_ref()?;
        if self
            .last_frame
            .is_some_and(|when| when.elapsed() < FRAME_PERIOD)
        {
            return None;
        }
        self.last_frame = Some(Instant::now());
        let result = (|| -> windows::core::Result<Option<CameraFrame>> {
            let frame = reader.TryAcquireLatestFrame()?;
            if frame.as_raw().is_null() {
                return Ok(None);
            }
            let video = frame.VideoMediaFrame()?;
            if video.as_raw().is_null() {
                return Ok(None);
            }
            let bitmap = video.SoftwareBitmap()?;
            if bitmap.as_raw().is_null() {
                return Ok(None);
            }
            let converted = if bitmap.BitmapPixelFormat()? == BitmapPixelFormat::Bgra8 {
                bitmap
            } else {
                SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)?
            };
            let width = converted.PixelWidth()? as usize;
            let height = converted.PixelHeight()? as usize;
            if width == 0
                || height == 0
                || width > MAX_WIDTH as usize
                || height > MAX_HEIGHT as usize
            {
                return Err(windows::core::Error::new(
                    HRESULT(0x80070057u32 as i32),
                    "Camera frame exceeds size limit",
                ));
            }
            let count = width * height * 4;
            let buffer = Buffer::Create(count as u32)?;
            buffer.SetLength(count as u32)?;
            converted.CopyToBuffer(&buffer)?;
            let mut rgba = vec![0; count];
            DataReader::FromBuffer(&buffer)?.ReadBytes(&mut rgba)?;
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            Ok(Some(CameraFrame {
                width,
                height,
                rgba,
            }))
        })();
        match result {
            Ok(frame) => frame.map(Ok),
            Err(error) => Some(Err(format!("Camera: {error}"))),
        }
    }

    fn stop_camera(&mut self) {
        if let Some(reader) = self.reader.take() {
            if let Ok(action) = reader.StopAsync() {
                let _ = wait(action, STOP_WAIT, "camera stop");
            }
        }
        self.capture = None;
        self.last_frame = None;
    }

    pub(super) fn stop(&mut self) {
        self.stop_camera();
        self.media_manager = None;
        if self.com_initialized {
            unsafe { CoUninitialize() };
            self.com_initialized = false;
        }
    }
}

fn api_status(source: &str, error: &windows::core::Error) -> Availability {
    let message = format!("{source}: {error}");
    match error.code().0 as u32 {
        0x80070005 | 0x800704c7 => Availability::Denied(message),
        0x80070490 | 0x8007001f => Availability::Unavailable(message),
        _ => Availability::Error(message),
    }
}

// WinRT's `join` waits indefinitely. Poll the public IAsyncInfo status on our
// MTA worker so a missing/disconnected device cannot hold shutdown forever.
trait BoundedOperation {
    type Output;
    fn info(&self) -> windows::core::Result<windows_future::IAsyncInfo>;
    fn result(&self) -> windows::core::Result<Self::Output>;
}

impl BoundedOperation for windows_future::IAsyncAction {
    type Output = ();

    fn info(&self) -> windows::core::Result<windows_future::IAsyncInfo> {
        self.cast()
    }

    fn result(&self) -> windows::core::Result<()> {
        self.GetResults()
    }
}

impl<T: RuntimeType + 'static> BoundedOperation for windows_future::IAsyncOperation<T> {
    type Output = T;

    fn info(&self) -> windows::core::Result<windows_future::IAsyncInfo> {
        self.cast()
    }

    fn result(&self) -> windows::core::Result<T> {
        self.GetResults()
    }
}

fn wait<O: BoundedOperation>(
    operation: O,
    timeout: Duration,
    label: &str,
) -> windows::core::Result<O::Output> {
    let info = operation.info()?;
    let start = Instant::now();
    loop {
        if info.Status()? != windows_future::AsyncStatus::Started {
            return operation.result();
        }
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            let _ = info.Cancel();
            return Err(windows::core::Error::new(
                HRESULT(0x800705B4u32 as i32),
                format!("Timed out waiting for {label}"),
            ));
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}
