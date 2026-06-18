//! Flywheel Common Platform SDK — Rust port of `flywheel-client.ts`.
//!
//! One vendorable module that posts the shared event taxonomy to the shared
//! `flywheel-core` Supabase `events` table, so usage of this desktop app shows
//! up in the same portfolio BI as the web apps. Faithful to the TS client's
//! contract: cookieless, first-party, fire-and-forget, and a hard no-op when
//! unconfigured — analytics must never block, slow, or crash the app.
//!
//! Transport: a detached `curl` POST (macOS always ships curl), so we pull in
//! no HTTP/TLS crates. Identity: an anonymous per-install `visitor_id` (random
//! UUID persisted next to the app's other state) plus a per-process
//! `session_id`. There is no auth in this app, so `flywheel_uid` is always null.
//!
//! Configuration (any one source, checked in order):
//!   * runtime env `FLYWHEEL_SUPABASE_URL` + `FLYWHEEL_SUPABASE_ANON_KEY`
//!   * compile-time bake via the same names (`option_env!`) for release builds
//! Opt-out: `MACDIRSTAT_NO_TELEMETRY=1` in the environment, or the user toggle
//! persisted in `telemetry.txt`. With no config baked, the SDK ships dark.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// The shared event taxonomy (mirrors `TAXONOMY` in flywheel-client.ts).
pub const TAXONOMY: &[&str] = &[
    "page_view",
    "signup_started",
    "signup_completed",
    "conversion",
    "key_action",
    "feedback_given",
    "error",
];

#[derive(Clone)]
pub struct Flywheel {
    app: String,
    enabled: bool,
    url: String,
    key: String,
    visitor_id: String,
    session_id: String,
}

fn state_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support/mac-dir-stat"))
}

/// Resolve `(url, anon_key)` from runtime env first, then a compile-time bake.
/// Returns `None` when neither is present — the ships-dark default.
fn resolve_config() -> Option<(String, String)> {
    let url = std::env::var("FLYWHEEL_SUPABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("FLYWHEEL_SUPABASE_URL").map(str::to_string))
        .filter(|s| !s.is_empty())?;
    let key = std::env::var("FLYWHEEL_SUPABASE_ANON_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("FLYWHEEL_SUPABASE_ANON_KEY").map(str::to_string))
        .filter(|s| !s.is_empty())?;
    Some((url.trim_end_matches('/').to_string(), key))
}

/// True unless the user opted out via env or the persisted toggle.
pub fn telemetry_opted_out() -> bool {
    if matches!(
        std::env::var("MACDIRSTAT_NO_TELEMETRY").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    ) {
        return true;
    }
    state_dir()
        .map(|d| d.join("telemetry.txt"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "disabled")
        .unwrap_or(false)
}

/// Persist the user's telemetry choice so it survives restarts.
pub fn set_telemetry_opt_out(opted_out: bool) {
    let Some(dir) = state_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(
        dir.join("telemetry.txt"),
        if opted_out { "disabled" } else { "enabled" },
    );
}

/// 16 random bytes from `/dev/urandom`, or a time-seeded fallback.
fn random_16() -> [u8; 16] {
    use std::io::Read;
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        if f.read_exact(&mut buf).is_ok() {
            return buf;
        }
    }
    // Fallback: derive from the clock + address entropy. Never used in practice
    // on macOS, but keeps id generation infallible.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seed = nanos ^ ((&buf as *const _ as u128) << 1);
    buf.copy_from_slice(&seed.to_le_bytes());
    buf
}

/// Format 16 bytes as a RFC-4122 v4 UUID string.
fn uuid_v4() -> String {
    let mut b = random_16();
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // variant 1
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
    )
}

/// Stable anonymous per-install id, persisted next to the app's other state.
fn load_or_create_visitor_id() -> String {
    let Some(dir) = state_dir() else {
        return uuid_v4();
    };
    let path = dir.join("visitor_id");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let id = existing.trim();
        if !id.is_empty() {
            return id.to_string();
        }
    }
    let id = uuid_v4();
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(&path, &id);
    }
    id
}

/// Current UTC time as an RFC-3339 string (e.g. `2026-06-18T09:01:23Z`).
fn rfc3339_utc(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hh, mm, ss
    )
}

/// Unix seconds → (year, month, day, hour, min, sec) in UTC.
/// Uses Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hh = (rem / 3600) as u32;
    let mm = ((rem % 3600) / 60) as u32;
    let ss = (rem % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d, hh, mm, ss)
}

impl Flywheel {
    /// Build the client for `app`. Resolves config + opt-out once; if either
    /// fails the client is disabled and every method is a cheap no-op.
    pub fn init(app: &str) -> Self {
        let (url, key, enabled) = match resolve_config() {
            Some((u, k)) if !telemetry_opted_out() => (u, k, true),
            Some((u, k)) => (u, k, false),
            None => (String::new(), String::new(), false),
        };
        Flywheel {
            app: app.to_string(),
            enabled,
            url,
            key,
            visitor_id: if enabled {
                load_or_create_visitor_id()
            } else {
                String::new()
            },
            session_id: if enabled { uuid_v4() } else { String::new() },
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Re-evaluate enablement after the user flips the in-app toggle, without
    /// rebuilding the whole app. Lazily mints ids the first time it turns on.
    pub fn refresh_enabled(&mut self) {
        let configured = resolve_config().is_some();
        self.enabled = configured && !telemetry_opted_out();
        if self.enabled && self.visitor_id.is_empty() {
            if let Some((u, k)) = resolve_config() {
                self.url = u;
                self.key = k;
            }
            self.visitor_id = load_or_create_visitor_id();
            self.session_id = uuid_v4();
        }
    }

    /// Track a taxonomy event. `props` is any JSON object value.
    pub fn track(&self, event: &str, props: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let row = serde_json::json!({
            "app": self.app,
            "event": event,
            "props": props,
            "visitor_id": self.visitor_id,
            "session_id": self.session_id,
            "flywheel_uid": serde_json::Value::Null,
            "created_at": rfc3339_utc(SystemTime::now()),
        });
        self.post("events", &row.to_string());
    }

    /// The declarative conversion (the app's aha moment). Set
    /// `activation_event: "conversion"` in flywheel.json and the KPI cron counts it.
    pub fn conversion(&self, props: serde_json::Value) {
        self.track("conversion", props);
    }

    /// Mirror of `logError` — never throws, truncates the message like the TS client.
    pub fn log_error(&self, message: &str, mut context: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let truncated: String = message.chars().take(500).collect();
        if let Some(obj) = context.as_object_mut() {
            obj.insert("message".into(), serde_json::Value::String(truncated));
        } else {
            context = serde_json::json!({ "message": truncated });
        }
        self.track("error", context);
    }

    /// Sean Ellis PMF micro-survey → `feedback` table + a `feedback_given` event.
    pub fn feedback(&self, sean_ellis: Option<&str>, score: Option<u8>, text: Option<&str>) {
        if !self.enabled {
            return;
        }
        let row = serde_json::json!({
            "app": self.app,
            "sean_ellis": sean_ellis,
            "score": score,
            "text": text,
            "flywheel_uid": serde_json::Value::Null,
            "visitor_id": self.visitor_id,
            "created_at": rfc3339_utc(SystemTime::now()),
        });
        self.post("feedback", &row.to_string());
        self.track("feedback_given", serde_json::json!({ "sean_ellis": sean_ellis }));
    }

    /// Fire-and-forget POST to PostgREST. Detached `curl`; we never wait on it,
    /// so a slow or unreachable network can't stall a UI frame.
    fn post(&self, table: &str, body: &str) {
        let endpoint = format!("{}/rest/v1/{}", self.url, table);
        let _ = Command::new("curl")
            .args([
                "-s",
                "-m",
                "5",
                "-X",
                "POST",
                &endpoint,
                "-H",
                &format!("apikey: {}", self.key),
                "-H",
                &format!("Authorization: Bearer {}", self.key),
                "-H",
                "Content-Type: application/json",
                "-H",
                "Prefer: return=minimal",
                "-d",
                body,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// A process-wide disabled client, handy for tests and the unconfigured default.
pub fn disabled() -> &'static Flywheel {
    static DISABLED: OnceLock<Flywheel> = OnceLock::new();
    DISABLED.get_or_init(|| Flywheel {
        app: "mac-dir-stat".into(),
        enabled: false,
        url: String::new(),
        key: String::new(),
        visitor_id: String::new(),
        session_id: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v4_has_correct_shape_and_version() {
        let id = uuid_v4();
        assert_eq!(id.len(), 36);
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        // version nibble
        assert!(parts[2].starts_with('4'));
        // variant nibble is one of 8,9,a,b
        let v = parts[3].chars().next().unwrap();
        assert!(matches!(v, '8' | '9' | 'a' | 'b'));
        // hex only
        assert!(id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn uuids_are_unique() {
        let a = uuid_v4();
        let b = uuid_v4();
        assert_ne!(a, b);
    }

    #[test]
    fn rfc3339_epoch_is_correct() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_known_timestamp() {
        // 1_781_686_883 = 2026-06-17T09:01:23Z
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_781_686_883);
        assert_eq!(rfc3339_utc(t), "2026-06-17T09:01:23Z");
    }

    #[test]
    fn civil_handles_leap_day() {
        // 2024-02-29T12:00:00Z = 1_709_208_000
        let (y, m, d, hh, mm, ss) = civil_from_unix(1_709_208_000);
        assert_eq!((y, m, d, hh, mm, ss), (2024, 2, 29, 12, 0, 0));
    }

    #[test]
    fn disabled_client_is_noop() {
        let fw = disabled();
        assert!(!fw.is_enabled());
        // Must not panic or attempt any IO.
        fw.track("page_view", serde_json::json!({}));
        fw.conversion(serde_json::json!({ "freed_bytes": 123 }));
        fw.log_error("boom", serde_json::json!({ "where": "test" }));
        fw.feedback(Some("very"), Some(9), Some("love it"));
    }

    #[test]
    fn init_without_config_is_disabled() {
        // The test env has no FLYWHEEL_SUPABASE_* set.
        let fw = Flywheel::init("mac-dir-stat");
        assert!(!fw.is_enabled());
    }

    #[test]
    fn taxonomy_matches_ts_client() {
        assert!(TAXONOMY.contains(&"conversion"));
        assert!(TAXONOMY.contains(&"feedback_given"));
        assert!(TAXONOMY.contains(&"error"));
        assert_eq!(TAXONOMY.len(), 7);
    }
}
