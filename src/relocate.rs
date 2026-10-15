//! Move a file or folder to another disk: copy, verify, trash the original,
//! optionally leave a symlink behind.
//!
//! The rules (`protected_reason`, `check`, `dest_path`) are pure so they can be
//! unit-tested; `inspect` gathers the facts from the filesystem and `relocate`
//! does the IO.

use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Folder created at the root of the destination volume. Everything we move
/// lands under it, mirroring its original absolute path.
pub const OFFLOAD_DIR: &str = "MacDirStat Offload";

/// Kept free on the destination on top of the estimated size.
pub const FREE_MARGIN: u64 = 1 << 30;

/// Package / bundle extensions that FAT-family filesystems (exFAT, msdos)
/// can't hold faithfully — they lose permissions, symlinks and metadata.
const PACKAGE_EXTS: &[&str] = &["app", "photoslibrary", "bundle", "framework", "pkg"];

/// `/<volume>/MacDirStat Offload/<original absolute path>`.
pub fn dest_path(volume: &Path, src: &Path) -> PathBuf {
    let rel = src.strip_prefix("/").unwrap_or(src);
    volume.join(OFFLOAD_DIR).join(rel)
}

/// Why `path` must never be moved or trashed, if it is protected.
pub fn protected_reason(path: &Path, home: Option<&Path>) -> Option<String> {
    if path == Path::new("/") {
        return Some("refusing to touch the root of the disk".into());
    }
    // `/tmp` and `/var` are symlinks into `/private`; callers normalize first.
    const SYSTEM_TREES: &[&str] = &[
        "/System", "/usr", "/bin", "/sbin", "/private", "/dev", "/etc", "/cores",
    ];
    // ponytail: per-user temp space is the one part of /private that holds
    // user files; scratch folders there are fair game.
    const USER_TEMP: &[&str] = &["/private/tmp", "/private/var/folders"];
    let user_temp = USER_TEMP
        .iter()
        .any(|t| path.starts_with(t) && path != Path::new(t));
    if let Some(root) = SYSTEM_TREES
        .iter()
        .find(|r| !user_temp && path.starts_with(r))
    {
        return Some(format!(
            "{} is a macOS system path (under {root})",
            path.display()
        ));
    }
    for exact in ["/Library", "/Applications", "/Users", "/Volumes", "/opt"] {
        if path == Path::new(exact) {
            return Some(format!(
                "{exact} itself is a system folder; pick something inside it"
            ));
        }
    }
    if let Some(home) = home {
        if path == home {
            return Some("refusing to move your whole home folder".into());
        }
        if path == home.join("Library") {
            return Some("refusing to move ~/Library itself; pick something inside it".into());
        }
    }
    None
}

/// Filesystem facts for the volume holding a path (via `statfs`).
#[derive(Debug, Clone)]
pub struct FsInfo {
    pub fs_type: String,
    pub mount: PathBuf,
    pub total: u64,
    pub free: u64,
    pub block_size: u64,
}

impl FsInfo {
    /// exFAT / FAT32: no symlinks, no Unix permissions, no packages.
    pub fn is_fat(&self) -> bool {
        matches!(self.fs_type.as_str(), "exfat" | "msdos")
    }
}

pub fn fs_info(path: &Path) -> Option<FsInfo> {
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut s: libc::statfs = std::mem::zeroed();
        if libc::statfs(c.as_ptr(), &mut s) != 0 {
            return None;
        }
        let bsize = s.f_bsize as u64;
        Some(FsInfo {
            fs_type: CStr::from_ptr(s.f_fstypename.as_ptr())
                .to_string_lossy()
                .into_owned(),
            mount: PathBuf::from(std::ffi::OsStr::from_bytes(
                CStr::from_ptr(s.f_mntonname.as_ptr()).to_bytes(),
            )),
            total: s.f_blocks * bsize,
            free: s.f_bavail * bsize,
            block_size: bsize,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub name: String,
    pub path: PathBuf,
    pub fs: FsInfo,
    /// The startup disk (`/`) — never a move destination.
    pub boot: bool,
}

/// The startup disk plus every volume mounted directly under `/Volumes`.
pub fn list_volumes() -> Vec<Volume> {
    let mut out = Vec::new();
    if let Some(fs) = fs_info(Path::new("/")) {
        out.push(Volume {
            name: "Startup disk".into(),
            path: "/".into(),
            fs,
            boot: true,
        });
    }
    let Ok(rd) = std::fs::read_dir("/Volumes") else {
        return out;
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let Some(fs) = fs_info(&path) else { continue };
        // `Macintosh HD` is a symlink to `/`; non-mount folders are leftovers.
        if fs.mount != path {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(Volume {
            name,
            path,
            fs,
            boot: false,
        });
    }
    out
}

/// What a source contains, gathered in one walk (symlinks are not followed).
#[derive(Debug, Clone, Default)]
pub struct Measure {
    pub bytes: u64,
    pub files: u64,
    /// Space the files take with clusters of `4 KiB << k`, k = 0..=8 (4K–1M).
    pub alloc: [u64; 9],
    pub symlinks: u64,
    pub first_symlink: Option<PathBuf>,
    pub first_package: Option<PathBuf>,
    pub unreadable: u64,
}

impl Measure {
    /// Estimated space on a filesystem with `cluster`-byte allocation units.
    pub fn alloc_for(&self, cluster: u64) -> u64 {
        let mut k = 0;
        while k < 8 && (4096u64 << k) < cluster {
            k += 1;
        }
        self.alloc[k]
    }
}

fn is_package(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| PACKAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Walks `path`. With `skip_appledouble`, `._*` files are ignored — ditto
/// writes those on FAT disks to hold extended attributes.
pub fn measure(path: &Path, skip_appledouble: bool) -> Measure {
    let mut m = Measure::default();
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(md) = std::fs::symlink_metadata(&p) else {
            m.unreadable += 1;
            continue;
        };
        let ft = md.file_type();
        if ft.is_symlink() {
            m.symlinks += 1;
            m.first_symlink.get_or_insert(p);
        } else if ft.is_dir() {
            if m.first_package.is_none() && is_package(&p) {
                m.first_package = Some(p.clone());
            }
            match std::fs::read_dir(&p) {
                Ok(rd) => stack.extend(rd.flatten().map(|e| e.path())),
                Err(_) => m.unreadable += 1,
            }
        } else {
            if skip_appledouble
                && p.file_name()
                    .is_some_and(|n| n.as_bytes().starts_with(b"._"))
            {
                continue;
            }
            let len = md.len();
            m.bytes += len;
            m.files += 1;
            for (k, slot) in m.alloc.iter_mut().enumerate() {
                let c = 4096u64 << k;
                *slot += len.div_ceil(c) * c;
            }
        }
    }
    m
}

/// Facts `check` decides on. Built by `inspect`; built by hand in tests.
pub struct Inputs<'a> {
    pub src: &'a Path,
    pub home: Option<&'a Path>,
    pub src_is_symlink: bool,
    pub src_dev: u64,
    pub volume: &'a Path,
    pub volume_dev: u64,
    /// Destination filesystem, `None` if `volume` is not a mounted volume.
    pub dest_fs: Option<&'a FsInfo>,
    pub dest_exists: bool,
    pub m: &'a Measure,
}

#[derive(Debug, Clone)]
pub struct Check {
    pub dest: PathBuf,
    /// Bytes needed on the destination (cluster-rounded estimate + margin).
    pub need: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Check {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn check(i: &Inputs) -> Check {
    let dest = dest_path(i.volume, i.src);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let cluster = i.dest_fs.map_or(4096, |f| f.block_size);
    let need = i.m.alloc_for(cluster) + FREE_MARGIN;

    if let Some(r) = protected_reason(i.src, i.home) {
        errors.push(r);
    }
    if i.src_is_symlink {
        errors.push("the item is itself a symlink; move what it points to instead".into());
    }
    if i.m.unreadable > 0 {
        errors.push(format!(
            "{} entries could not be read (permissions?)",
            i.m.unreadable
        ));
    }
    if i.volume.starts_with(i.src) || dest.starts_with(i.src) {
        errors.push("the destination is inside the item being moved".into());
    }
    match i.dest_fs {
        None => errors.push(format!("{} is not a mounted volume", i.volume.display())),
        Some(fs) => {
            if fs.mount != i.volume {
                errors.push(format!(
                    "{} is not the root of a mounted volume",
                    i.volume.display()
                ));
            } else if fs.mount == Path::new("/") || i.src_dev == i.volume_dev {
                errors
                    .push("the destination is on the same disk; moving would free nothing".into());
            }
            if fs.free < need {
                errors.push(format!(
                    "not enough free space: needs {} (incl. 1 GiB margin), {} free",
                    crate::ui::theme::format_size(need),
                    crate::ui::theme::format_size(fs.free)
                ));
            }
            if fs.is_fat() {
                if let Some(p) = &i.m.first_package {
                    errors.push(format!(
                        "{} is an app bundle or macOS package, which {} cannot store",
                        p.display(),
                        fs.fs_type
                    ));
                }
                if i.m.symlinks > 0 && !i.src_is_symlink {
                    errors.push(format!(
                        "contains {} symlink(s) (first: {}), which {} cannot store",
                        i.m.symlinks,
                        i.m.first_symlink
                            .as_deref()
                            .unwrap_or(Path::new("?"))
                            .display(),
                        fs.fs_type
                    ));
                }
                warnings.push(format!(
                    "{} keeps no Unix permissions or owners; extended attributes become ._ files",
                    fs.fs_type
                ));
                if let Some(small) = i.m.alloc_for(cluster).checked_sub(i.m.bytes) {
                    if small > i.m.bytes / 4 && small > 64 << 20 {
                        warnings.push(format!(
                            "many small files: takes about {} on the destination ({} KiB clusters)",
                            crate::ui::theme::format_size(i.m.alloc_for(cluster)),
                            cluster / 1024
                        ));
                    }
                }
            }
        }
    }
    if i.dest_exists {
        errors.push(format!(
            "{} already exists; refusing to overwrite",
            dest.display()
        ));
    }
    Check {
        dest,
        need,
        errors,
        warnings,
    }
}

/// Absolute path with symlinks resolved in the parent but not in the last
/// component (so moving a symlink is detected, not silently followed).
pub fn normalize(src: &Path) -> Result<PathBuf, String> {
    if !src.is_absolute() {
        return Err(format!("{} is not an absolute path", src.display()));
    }
    let (Some(parent), Some(name)) = (src.parent(), src.file_name()) else {
        return Err(format!("{} has no parent folder", src.display()));
    };
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("{}: {e}", src.display()))?;
    Ok(parent.join(name))
}

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Gathers the facts for moving `src` to `volume` and runs `check`.
pub fn inspect(src: &Path, volume: &Path) -> Result<(Check, Measure), String> {
    let norm = normalize(src)?;
    let fat = fs_info(volume).is_some_and(|f| f.is_fat());
    // Never walk a protected tree (e.g. all of /System) just to refuse it.
    let m = if protected_reason(&norm, home().as_deref()).is_some() {
        Measure::default()
    } else {
        measure(&norm, fat)
    };
    Ok((check_with(src, volume, &m)?, m))
}

/// `check` with an already-taken `Measure` (the GUI measures once, then
/// re-checks as the user picks a volume).
pub fn check_with(src: &Path, volume: &Path, m: &Measure) -> Result<Check, String> {
    let src = normalize(src)?;
    let md = std::fs::symlink_metadata(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    let dest_fs = fs_info(volume);
    let home = home();
    Ok(check(&Inputs {
        src: &src,
        home: home.as_deref(),
        src_is_symlink: md.file_type().is_symlink(),
        src_dev: md.dev(),
        volume,
        volume_dev: std::fs::metadata(volume).map(|m| m.dev()).unwrap_or(0),
        dest_fs: dest_fs.as_ref(),
        dest_exists: std::fs::symlink_metadata(dest_path(volume, &src)).is_ok(),
        m,
    }))
}

/// The copy holds the same number of files and bytes as the source.
pub fn verify(src: &Measure, dest: &Measure) -> Result<(), String> {
    if dest.unreadable > 0 {
        return Err(format!(
            "{} copied entries could not be read back",
            dest.unreadable
        ));
    }
    if (src.files, src.bytes) != (dest.files, dest.bytes) {
        return Err(format!(
            "copy mismatch: source {} files / {} bytes, copy {} files / {} bytes",
            src.files, src.bytes, dest.files, dest.bytes
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub dest: PathBuf,
    pub bytes: u64,
    pub files: u64,
    pub linked: bool,
}

/// Copy → verify → trash the original → optional symlink. On a failed copy
/// or verify the partial copy is removed and the source is left untouched.
pub fn relocate(src: &Path, volume: &Path, link: bool) -> Result<Outcome, String> {
    let (c, m) = inspect(src, volume)?;
    if !c.ok() {
        return Err(c.errors.join("; "));
    }
    let src = normalize(src)?;
    let dest = c.dest.clone();
    let parent = dest.parent().ok_or("bad destination")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;

    let copied = std::process::Command::new("/usr/bin/ditto")
        .arg(&src)
        .arg(&dest)
        .output()
        .map_err(|e| format!("ditto failed to start: {e}"))
        .and_then(|o| {
            if o.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "ditto failed: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                ))
            }
        });
    let fat = fs_info(volume).is_some_and(|f| f.is_fat());
    if let Err(e) = copied.and_then(|_| verify(&m, &measure(&dest, fat))) {
        remove_partial(&dest, volume);
        return Err(e);
    }

    crate::platform::trash::move_to_trash(&src).map_err(|e| {
        format!(
            "copied and verified to {}, but the original was not trashed: {e}",
            dest.display()
        )
    })?;
    if link {
        std::os::unix::fs::symlink(&dest, &src).map_err(|e| {
            format!(
                "moved to {}, but the link could not be created: {e}",
                dest.display()
            )
        })?;
    }
    Ok(Outcome {
        dest,
        bytes: m.bytes,
        files: m.files,
        linked: link,
    })
}

/// Deletes a failed copy (we created it: `check` refuses existing
/// destinations) and any now-empty folders up to the volume root, including
/// the `._name` AppleDouble files FAT disks keep beside them.
fn remove_partial(dest: &Path, volume: &Path) {
    let appledouble = |p: &Path| {
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        let _ = std::fs::remove_file(p.with_file_name(format!("._{name}")));
    };
    let _ = std::fs::remove_dir_all(dest).or_else(|_| std::fs::remove_file(dest));
    appledouble(dest);
    let mut cur = dest.parent();
    while let Some(p) = cur {
        if p == volume || std::fs::remove_dir(p).is_err() {
            break;
        }
        appledouble(p);
        cur = p.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(fs_type: &str, mount: &str, free: u64) -> FsInfo {
        FsInfo {
            fs_type: fs_type.into(),
            mount: mount.into(),
            total: free * 2,
            free,
            block_size: 131072,
        }
    }

    fn inputs<'a>(src: &'a Path, f: Option<&'a FsInfo>, m: &'a Measure) -> Inputs<'a> {
        Inputs {
            src,
            home: Some(Path::new("/Users/me")),
            src_is_symlink: false,
            src_dev: 1,
            volume: Path::new("/Volumes/Ext"),
            volume_dev: 2,
            dest_fs: f,
            dest_exists: false,
            m,
        }
    }

    fn small() -> Measure {
        Measure {
            bytes: 10,
            files: 1,
            alloc: [4096; 9],
            ..Default::default()
        }
    }

    #[test]
    fn mirrors_absolute_path_under_offload_dir() {
        assert_eq!(
            dest_path(
                Path::new("/Volumes/Ext"),
                Path::new("/Users/me/Movies/a.mov")
            ),
            PathBuf::from("/Volumes/Ext/MacDirStat Offload/Users/me/Movies/a.mov")
        );
    }

    #[test]
    fn protected_paths_are_refused() {
        let home = Some(Path::new("/Users/me"));
        for p in [
            "/",
            "/System/Library",
            "/usr/local/bin",
            "/private/var",
            "/private/tmp",
            "/private/var/folders",
            "/Library",
            "/Users/me",
            "/Users/me/Library",
            "/Applications",
        ] {
            assert!(
                protected_reason(Path::new(p), home).is_some(),
                "{p} should be protected"
            );
        }
        for p in [
            "/Users/me/Movies",
            "/Users/me/Library/Caches",
            "/Library/Caches",
            "/usrx",
            "/Applications/Foo.app",
            "/private/tmp/scratch",
            "/private/var/folders/x/T/y",
        ] {
            assert!(
                protected_reason(Path::new(p), home).is_none(),
                "{p} should be allowed"
            );
        }
    }

    #[test]
    fn clean_move_passes() {
        let f = fs("apfs", "/Volumes/Ext", 100 << 30);
        let m = small();
        let c = check(&inputs(Path::new("/Users/me/Movies"), Some(&f), &m));
        assert!(c.ok(), "{:?}", c.errors);
        assert_eq!(c.need, 4096 + FREE_MARGIN);
    }

    #[test]
    fn refuses_same_volume_and_non_mount() {
        let m = small();
        let f = fs("apfs", "/Volumes/Ext", 100 << 30);
        let mut i = inputs(Path::new("/Users/me/x"), Some(&f), &m);
        i.volume_dev = 1;
        assert!(check(&i).errors.iter().any(|e| e.contains("same disk")));

        let root = fs("apfs", "/", 100 << 30);
        let i = inputs(Path::new("/Users/me/x"), Some(&root), &m);
        assert!(check(&i).errors.iter().any(|e| e.contains("not the root")));
        let i = inputs(Path::new("/Users/me/x"), None, &m);
        assert!(!check(&i).ok());
    }

    #[test]
    fn refuses_destination_inside_source() {
        let m = small();
        let f = fs("apfs", "/Volumes/Ext", 100 << 30);
        let mut i = inputs(Path::new("/Volumes"), Some(&f), &m);
        i.home = None;
        assert!(check(&i).errors.iter().any(|e| e.contains("inside")));
    }

    #[test]
    fn refuses_when_space_short_counting_clusters() {
        // One byte per file, 128 KiB clusters: 1000 files need 125 MiB + margin.
        let m = Measure {
            bytes: 1000,
            files: 1000,
            alloc: [1000 * 131072; 9],
            ..Default::default()
        };
        let f = fs("exfat", "/Volumes/Ext", FREE_MARGIN + 1000 * 131072 - 1);
        let c = check(&inputs(Path::new("/Users/me/x"), Some(&f), &m));
        assert!(
            c.errors.iter().any(|e| e.contains("free space")),
            "{:?}",
            c.errors
        );
    }

    #[test]
    fn fat_refuses_packages_and_symlinks_and_warns() {
        let f = fs("exfat", "/Volumes/Ext", 100 << 30);
        let mut m = small();
        m.first_package = Some("/Users/me/x/Foo.app".into());
        m.symlinks = 2;
        m.first_symlink = Some("/Users/me/x/link".into());
        let c = check(&inputs(Path::new("/Users/me/x"), Some(&f), &m));
        assert!(c.errors.iter().any(|e| e.contains("package")));
        assert!(c.errors.iter().any(|e| e.contains("symlink")));
        assert!(c.warnings.iter().any(|w| w.contains("permissions")));

        // Same content to an APFS disk is fine.
        let apfs = fs("apfs", "/Volumes/Ext", 100 << 30);
        assert!(check(&inputs(Path::new("/Users/me/x"), Some(&apfs), &m)).ok());
    }

    #[test]
    fn refuses_symlink_source_and_existing_dest() {
        let f = fs("apfs", "/Volumes/Ext", 100 << 30);
        let m = small();
        let mut i = inputs(Path::new("/Users/me/x"), Some(&f), &m);
        i.src_is_symlink = true;
        i.dest_exists = true;
        let c = check(&i);
        assert!(c.errors.iter().any(|e| e.contains("symlink")));
        assert!(c.errors.iter().any(|e| e.contains("already exists")));
    }

    #[test]
    fn alloc_for_picks_cluster_bucket() {
        let m = Measure {
            alloc: [1, 2, 3, 4, 5, 6, 7, 8, 9],
            ..Default::default()
        };
        assert_eq!(m.alloc_for(4096), 1);
        assert_eq!(m.alloc_for(131072), 6);
        assert_eq!(m.alloc_for(1 << 24), 9);
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mds-reloc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn measure_and_verify_detect_mismatch() {
        let d = scratch("verify");
        std::fs::create_dir_all(d.join("a/b")).unwrap();
        std::fs::write(d.join("a/one"), vec![0u8; 5000]).unwrap();
        std::fs::write(d.join("a/b/two"), b"hi").unwrap();
        std::fs::write(d.join("a/._two"), b"appledouble").unwrap();
        std::os::unix::fs::symlink("one", d.join("a/ln")).unwrap();

        let m = measure(&d.join("a"), true);
        assert_eq!((m.files, m.bytes, m.symlinks), (2, 5002, 1));
        assert_eq!(m.alloc_for(4096), 8192 + 4096);
        assert_eq!(measure(&d.join("a"), false).files, 3);

        let copy = d.join("copy");
        let ok = std::process::Command::new("/usr/bin/ditto")
            .arg(d.join("a"))
            .arg(&copy)
            .status()
            .unwrap();
        assert!(ok.success());
        assert!(verify(&m, &measure(&copy, true)).is_ok());
        std::fs::write(copy.join("b/two"), b"h").unwrap();
        assert!(verify(&m, &measure(&copy, true)).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn remove_partial_cleans_empty_parents_only() {
        let vol = scratch("partial");
        std::fs::create_dir_all(vol.join("MacDirStat Offload/keep")).unwrap();
        std::fs::write(vol.join("MacDirStat Offload/keep/f"), b"x").unwrap();
        let dest = vol.join("MacDirStat Offload/Users/me/x");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("f"), b"x").unwrap();
        // exFAT: ditto leaves AppleDouble `._name` files next to what it copied.
        for ad in ["MacDirStat Offload/._Users", "MacDirStat Offload/Users/._me", "MacDirStat Offload/Users/me/._x"] {
            std::fs::write(vol.join(ad), b"ad").unwrap();
        }
        remove_partial(&dest, &vol);
        assert!(!vol.join("MacDirStat Offload/Users").exists());
        assert!(!vol.join("MacDirStat Offload/._Users").exists());
        assert!(vol.join("MacDirStat Offload/keep/f").exists());
        std::fs::remove_dir_all(&vol).unwrap();
    }
}
