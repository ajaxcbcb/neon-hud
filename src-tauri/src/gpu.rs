//! Local GPU readings. Adapter identity is cached; unavailable counters stay null.
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSnapshot {
    pub id: String,
    pub name: String,
    pub utilization_percent: Option<f64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub sampled_at: f64,
    pub source: String,
    pub status: String,
    pub message: Option<String>,
}

pub struct GpuMonitor {
    platform: platform::Provider,
    discovered_at: Option<Instant>,
    adapters: Vec<platform::Adapter>,
}

impl GpuMonitor {
    pub fn new() -> Self {
        Self {
            platform: platform::Provider::new(),
            discovered_at: None,
            adapters: Vec::new(),
        }
    }

    pub fn refresh(&mut self, now: f64) -> Vec<GpuSnapshot> {
        if !now.is_finite() {
            return Vec::new();
        }
        if self
            .discovered_at
            .is_none_or(|time| time.elapsed() >= Duration::from_secs(30))
        {
            self.adapters = self.platform.discover();
            self.discovered_at = Some(Instant::now());
        }
        self.platform.sample(&self.adapters, now)
    }
}

impl Default for GpuMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::GpuSnapshot;
    use std::collections::HashMap;
    use std::mem::{size_of, MaybeUninit};
    use std::time::{Duration, Instant};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_FMT_LARGE, PDH_HCOUNTER,
        PDH_HQUERY, PDH_MORE_DATA,
    };

    #[derive(Clone)]
    pub struct Adapter {
        pub id: String,
        pub name: String,
        pub luid: String,
        pub memory_total: Option<u64>,
    }

    struct Counters {
        query: PDH_HQUERY,
        engine: PDH_HCOUNTER,
        memory: PDH_HCOUNTER,
        primed: bool,
    }

    impl Counters {
        fn complete(&self) -> bool {
            !self.engine.0.is_null() && !self.memory.0.is_null()
        }
    }

    fn retry_due(has_counters: bool, complete: bool, elapsed: Duration) -> bool {
        (!has_counters || !complete) && elapsed >= Duration::from_secs(30)
    }

    // PDH query handles are owned by this provider and only touched on the monitor thread.
    unsafe impl Send for Counters {}

    impl Drop for Counters {
        fn drop(&mut self) {
            unsafe { PdhCloseQuery(self.query) };
        }
    }

    pub struct Provider {
        counters: Option<Counters>,
        last_counter_attempt: Instant,
    }

    impl Provider {
        pub fn new() -> Self {
            Self {
                counters: open_counters(),
                last_counter_attempt: Instant::now(),
            }
        }

        pub fn discover(&mut self) -> Vec<Adapter> {
            let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
                return Vec::new();
            };
            let mut adapters = Vec::new();
            for index in 0..32 {
                let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
                    break;
                };
                let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
                    continue;
                };
                if desc.Flags & 2 != 0 {
                    continue; // DXGI_ADAPTER_FLAG_SOFTWARE
                }
                let name_end = desc.Description.iter().position(|c| *c == 0).unwrap_or(128);
                let name = String::from_utf16_lossy(&desc.Description[..name_end]);
                let luid = format!(
                    "{:08x}_{:08x}",
                    desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart
                );
                adapters.push(Adapter {
                    id: format!("dxgi:{luid}"),
                    name,
                    luid,
                    memory_total: (desc.DedicatedVideoMemory > 0)
                        .then_some(desc.DedicatedVideoMemory as u64),
                });
            }
            adapters
        }

        pub fn sample(&mut self, adapters: &[Adapter], now: f64) -> Vec<GpuSnapshot> {
            if retry_due(
                self.counters.is_some(),
                self.counters.as_ref().is_some_and(Counters::complete),
                self.last_counter_attempt.elapsed(),
            ) {
                drop(self.counters.take()); // Close the old query before opening a replacement.
                self.counters = open_counters();
                self.last_counter_attempt = Instant::now();
            }
            let mut engines = HashMap::new();
            let mut memory = HashMap::new();
            let mut collection_failed = false;
            if let Some(counters) = &mut self.counters {
                if unsafe { PdhCollectQueryData(counters.query) } == 0 {
                    if counters.primed {
                        engines = aggregate_engines(read_array(counters.engine, PDH_FMT_DOUBLE));
                        memory = aggregate_memory(read_array(counters.memory, PDH_FMT_LARGE));
                    }
                    counters.primed = true;
                } else {
                    collection_failed = true;
                }
            }
            if collection_failed {
                drop(self.counters.take());
                self.last_counter_attempt = Instant::now();
            }
            adapters.iter().map(|adapter| {
                let utilization = engines.get(&adapter.luid).copied();
                let used = memory.get(&adapter.luid).copied();
                let live = utilization.is_some() || used.is_some();
                GpuSnapshot {
                    id: adapter.id.clone(), name: adapter.name.clone(),
                    utilization_percent: utilization, memory_used_bytes: used,
                    memory_total_bytes: adapter.memory_total, temperature_celsius: None,
                    sampled_at: now, source: "DXGI + Windows GPU performance counters".into(),
                    status: if live { "live" } else { "unavailable" }.into(),
                    message: (!live).then(|| "GPU activity counters unavailable or warming up; temperature unsupported".into()),
                }
            }).collect()
        }
    }

    fn open_counters() -> Option<Counters> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let mut engine = PDH_HCOUNTER::default();
            let mut memory = PDH_HCOUNTER::default();
            let engine_ok = PdhAddEnglishCounterW(
                query,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut engine,
            ) == 0;
            let memory_ok = PdhAddEnglishCounterW(
                query,
                w!("\\GPU Adapter Memory(*)\\Dedicated Usage"),
                0,
                &mut memory,
            ) == 0;
            if !engine_ok {
                engine = PDH_HCOUNTER::default();
            }
            if !memory_ok {
                memory = PDH_HCOUNTER::default();
            }
            if !engine_ok && !memory_ok {
                PdhCloseQuery(query);
                return None;
            }
            Some(Counters {
                query,
                engine,
                memory,
                primed: false,
            })
        }
    }

    fn read_array(
        counter: PDH_HCOUNTER,
        format: windows::Win32::System::Performance::PDH_FMT,
    ) -> Vec<(String, f64)> {
        if counter.0.is_null() {
            return Vec::new();
        }
        unsafe {
            let mut bytes = 0u32;
            let mut count = 0u32;
            let status =
                PdhGetFormattedCounterArrayW(counter, format, &mut bytes, &mut count, None);
            if status != PDH_MORE_DATA || bytes == 0 || bytes > 4 * 1024 * 1024 {
                return Vec::new();
            }
            let words = (bytes as usize).div_ceil(size_of::<usize>());
            let mut buffer = vec![MaybeUninit::<usize>::uninit(); words];
            if PdhGetFormattedCounterArrayW(
                counter,
                format,
                &mut bytes,
                &mut count,
                Some(buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()),
            ) != 0
            {
                return Vec::new();
            }
            if count as usize > bytes as usize / size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() {
                return Vec::new();
            }
            let items = std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            );
            items
                .iter()
                .filter_map(|item| {
                    if item.szName.is_null() || item.FmtValue.CStatus > 1 {
                        return None;
                    }
                    let name = item.szName.to_string().ok()?;
                    let value = if format == PDH_FMT_DOUBLE {
                        item.FmtValue.Anonymous.doubleValue
                    } else {
                        item.FmtValue.Anonymous.largeValue as f64
                    };
                    value.is_finite().then_some((name, value))
                })
                .collect()
        }
    }

    fn luid_from_instance(name: &str) -> Option<String> {
        let lower = name.to_ascii_lowercase();
        let rest = lower.split("luid_0x").nth(1)?;
        let (hi, lo) = rest.split_once("_0x")?;
        let lo = lo.split('_').next()?;
        let hi = u32::from_str_radix(hi, 16).ok()?;
        let lo = u32::from_str_radix(lo, 16).ok()?;
        Some(format!("{hi:08x}_{lo:08x}"))
    }

    fn engine_key(name: &str) -> Option<(String, String)> {
        let luid = luid_from_instance(name)?;
        let lower = name.to_ascii_lowercase();
        let physical = lower.split("_phys_").nth(1)?.split('_').next()?;
        let engine = lower.split("_eng_").nth(1)?.split('_').next()?;
        let engine_type = lower.split("_engtype_").nth(1)?.split('_').next()?;
        Some((luid, format!("{physical}:{engine}:{engine_type}")))
    }

    fn aggregate_engines(items: Vec<(String, f64)>) -> HashMap<String, f64> {
        let mut sums: HashMap<(String, String), f64> = HashMap::new();
        for (name, value) in items {
            if let Some(key) = engine_key(&name) {
                *sums.entry(key).or_default() += value.max(0.0);
            }
        }
        let mut busiest: HashMap<String, f64> = HashMap::new();
        for ((luid, _), sum) in sums {
            busiest
                .entry(luid)
                .and_modify(|v| *v = v.max(sum))
                .or_insert(sum);
        }
        busiest.values_mut().for_each(|v| *v = v.clamp(0.0, 100.0));
        busiest
    }

    fn aggregate_memory(items: Vec<(String, f64)>) -> HashMap<String, u64> {
        let mut totals = HashMap::new();
        for (name, value) in items {
            if let Some(luid) = luid_from_instance(&name) {
                if value >= 0.0 && value <= u64::MAX as f64 {
                    let total = totals.entry(luid).or_insert(0u64);
                    *total = total.saturating_add(value as u64);
                }
            }
        }
        totals
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn missing_or_partial_counters_retry_only_after_thirty_seconds() {
            assert!(!retry_due(false, false, Duration::from_secs(29)));
            assert!(retry_due(false, false, Duration::from_secs(30)));
            assert!(retry_due(true, false, Duration::from_secs(30)));
            assert!(!retry_due(true, true, Duration::from_secs(600)));
        }
        #[test]
        fn busiest_engine_not_sum_of_all_engines() {
            let prefix = "pid_1_luid_0x00000002_0x0000000a_phys_0";
            let readings = vec![
                (format!("{prefix}_eng_0_engtype_3D"), 30.0),
                (format!("pid_2_luid_0x2_0xa_phys_0_eng_0_engtype_3D"), 20.0),
                (format!("{prefix}_eng_1_engtype_Copy"), 70.0),
            ];
            assert_eq!(aggregate_engines(readings)["00000002_0000000a"], 70.0);
            assert_eq!(luid_from_instance("invalid"), None);
        }
        #[test]
        fn multiple_adapter_memory_is_separate() {
            let values = aggregate_memory(vec![
                ("luid_0x1_0x1_phys_0".into(), 100.0),
                ("luid_0x2_0x2_phys_0".into(), 200.0),
            ]);
            assert_eq!(values["00000001_00000001"], 100);
            assert_eq!(values["00000002_00000002"], 200);
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::GpuSnapshot;
    use std::ffi::{c_char, c_void, CStr};

    #[link(name = "Metal", kind = "framework")]
    extern "C" {
        fn MTLCopyAllDevices() -> *mut c_void;
    }
    #[link(name = "objc")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }
    type Id = *mut c_void;

    #[derive(Clone)]
    pub struct Adapter {
        id: String,
        name: String,
    }
    pub struct Provider;
    impl Provider {
        pub fn new() -> Self {
            Self
        }
        pub fn discover(&mut self) -> Vec<Adapter> {
            unsafe {
                let devices = MTLCopyAllDevices();
                if devices.is_null() {
                    return Vec::new();
                }
                let selector = |name: &'static [u8]| sel_registerName(name.as_ptr().cast());
                let send_usize: unsafe extern "C" fn(Id, Id) -> usize =
                    std::mem::transmute(objc_msgSend as *const ());
                let send_object: unsafe extern "C" fn(Id, Id, usize) -> Id =
                    std::mem::transmute(objc_msgSend as *const ());
                let send_id: unsafe extern "C" fn(Id, Id) -> Id =
                    std::mem::transmute(objc_msgSend as *const ());
                let send_u64: unsafe extern "C" fn(Id, Id) -> u64 =
                    std::mem::transmute(objc_msgSend as *const ());
                let mut adapters = Vec::new();
                let count = send_usize(devices, selector(b"count\0")).min(32);
                for index in 0..count {
                    let device = send_object(devices, selector(b"objectAtIndex:\0"), index);
                    if device.is_null() {
                        continue;
                    }
                    let registry = send_u64(device, selector(b"registryID\0"));
                    let ns_name = send_id(device, selector(b"name\0"));
                    if ns_name.is_null() {
                        continue;
                    }
                    let utf8 = send_id(ns_name, selector(b"UTF8String\0"));
                    if utf8.is_null() {
                        continue;
                    }
                    let name = CStr::from_ptr(utf8.cast()).to_string_lossy().into_owned();
                    adapters.push(Adapter {
                        id: format!("metal:{registry:016x}"),
                        name,
                    });
                }
                send_id(devices, selector(b"release\0"));
                adapters
            }
        }
        pub fn sample(&mut self, adapters: &[Adapter], now: f64) -> Vec<GpuSnapshot> {
            adapters.iter().map(|adapter| GpuSnapshot {
                id: adapter.id.clone(), name: adapter.name.clone(),
                utilization_percent: None, memory_used_bytes: None, memory_total_bytes: None,
                temperature_celsius: None, sampled_at: now, source: "Metal".into(),
                status: "unavailable".into(),
                message: Some("macOS Metal exposes adapter identity, not system-wide GPU usage, memory use, or temperature".into()),
            }).collect()
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    use super::GpuSnapshot;
    #[derive(Clone)]
    pub struct Adapter;
    pub struct Provider;
    impl Provider {
        pub fn new() -> Self {
            Self
        }
        pub fn discover(&mut self) -> Vec<Adapter> {
            Vec::new()
        }
        pub fn sample(&mut self, _: &[Adapter], _: f64) -> Vec<GpuSnapshot> {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_time_is_not_sampled() {
        assert!(GpuMonitor::new().refresh(f64::NAN).is_empty());
    }

    #[test]
    fn unsupported_readings_serialize_as_null() {
        let snapshot = GpuSnapshot {
            id: "metal:0001".into(),
            name: "Example GPU".into(),
            utilization_percent: None,
            memory_used_bytes: None,
            memory_total_bytes: None,
            temperature_celsius: None,
            sampled_at: 123.0,
            source: "Metal".into(),
            status: "unavailable".into(),
            message: Some("Unsupported".into()),
        };
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["utilizationPercent"], serde_json::Value::Null);
        assert_eq!(json["memoryUsedBytes"], serde_json::Value::Null);
        assert_eq!(json["temperatureCelsius"], serde_json::Value::Null);
        assert_eq!(json["sampledAt"], 123.0);
    }
}
