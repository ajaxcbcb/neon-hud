use serde_json::Value;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct Screen {
    pub id: String,
    pub name: String,
    pub origin: [f64; 2],
    pub size: [f64; 2],
    pub scale: f64,
    pub primary: bool,
}

pub fn restore_position(saved: &Value, screens: &[Screen], size: [f64; 2]) -> Option<[f64; 2]> {
    let screen = screens
        .iter()
        .find(|s| s.id == text(saved, "monitor"))
        .or_else(|| screens.iter().find(|s| s.name == text(saved, "monitor")))
        .or_else(|| screens.iter().find(|s| s.primary))
        .or_else(|| screens.first())?;
    let x = number(saved, "x")? * screen.scale;
    let y = number(saved, "y")? * screen.scale;
    Some([
        screen.origin[0] + x.clamp(0., (screen.size[0] - size[0] * screen.scale).max(0.)),
        screen.origin[1] + y.clamp(0., (screen.size[1] - size[1] * screen.scale).max(0.)),
    ])
}

pub fn remember_position(point: [f64; 2], size: [f64; 2], screens: &[Screen]) -> Option<Value> {
    let overlap = |s: &Screen| {
        ((point[0] + size[0]).min(s.origin[0] + s.size[0]) - point[0].max(s.origin[0])).max(0.)
            * ((point[1] + size[1]).min(s.origin[1] + s.size[1]) - point[1].max(s.origin[1]))
                .max(0.)
    };
    let screen = screens
        .iter()
        .max_by(|a, b| overlap(a).total_cmp(&overlap(b)))?;
    let round = |n: f64| (n * 2.).round() / 2.;
    Some(serde_json::json!({
        "x": round((point[0] - screen.origin[0]) / screen.scale),
        "y": round((point[1] - screen.origin[1]) / screen.scale),
        "monitor": screen.id,
    }))
}

pub fn number(v: &Value, key: &str) -> Option<f64> {
    v[key].as_f64().filter(|n| n.is_finite())
}
pub fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn flag(v: &Value, key: &str) -> bool {
    v[key].as_bool().unwrap_or(false)
}
pub fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub fn bytes(n: f64) -> String {
    if n >= 1_073_741_824.0 {
        format!("{:.1} GB", n / 1_073_741_824.0)
    } else if n >= 1_048_576.0 {
        format!("{:.1} MB", n / 1_048_576.0)
    } else {
        format!("{:.0} KB", n / 1024.0)
    }
}
pub fn percent(n: Option<f64>) -> String {
    n.map(|x| format!("{x:.0}%")).unwrap_or_else(|| "—".into())
}
pub fn ratio(used: Option<f64>, total: Option<f64>) -> Option<f64> {
    used.zip(total)
        .filter(|(_, t)| *t > 0.0)
        .map(|(u, t)| (u / t * 100.0).clamp(0.0, 100.0))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Pressure,
    Critical,
}
impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Pressure => "pressure",
            Self::Critical => "critical",
        }
    }
    pub fn interval(self, requested: u64, hidden: bool) -> u64 {
        let n = match self {
            Self::Normal => requested.max(250),
            Self::Pressure => 4000,
            Self::Critical => 8000,
        };
        if hidden {
            n.max(8000)
        } else {
            n
        }
    }
}
pub fn resource_mode(system: &Value, settings: &Value) -> Mode {
    if !flag(&settings["resources"], "adaptive") {
        return Mode::Normal;
    }
    let cpu = number(system, "cpu").unwrap_or(0.0);
    let ram = ratio(
        number(&system["memory"], "used"),
        number(&system["memory"], "total"),
    )
    .unwrap_or(0.0);
    let hot = array(system, "temperatures")
        .iter()
        .filter_map(|v| number(v, "celsius"))
        .chain(
            array(system, "gpus")
                .iter()
                .filter_map(|v| number(v, "temperatureCelsius")),
        )
        .any(|t| t >= number(&settings["performance"], "temperatureCelsius").unwrap_or(85.0));
    if hot || ram >= 97.0 {
        Mode::Critical
    } else if cpu >= number(&settings["performance"], "cpuPercent").unwrap_or(90.0)
        || ram >= number(&settings["performance"], "memoryPercent").unwrap_or(90.0)
    {
        Mode::Pressure
    } else {
        Mode::Normal
    }
}

#[derive(Clone, Default)]
pub struct Drain {
    samples: HashMap<String, VecDeque<(f64, f64, Option<f64>)>>,
}
#[derive(Default, Debug)]
pub struct Rate {
    pub per_hour: Option<f64>,
    pub eta_minutes: Option<f64>,
    pub fast: bool,
}
impl Drain {
    pub fn observe(&mut self, usage: &Value, now: f64) {
        if text(usage, "state") != "connected" {
            return;
        }
        let Some(at) = number(usage, "fetchedAt").filter(|at| (0.0..=600.0).contains(&(now - at)))
        else {
            return;
        };
        for w in array(usage, "windows") {
            let Some(used) = number(w, "usedPercent").filter(|v| (0.0..=100.0).contains(v)) else {
                continue;
            };
            let reset = number(w, "resetsAt");
            if reset.is_some_and(|r| r <= now) {
                continue;
            }
            let key = format!("{}/{}", text(usage, "surface"), text(w, "label"));
            let list = self.samples.entry(key).or_default();
            if list.back().is_some_and(|(last, value, last_reset)| {
                at < *last || used < *value || reset != *last_reset
            }) {
                list.clear();
            }
            if list.back().is_some_and(|(last, _, _)| *last == at) {
                continue;
            }
            list.push_back((at, used, reset));
            while list.len() > 361 || (list.len() > 2 && list[1].0 < at - 1800.0) {
                list.pop_front();
            }
        }
    }
    pub fn rate(&self, usage: &Value, w: &Value, now: f64) -> Rate {
        if text(usage, "state") != "connected"
            || !number(usage, "fetchedAt").is_some_and(|t| (0.0..=600.0).contains(&(now - t)))
        {
            return Rate::default();
        }
        let key = format!("{}/{}", text(usage, "surface"), text(w, "label"));
        let Some(list) = self.samples.get(&key) else {
            return Rate::default();
        };
        let Some((first, a, _)) = list.front() else {
            return Rate::default();
        };
        let Some((last, b, reset)) = list.back() else {
            return Rate::default();
        };
        if last - first < 120.0 || reset.is_some_and(|r| r <= now) {
            return Rate::default();
        }
        let rate = (b - a).max(0.0) / (last - first) * 3600.0;
        let eta = (rate > 0.0).then(|| (100.0 - b) / rate * 60.0);
        Rate {
            per_hour: Some(rate),
            eta_minutes: eta,
            fast: eta.zip(*reset).is_some_and(|(m, r)| m * 60.0 < r - now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn placement_tracks_negative_monitor_origins_and_dpi() {
        let screens = vec![
            Screen {
                id: "display:1".into(),
                name: "Main".into(),
                origin: [0., 0.],
                size: [1920., 1080.],
                scale: 1.,
                primary: true,
            },
            Screen {
                id: "display:2".into(),
                name: "Left".into(),
                origin: [-3840., -200.],
                size: [3840., 2160.],
                scale: 2.,
                primary: false,
            },
        ];
        let saved = remember_position([-3440., 100.], [320., 112.], &screens).unwrap();
        assert_eq!(saved, json!({"x":200.,"y":150.,"monitor":"display:2"}));
        assert_eq!(
            restore_position(&saved, &screens, [160., 56.]),
            Some([-3440., 100.])
        );
        let changed = vec![Screen {
            id: "display:2".into(),
            name: "Left".into(),
            origin: [-1920., 0.],
            size: [1920., 1080.],
            scale: 1.,
            primary: true,
        }];
        assert_eq!(
            restore_position(&saved, &changed, [160., 56.]),
            Some([-1720., 150.])
        );
        let offscreen = json!({"x":9999.,"y":9999.,"monitor":"Unplugged"});
        assert_eq!(
            restore_position(&offscreen, &screens, [280., 56.]),
            Some([1640., 1024.])
        );
        let mut duplicates = screens.clone();
        duplicates[1].name = "Main".into();
        let saved = remember_position([-3440., 100.], [320., 112.], &duplicates).unwrap();
        assert_eq!(
            restore_position(&saved, &duplicates, [160., 56.]),
            Some([-3440., 100.])
        );
        let legacy = json!({"x":10.,"y":20.,"monitor":"Main"});
        assert_eq!(
            restore_position(&legacy, &screens, [160., 56.]),
            Some([10., 20.])
        );
    }
    fn usage(at: f64, used: f64, reset: f64) -> Value {
        json!({"surface":"codex","state":"connected","fetchedAt":at,"windows":[{"label":"5 hours","minutes":300,"usedPercent":used,"resetsAt":reset}]})
    }
    #[test]
    fn time_weighted_drain_resets_and_expires() {
        let mut d = Drain::default();
        d.observe(&usage(1000., 10., 19000.), 1000.);
        let u = usage(1600., 20., 19000.);
        d.observe(&u, 1600.);
        d.observe(&u, 1600.);
        let r = d.rate(&u, &u["windows"][0], 1600.);
        assert_eq!(r.per_hour, Some(60.));
        assert_eq!(r.eta_minutes, Some(80.));
        assert!(r.fast);
        assert!(d.rate(&u, &u["windows"][0], 2300.).per_hour.is_none());
        let u = usage(1700., 1., 37000.);
        d.observe(&u, 1700.);
        assert!(d.rate(&u, &u["windows"][0], 1700.).per_hour.is_none());
    }
    #[test]
    fn heat_and_memory_backoff_without_missing_data_fabrication() {
        let s = json!({"resources":{"adaptive":true},"performance":{"temperatureCelsius":85}});
        assert_eq!(resource_mode(&json!({}), &s), Mode::Normal);
        assert_eq!(
            resource_mode(&json!({"temperatures":[{"celsius":90}]}), &s),
            Mode::Critical
        );
        assert_eq!(Mode::Critical.interval(250, false), 8000);
        assert_eq!(Mode::Normal.interval(250, true), 8000);
        assert_eq!(percent(ratio(Some(12.), Some(0.))), "—");
    }
}
