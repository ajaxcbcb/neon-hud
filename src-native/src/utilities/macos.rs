//! macOS adapters. All entry points run on the utilities worker, never in paint.
//! Objective-C calls below use public Foundation, AppKit, EventKit and AVFoundation APIs.
use super::{Availability, Calendar, CalendarEvent, CameraFrame, Media, MediaCommand, Mirror, Power};
use block2::RcBlock;
use std::{ffi::{c_char, c_int, c_void, CStr}, ptr, sync::{Arc, Mutex, OnceLock, atomic::{AtomicBool, AtomicU8, Ordering}, mpsc}, thread, time::{Duration, Instant}};

type Obj = *mut c_void;
type Sel = *mut c_void;
type Cf = *const c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Obj;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    fn objc_retain(object: Obj) -> Obj;
    fn objc_release(object: Obj);
    fn objc_allocateClassPair(superclass: Obj, name: *const c_char, extra: usize) -> Obj;
    fn objc_registerClassPair(class: Obj);
    fn class_addMethod(class: Obj, selector: Sel, implementation: *const c_void, types: *const c_char) -> bool;
    fn class_addProtocol(class: Obj, protocol: Obj) -> bool;
    fn objc_getProtocol(name: *const c_char) -> Obj;
}
#[link(name = "Foundation", kind = "framework")]
extern "C" {}
#[link(name = "AppKit", kind = "framework")]
extern "C" {}
#[link(name = "EventKit", kind = "framework")]
extern "C" {}
#[link(name = "AVFoundation", kind = "framework")]
extern "C" { static AVCaptureSessionPreset640x480: Obj; static AVMediaTypeVideo: Obj; }
#[link(name = "CoreMedia", kind = "framework")]
extern "C" { fn CMSampleBufferGetImageBuffer(sample: Obj) -> Obj; }
#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    static kCVPixelBufferPixelFormatTypeKey: Obj;
    fn CVPixelBufferLockBaseAddress(buffer: Obj, flags: u64) -> c_int;
    fn CVPixelBufferUnlockBaseAddress(buffer: Obj, flags: u64) -> c_int;
    fn CVPixelBufferGetBaseAddress(buffer: Obj) -> *const u8;
    fn CVPixelBufferGetWidth(buffer: Obj) -> usize;
    fn CVPixelBufferGetHeight(buffer: Obj) -> usize;
    fn CVPixelBufferGetBytesPerRow(buffer: Obj) -> usize;
    fn CVPixelBufferGetPixelFormatType(buffer: Obj) -> u32;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: Cf);
    fn CFArrayGetCount(array: Cf) -> isize;
    fn CFArrayGetValueAtIndex(array: Cf, index: isize) -> Cf;
    fn CFDictionaryGetValue(dictionary: Cf, key: Cf) -> Cf;
    fn CFStringCreateWithCString(allocator: Cf, value: *const c_char, encoding: u32) -> Cf;
    fn CFStringCompare(a: Cf, b: Cf, options: u64) -> isize;
    fn CFNumberGetValue(number: Cf, number_type: isize, value: *mut c_void) -> bool;
    fn CFBooleanGetValue(value: Cf) -> bool;
}
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> Cf;
    fn IOPSCopyPowerSourcesList(info: Cf) -> Cf;
    fn IOPSGetPowerSourceDescription(info: Cf, source: Cf) -> Cf;
}
#[link(name = "System")]
extern "C" {
    fn dispatch_queue_create(label: *const c_char, attr: Obj) -> Obj;
    fn dispatch_release(object: Obj);
    fn dispatch_sync_f(queue: Obj, context: *mut c_void, work: unsafe extern "C" fn(*mut c_void));
}

macro_rules! msg0 { ($o:expr, $s:literal, $r:ty) => {{ let f: unsafe extern "C" fn(Obj, Sel) -> $r = std::mem::transmute(objc_msgSend as *const ()); f($o, sel(concat!($s, "\0").as_ptr().cast())) }}; }
macro_rules! msg1 { ($o:expr, $s:literal, $a:expr, $t:ty, $r:ty) => {{ let f: unsafe extern "C" fn(Obj, Sel, $t) -> $r = std::mem::transmute(objc_msgSend as *const ()); f($o, sel(concat!($s, "\0").as_ptr().cast()), $a) }}; }
macro_rules! msg2 { ($o:expr, $s:literal, $a:expr, $ta:ty, $b:expr, $tb:ty, $r:ty) => {{ let f: unsafe extern "C" fn(Obj, Sel, $ta, $tb) -> $r = std::mem::transmute(objc_msgSend as *const ()); f($o, sel(concat!($s, "\0").as_ptr().cast()), $a, $b) }}; }
macro_rules! msg3 { ($o:expr, $s:literal, $a:expr, $ta:ty, $b:expr, $tb:ty, $c:expr, $tc:ty, $r:ty) => {{ let f: unsafe extern "C" fn(Obj, Sel, $ta, $tb, $tc) -> $r = std::mem::transmute(objc_msgSend as *const ()); f($o, sel(concat!($s, "\0").as_ptr().cast()), $a, $b, $c) }}; }

unsafe fn sel(name: *const c_char) -> Sel { sel_registerName(name) }
unsafe fn class(name: &'static [u8]) -> Obj { objc_getClass(name.as_ptr().cast()) }
unsafe fn ns_string(value: &str) -> Obj {
    let alloc = msg0!(class(b"NSString\0"), "alloc", Obj);
    msg3!(alloc, "initWithBytes:length:encoding:", value.as_ptr().cast(), *const c_void, value.len(), usize, 4usize, usize, Obj)
}
unsafe fn rust_string(value: Obj) -> String {
    if value.is_null() { return String::new(); }
    let bytes = msg0!(value, "UTF8String", *const c_char);
    if bytes.is_null() { String::new() } else { CStr::from_ptr(bytes).to_string_lossy().into_owned() }
}
unsafe fn release(value: Obj) { if !value.is_null() { objc_release(value); } }
struct Pool(Obj);
impl Pool {
    unsafe fn new() -> Self { Self(msg0!(class(b"NSAutoreleasePool\0"), "new", Obj)) }
}
impl Drop for Pool { fn drop(&mut self) { unsafe { msg0!(self.0, "drain", ()); } } }
unsafe fn responds(value: Obj, name: &'static [u8]) -> bool {
    !value.is_null() && msg1!(value, "respondsToSelector:", sel(name.as_ptr().cast()), Sel, bool)
}
unsafe fn cf_key(name: &'static [u8]) -> Cf {
    CFStringCreateWithCString(ptr::null(), name.as_ptr().cast(), 0x0800_0100)
}
unsafe fn cf_number(dictionary: Cf, name: &'static [u8]) -> Option<i32> {
    let key = cf_key(name);
    let value = CFDictionaryGetValue(dictionary, key);
    CFRelease(key);
    let mut result = 0i32;
    if !value.is_null() && CFNumberGetValue(value, 9, (&mut result as *mut i32).cast()) { Some(result) } else { None }
}
unsafe fn cf_bool(dictionary: Cf, name: &'static [u8]) -> Option<bool> {
    let key = cf_key(name);
    let value = CFDictionaryGetValue(dictionary, key);
    CFRelease(key);
    if value.is_null() { None } else { Some(CFBooleanGetValue(value)) }
}
unsafe fn cf_equal(dictionary: Cf, name: &'static [u8], expected: &'static [u8]) -> Option<bool> {
    let key = cf_key(name);
    let value = CFDictionaryGetValue(dictionary, key);
    CFRelease(key);
    if value.is_null() { return None; }
    let match_value = cf_key(expected);
    let equal = CFStringCompare(value, match_value, 0) == 0;
    CFRelease(match_value);
    Some(equal)
}

struct Capture { cancel: Arc<AtomicBool>, state: Arc<AtomicU8>, error: Arc<Mutex<Option<String>>> }
// A single helper owns all capture objects until AVFoundation returns from stopRunning.
// This prevents a late callback or a blocked system call from using freed objects.
static CAPTURE_BUSY: AtomicBool = AtomicBool::new(false);
struct FrameSlot { frame: Option<CameraFrame>, last: Option<Instant>, active: bool }
static FRAME: OnceLock<Mutex<FrameSlot>> = OnceLock::new();
fn frame_slot() -> &'static Mutex<FrameSlot> {
    FRAME.get_or_init(|| Mutex::new(FrameSlot { frame: None, last: None, active: false }))
}
// Scripts are fixed here; only a checked numeric seek value is interpolated.
const MUSIC_READ: &str = r#"with timeout of 2 seconds
    tell application id "com.apple.Music"
        set playbackState to player state
        if playbackState is stopped then return {"", "", 0.0, 0.0, false}
        set activeTrack to current track
        return {(name of activeTrack) as text, (artist of activeTrack) as text, (duration of activeTrack) as real, (player position) as real, (playbackState is playing)}
    end tell
end timeout"#;

unsafe fn music_running() -> bool {
    let bundle = ns_string("com.apple.Music");
    let running = msg1!(class(b"NSRunningApplication\0"), "runningApplicationsWithBundleIdentifier:", bundle, Obj, Obj);
    release(bundle);
    !running.is_null() && msg0!(running, "count", usize) > 0
}

unsafe fn music_script(source: &str) -> Result<Obj, String> {
    let source = ns_string(source);
    let script = msg1!(msg0!(class(b"NSAppleScript\0"), "alloc", Obj), "initWithSource:", source, Obj, Obj);
    release(source);
    if script.is_null() { return Err("Music script could not be initialized".into()); }
    let mut error: Obj = ptr::null_mut();
    let result = msg1!(script, "executeAndReturnError:", &mut error as *mut Obj, *mut Obj, Obj);
    release(script);
    if !result.is_null() { return Ok(result); }
    let key = ns_string("NSAppleScriptErrorNumber");
    let number = if error.is_null() { ptr::null_mut() } else { msg1!(error, "objectForKey:", key, Obj, Obj) };
    release(key);
    let code = if number.is_null() { 0 } else { msg0!(number, "integerValue", isize) };
    Err(match code {
        -1743 => "Music automation access denied".into(),
        -600 => "Music is not running".into(),
        -1728 => "No Music track is active".into(),
        -1712 => "Music did not respond in time".into(),
        _ => format!("Music AppleScript failed (error {code})"),
    })
}

unsafe fn music_item(list: Obj, index: isize) -> Result<Obj, String> {
    let item = msg1!(list, "descriptorAtIndex:", index, isize, Obj);
    if item.is_null() { Err("Music returned an incomplete result".into()) } else { Ok(item) }
}

// AVFoundation invokes this on a private serial dispatch queue. Keep only one <=10 fps frame.
unsafe extern "C" fn capture_frame(_this: Obj, _cmd: Sel, _output: Obj, sample: Obj, _connection: Obj) {
    let Ok(mut slot) = frame_slot().try_lock() else { return; };
    if !slot.active || slot.last.is_some_and(|t| t.elapsed() < Duration::from_millis(100)) { return; }
    let pixel = CMSampleBufferGetImageBuffer(sample);
    if pixel.is_null() || CVPixelBufferGetPixelFormatType(pixel) != u32::from_be_bytes(*b"BGRA") { return; }
    let (width, height) = (CVPixelBufferGetWidth(pixel), CVPixelBufferGetHeight(pixel));
    if width == 0 || height == 0 || width > 640 || height > 480 { return; }
    if CVPixelBufferLockBaseAddress(pixel, 1) != 0 { return; }
    let stride = CVPixelBufferGetBytesPerRow(pixel);
    let base = CVPixelBufferGetBaseAddress(pixel);
    if !base.is_null() && stride >= width * 4 {
        let mut rgba = vec![0; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let source = base.add(y * stride + x * 4);
                let dest = (y * width + (width - 1 - x)) * 4;
                rgba[dest] = *source.add(2);
                rgba[dest + 1] = *source.add(1);
                rgba[dest + 2] = *source;
                rgba[dest + 3] = *source.add(3);
            }
        }
        slot.frame = Some(CameraFrame { width, height, rgba });
        slot.last = Some(Instant::now());
    }
    CVPixelBufferUnlockBaseAddress(pixel, 1);
}

unsafe fn capture_delegate() -> Obj {
    static DELEGATE_CLASS: OnceLock<usize> = OnceLock::new();
    let cls = *DELEGATE_CLASS.get_or_init(|| unsafe {
        let cls = objc_allocateClassPair(class(b"NSObject\0"), b"NeonHUDCaptureDelegate\0".as_ptr().cast(), 0);
        if cls.is_null() { return class(b"NeonHUDCaptureDelegate\0") as usize; }
        let protocol = objc_getProtocol(b"AVCaptureVideoDataOutputSampleBufferDelegate\0".as_ptr().cast());
        if !protocol.is_null() { class_addProtocol(cls, protocol); }
        class_addMethod(cls, sel(b"captureOutput:didOutputSampleBuffer:fromConnection:\0".as_ptr().cast()), capture_frame as *const c_void, b"v@:@@@\0".as_ptr().cast());
        objc_registerClassPair(cls);
        cls as usize
    }) as Obj;
    msg0!(cls, "new", Obj)
}

unsafe extern "C" fn capture_queue_barrier(_context: *mut c_void) {}

unsafe fn run_capture(cancel: &AtomicBool, state: &AtomicU8) -> Result<(), String> {
    let _pool = Pool::new();
    let device_class = class(b"AVCaptureDevice\0");
    let mut auth = msg1!(device_class, "authorizationStatusForMediaType:", AVMediaTypeVideo, Obj, isize);
    if auth == 0 {
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |granted: bool| { let _ = tx.send(granted); });
        msg2!(device_class, "requestAccessForMediaType:completionHandler:", AVMediaTypeVideo, Obj, RcBlock::as_ptr(&block).cast::<c_void>(), Obj, ());
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if cancel.load(Ordering::Acquire) { return Ok(()); }
            if Instant::now() >= deadline { return Err("Camera permission timed out".into()); }
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(true) => { auth = 3; break; }
                Ok(false) => { auth = 2; break; }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Camera permission request failed".into()),
            }
        }
    }
    if cancel.load(Ordering::Acquire) { return Ok(()); }
    if auth != 3 { return Err("Camera access denied".into()); }
    let device = msg1!(device_class, "defaultDeviceWithMediaType:", AVMediaTypeVideo, Obj, Obj);
    if device.is_null() { return Err("Camera disconnected".into()); }
    let mut error: Obj = ptr::null_mut();
    let input = msg2!(class(b"AVCaptureDeviceInput\0"), "deviceInputWithDevice:error:", device, Obj, &mut error as *mut Obj, *mut Obj, Obj);
    if input.is_null() { return Err("Camera input unavailable".into()); }
    let session = msg0!(class(b"AVCaptureSession\0"), "new", Obj);
    let output = msg0!(class(b"AVCaptureVideoDataOutput\0"), "new", Obj);
    if session.is_null() || output.is_null() {
        release(session); release(output);
        return Err("Camera capture unavailable".into());
    }
    let configured: Result<(), String> = (|| {
        if !msg1!(session, "canSetSessionPreset:", AVCaptureSessionPreset640x480, Obj, bool) {
            return Err("Camera does not support 640x480".into());
        }
        msg1!(session, "setSessionPreset:", AVCaptureSessionPreset640x480, Obj, ());
        if !msg1!(session, "canAddInput:", input, Obj, bool) || !msg1!(session, "canAddOutput:", output, Obj, bool) {
            return Err("Camera capture configuration failed".into());
        }
        msg1!(session, "addInput:", input, Obj, ());
        msg1!(session, "addOutput:", output, Obj, ());
        let number = msg1!(class(b"NSNumber\0"), "numberWithUnsignedInt:", u32::from_be_bytes(*b"BGRA"), u32, Obj);
        let formats = msg0!(output, "availableVideoCVPixelFormatTypes", Obj);
        if formats.is_null() || !msg1!(formats, "containsObject:", number, Obj, bool) {
            return Err("Camera does not support BGRA frames".into());
        }
        let settings = msg2!(class(b"NSDictionary\0"), "dictionaryWithObject:forKey:", number, Obj, kCVPixelBufferPixelFormatTypeKey, Obj, Obj);
        msg1!(output, "setVideoSettings:", settings, Obj, ());
        msg1!(output, "setAlwaysDiscardsLateVideoFrames:", true, bool, ());
        Ok(())
    })();
    if let Err(message) = configured { release(output); release(session); return Err(message); }
    let delegate = capture_delegate();
    let queue = dispatch_queue_create(b"neon-hud.camera\0".as_ptr().cast(), ptr::null_mut());
    if delegate.is_null() || queue.is_null() {
        release(delegate); release(output); release(session);
        if !queue.is_null() { dispatch_release(queue); }
        return Err("Camera delegate unavailable".into());
    }
    msg2!(output, "setSampleBufferDelegate:queue:", delegate, Obj, queue, Obj, ());
    if let Ok(mut slot) = frame_slot().lock() { slot.frame = None; slot.last = None; slot.active = true; }
    if !cancel.load(Ordering::Acquire) { msg0!(session, "startRunning", ()); }
    let running = msg0!(session, "isRunning", bool);
    if running && !cancel.load(Ordering::Acquire) { state.store(1, Ordering::Release); }
    let mut failure = if !running && !cancel.load(Ordering::Acquire) { Some("Camera did not start".to_string()) } else { None };
    while running && !cancel.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(100));
        if !msg0!(session, "isRunning", bool) { failure = Some("Camera disconnected".into()); break; }
    }
    state.store(3, Ordering::Release);
    if let Ok(mut slot) = frame_slot().lock() { slot.active = false; slot.frame = None; slot.last = None; }
    msg2!(output, "setSampleBufferDelegate:queue:", ptr::null_mut(), Obj, ptr::null_mut(), Obj, ());
    if running { msg0!(session, "stopRunning", ()); }
    dispatch_sync_f(queue, ptr::null_mut(), capture_queue_barrier);
    release(delegate); release(output); release(session); dispatch_release(queue);
    failure.map_or(Ok(()), Err)
}

pub(super) struct Platform { event_store: Obj, capture: Option<Capture> }
impl Platform {
    pub(super) fn new() -> Self { Self { event_store: ptr::null_mut(), capture: None } }

    pub(super) fn refresh_media(&mut self) -> Media {
        let _pool = unsafe { Pool::new() };
        unsafe {
            if !music_running() { return Media { status: Availability::Disconnected, ..Media::default() }; }
            let result = match music_script(MUSIC_READ) {
                Ok(result) => result,
                Err(error) if error.contains("denied") => return Media { status: Availability::Denied(error), ..Media::default() },
                Err(error) if error == "Music is not running" || error == "No Music track is active" => return Media { status: Availability::Disconnected, ..Media::default() },
                Err(error) => return Media { status: Availability::Error(error), ..Media::default() },
            };
            if msg0!(result, "numberOfItems", isize) != 5 {
                return Media { status: Availability::Error("Music returned an unexpected result".into()), ..Media::default() };
            }
            let parsed = (|| -> Result<Media, String> {
                let title = rust_string(msg0!(music_item(result, 1)?, "stringValue", Obj));
                if title.is_empty() { return Ok(Media { status: Availability::Disconnected, ..Media::default() }); }
                let artist = rust_string(msg0!(music_item(result, 2)?, "stringValue", Obj));
                let duration = msg0!(music_item(result, 3)?, "doubleValue", f64);
                let position = msg0!(music_item(result, 4)?, "doubleValue", f64);
                let playing = msg0!(music_item(result, 5)?, "booleanValue", bool);
                if !duration.is_finite() || !position.is_finite() || duration < 0.0 || position < 0.0 {
                    return Err("Music returned invalid timing data".into());
                }
                Ok(Media { status: Availability::Ready, title, artist, source: "Music".into(),
                    artwork: None, playing, position_seconds: position, duration_seconds: duration,
                    can_seek: duration > 0.0, can_toggle: true, can_next: true, can_previous: true })
            })();
            match parsed {
                Ok(media) => media,
                Err(error) => Media { status: Availability::Error(error), ..Media::default() },
            }
        }
    }

    pub(super) fn media_command(&mut self, command: MediaCommand) -> Result<(), String> {
        let _pool = unsafe { Pool::new() };
        unsafe {
            if !music_running() { return Err("Music is not running".into()); }
            let script = match command {
                MediaCommand::Toggle => "tell application id \"com.apple.Music\" to playpause".to_string(),
                MediaCommand::Next => "tell application id \"com.apple.Music\" to next track".to_string(),
                MediaCommand::Previous => "tell application id \"com.apple.Music\" to previous track".to_string(),
                MediaCommand::Seek(seconds) => {
                    if !seconds.is_finite() || !(0.0..=86_400.0).contains(&seconds) { return Err("Invalid seek position".into()); }
                    let media = self.refresh_media();
                    if media.status != Availability::Ready || !media.can_seek { return Err("Track is not seekable".into()); }
                    format!("tell application id \"com.apple.Music\" to set player position to {:.6}", seconds.min(media.duration_seconds))
                }
            };
            music_script(&format!("with timeout of 2 seconds\n{script}\nend timeout"))?;
        }
        Ok(())
    }

    pub(super) fn power(&mut self) -> Power {
        unsafe {
            let info = IOPSCopyPowerSourcesInfo();
            if info.is_null() { return Power { status: Availability::Error("Power sources unavailable".into()), ..Power::default() }; }
            let list = IOPSCopyPowerSourcesList(info);
            if list.is_null() { CFRelease(info); return Power { status: Availability::Disconnected, ..Power::default() }; }
            let mut power = Power { status: Availability::Disconnected, ..Power::default() };
            for i in 0..CFArrayGetCount(list) {
                let description = IOPSGetPowerSourceDescription(info, CFArrayGetValueAtIndex(list, i));
                if description.is_null() { continue; }
                if cf_equal(description, b"Type\0", b"InternalBattery\0") != Some(true) { continue; }
                power.status = Availability::Ready;
                power.charge_percent = cf_number(description, b"Current Capacity\0").and_then(|n| u8::try_from(n.clamp(0, 100)).ok());
                power.charging = cf_bool(description, b"Is Charging\0");
                power.on_ac = cf_equal(description, b"Power Source State\0", b"AC Power\0");
                power.remaining_seconds = cf_number(description, b"Time to Empty\0").and_then(|n| if n >= 0 { Some(n as u64 * 60) } else { None });
                break;
            }
            CFRelease(list);
            CFRelease(info);
            power
        }
    }

    pub(super) fn calendar(&mut self, start_ms: i64, end_ms: i64, request_permission: bool) -> Calendar {
        let _pool = unsafe { Pool::new() };
        unsafe {
            let cls = class(b"EKEventStore\0");
            if cls.is_null() { return Calendar { status: Availability::Unavailable("EventKit unavailable".into()), ..Calendar::default() }; }
            // EKEntityTypeEvent = 0; full access = 3 on old macOS, 4 on macOS 14+.
            let mut auth = msg1!(cls, "authorizationStatusForEntityType:", 0isize, isize, isize);
            if auth == 0 && request_permission {
                if self.event_store.is_null() { self.event_store = msg0!(cls, "new", Obj); }
                let (tx, rx) = mpsc::channel();
                let block = RcBlock::new(move |granted: bool, _error: Obj| { let _ = tx.send(granted); });
                if responds(self.event_store, b"requestFullAccessToEventsWithCompletion:\0") {
                    msg1!(self.event_store, "requestFullAccessToEventsWithCompletion:", RcBlock::as_ptr(&block).cast::<c_void>(), Obj, ());
                } else {
                    msg2!(self.event_store, "requestAccessToEntityType:completion:", 0isize, isize, RcBlock::as_ptr(&block).cast::<c_void>(), Obj, ());
                }
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(_) => auth = msg1!(cls, "authorizationStatusForEntityType:", 0isize, isize, isize),
                    Err(_) => return Calendar { status: Availability::Error("Calendar permission timed out".into()), ..Calendar::default() },
                }
            }
            if auth == 0 { return Calendar { status: Availability::Unavailable("Connect calendar to request access".into()), ..Calendar::default() }; }
            if auth != 3 && auth != 4 { return Calendar { status: Availability::Denied("Calendar access denied".into()), ..Calendar::default() }; }
            if self.event_store.is_null() { self.event_store = msg0!(cls, "new", Obj); }
            if self.event_store.is_null() { return Calendar { status: Availability::Error("Calendar store unavailable".into()), ..Calendar::default() }; }
            let from = msg1!(class(b"NSDate\0"), "dateWithTimeIntervalSince1970:", start_ms as f64 / 1000.0, f64, Obj);
            let to = msg1!(class(b"NSDate\0"), "dateWithTimeIntervalSince1970:", end_ms as f64 / 1000.0, f64, Obj);
            let predicate = msg3!(self.event_store, "predicateForEventsWithStartDate:endDate:calendars:", from, Obj, to, Obj, ptr::null_mut(), Obj, Obj);
            if predicate.is_null() { return Calendar { status: Availability::Error("Calendar query unavailable".into()), ..Calendar::default() }; }
            let found = msg1!(self.event_store, "eventsMatchingPredicate:", predicate, Obj, Obj);
            let count = if found.is_null() { 0 } else { msg0!(found, "count", usize) };
            let mut events = Vec::with_capacity(count.min(256));
            for index in 0..count.min(256) {
                let event = msg1!(found, "objectAtIndex:", index, usize, Obj);
                let start = msg0!(event, "startDate", Obj);
                let end = msg0!(event, "endDate", Obj);
                events.push(CalendarEvent {
                    title: rust_string(msg0!(event, "title", Obj)),
                    starts_at_ms: (msg0!(start, "timeIntervalSince1970", f64) * 1000.0) as i64,
                    ends_at_ms: (msg0!(end, "timeIntervalSince1970", f64) * 1000.0) as i64,
                    all_day: msg0!(event, "isAllDay", bool),
                });
            }
            Calendar { status: Availability::Ready, source: "macOS Calendar".into(), events }
        }
    }

    pub(super) fn set_mirror(&mut self, enabled: bool) -> Mirror {
        if !enabled {
            self.stop_capture();
            return Mirror::default();
        }
        if self.capture.is_some() { return Mirror { status: Availability::Unavailable("Camera starting".into()), ..Mirror::default() }; }
        if CAPTURE_BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return Mirror { status: Availability::Unavailable("Camera is stopping".into()), ..Mirror::default() };
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let state = Arc::new(AtomicU8::new(0)); // starting, running, failed, stopping
        let error = Arc::new(Mutex::new(None));
        let capture = Capture { cancel: Arc::clone(&cancel), state: Arc::clone(&state), error: Arc::clone(&error) };
        let started = thread::Builder::new().name("neon-hud-camera".into()).spawn(move || {
            let result = unsafe { run_capture(&cancel, &state) };
            if let Err(message) = result {
                if let Ok(mut slot) = error.lock() { *slot = Some(message); }
                state.store(2, Ordering::Release);
            } else if !cancel.load(Ordering::Acquire) {
                state.store(2, Ordering::Release);
            }
            CAPTURE_BUSY.store(false, Ordering::Release);
        });
        if started.is_err() {
            CAPTURE_BUSY.store(false, Ordering::Release);
            return Mirror { status: Availability::Error("Camera worker could not start".into()), ..Mirror::default() };
        }
        self.capture = Some(capture);
        Mirror { status: Availability::Unavailable("Camera starting".into()), ..Mirror::default() }
    }

    pub(super) fn next_frame(&mut self) -> Option<Result<CameraFrame, String>> {
        let capture = self.capture.as_ref()?;
        match capture.state.load(Ordering::Acquire) {
            0 | 3 => None,
            1 => frame_slot().lock().ok()?.frame.take().map(Ok),
            _ => Some(Err(capture.error.lock().ok()?.clone().unwrap_or_else(|| "Camera stopped".into()))),
        }
    }

    fn stop_capture(&mut self) {
        if let Some(capture) = self.capture.take() { capture.cancel.store(true, Ordering::Release); }
        if let Ok(mut slot) = frame_slot().lock() { slot.active = false; slot.frame = None; slot.last = None; }
    }
    pub(super) fn stop(&mut self) {
        self.stop_capture();
        unsafe { release(self.event_store); }
        self.event_store = ptr::null_mut();
    }
}
