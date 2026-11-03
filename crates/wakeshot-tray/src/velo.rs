//! The Velopack wiring, the only file that names the `velopack` crate (architecture section 4, tray startup step 1).
//!
//! Owner: ship track (`p6-velopack-dependency`), which keeps this signature; then `p6-int-update-wiring`; fixes by the
//! e2e cards.

use std::ffi::OsString;
#[cfg(debug_assertions)]
use std::path::PathBuf;
use std::sync::OnceLock;

use velopack::locator::{self, LocationContext};
#[cfg(debug_assertions)]
use velopack::sources::FileSource;
use velopack::sources::{GithubSource, UpdateSource};
use velopack::{UpdateCheck, UpdateManager, VelopackApp, VelopackAsset};
use wakeshot_core::command::Failure;
use wakeshot_windows::app::tray_mode::{MakeFeed, Updates};
use wakeshot_windows::hooks::{self, RealHookEnv};
use wakeshot_windows::update::{FeedError, UpdateFeed};

/// Where users' updates come from. No channel is named: an installed app reads the one it was packed with, so x64 and
/// ARM64 each get their own (`docs/research/distribution.md`, "x64 and ARM64").
const REPO_URL: &str = "https://github.com/HaikAsatryan/wakeshot";

/// `false`: a version with a pre-release part (`X.Y.Z-rc.N`) is a GitHub pre-release for testing, never for users
/// (`docs/phases/06-ship.md`, "Release").
const PRERELEASES: bool = false;

/// Debug builds only: a folder that stands in for GitHub, for the update path test
/// (`docs/phases/06-integration.md` note 8). A release build holds neither this name nor the code that reads it, so a
/// shipped exe updates from GitHub alone.
#[cfg(debug_assertions)]
const LOCAL_FEED: &str = "WAKESHOT_UPDATE_FEED";

/// The version Velopack restarted the tray with after it applied an update. Set inside `startup`, since `run()`
/// removes `VELOPACK_RESTART` before it returns, so nothing later can tell.
static RESTARTED: OnceLock<String> = OnceLock::new();

/// Runs `VelopackApp` with the hook callbacks; for a hook, Velopack runs it and exits the process here.
pub fn startup() {
    VelopackApp::build()
        // A staged update applied here would skip the core's apply rules and kill every process started from the
        // install folder, a `wakeshot run` wrapper too; the update thread finds it again and lets the core decide.
        .set_auto_apply_on_startup(false)
        .on_restarted(|v| {
            // `run()` calls it at most once, so the set cannot fail.
            let _ = RESTARTED.set(v.to_string());
        })
        .on_after_install_fast_callback(|v| {
            hooks::on_install(&mut RealHookEnv::installed(), &v.to_string())
        })
        .on_before_update_fast_callback(|v| {
            hooks::on_obsolete(&mut RealHookEnv::installed(), &v.to_string())
        })
        .on_after_update_fast_callback(|v| {
            hooks::on_updated(&mut RealHookEnv::installed(), &v.to_string())
        })
        .on_before_uninstall_fast_callback(|v| {
            hooks::on_uninstall(&mut RealHookEnv::installed(), &v.to_string())
        })
        .run();
}

/// What tray mode gets, after `startup`: the feed when this exe is an install, and the version Velopack restarted it
/// with. Tray mode only: a dry run must never update a real install.
pub fn updates() -> Updates {
    let feed: Option<MakeFeed> = installed().then(|| {
        let make: MakeFeed = Box::new(|| -> Box<dyn UpdateFeed> { Box::new(feed()) });
        make
    });
    Updates {
        feed,
        restarted: RESTARTED.get().cloned(),
    }
}

/// Velopack finds an install around this exe: `Update.exe` in the parent folder and the manifest in `current`. Only
/// `NotInstalled` means none; any other error is a broken install, whose update checks then log that error.
fn installed() -> bool {
    !matches!(
        locator::auto_locate_app_manifest(LocationContext::FromCurrentExe),
        Err(velopack::Error::NotInstalled(_))
    )
}

/// The update feed. Called on the update thread, so the manager is made there and its file reads never slow the
/// tray's start.
pub fn feed() -> impl UpdateFeed {
    let source = Source::pick(|name| std::env::var_os(name));
    VeloFeed {
        manager: UpdateManager::new_boxed(source.boxed(), None, None)
            .map_err(|e| feed_error("UpdateManager::new", &e)),
        kept: None,
    }
}

/// Where the feed reads releases from.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    Github,
    /// `WAKESHOT_UPDATE_FEED`: a folder with `releases.<channel>.json` and the packages (velopack `FileSource`).
    #[cfg(debug_assertions)]
    Folder(PathBuf),
}

impl Source {
    /// `env` reads a variable. A release build never calls it; an empty value counts as unset, since it would name
    /// the working folder.
    fn pick(env: impl Fn(&str) -> Option<OsString>) -> Self {
        #[cfg(debug_assertions)]
        if let Some(folder) = env(LOCAL_FEED).filter(|v| !v.is_empty()) {
            return Self::Folder(PathBuf::from(folder));
        }
        #[cfg(not(debug_assertions))]
        let _ = env;
        Self::Github
    }

    fn boxed(self) -> Box<dyn UpdateSource> {
        match self {
            Self::Github => Box::new(GithubSource::new(REPO_URL, None, PRERELEASES)),
            #[cfg(debug_assertions)]
            Self::Folder(folder) => Box::new(FileSource::new(folder)),
        }
    }
}

struct VeloFeed {
    /// `Err` once and for all: a build run from `target\` stays not installed.
    manager: Result<UpdateManager, FeedError>,
    /// The update the last good `check_and_download` downloaded, for `apply`.
    kept: Option<VelopackAsset>,
}

impl VeloFeed {
    fn manager(&self) -> Result<&UpdateManager, FeedError> {
        self.manager.as_ref().map_err(Clone::clone)
    }
}

impl UpdateFeed for VeloFeed {
    fn check_and_download(&mut self) -> Result<Option<String>, FeedError> {
        let manager = self.manager()?;
        let info = match manager
            .check_for_updates()
            .map_err(|e| feed_error("check_for_updates", &e))?
        {
            UpdateCheck::UpdateAvailable(info) => info,
            UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => return Ok(None),
        };
        // A package already staged by an earlier run is not downloaded again (velopack 1.2.161 `download_updates`).
        manager
            .download_updates(&info, None)
            .map_err(|e| feed_error("download_updates", &e))?;
        let version = info.TargetFullRelease.Version.clone();
        self.kept = Some(info.TargetFullRelease);
        Ok(Some(version))
    }

    fn apply(&mut self) -> Result<(), FeedError> {
        const CALL: &str = "wait_exit_then_apply_updates";
        let manager = self.manager()?;
        let Some(asset) = &self.kept else {
            return Err(FeedError::Failed {
                failure: Failure {
                    call: CALL,
                    code: 0,
                },
                detail: "no update downloaded yet".to_owned(),
            });
        };
        // Silent: the tray shows its own notice. Restart: the new tray starts at once, as the old one ran.
        manager
            .wait_exit_then_apply_updates(asset, true, true, std::iter::empty::<&str>())
            .map_err(|e| feed_error(CALL, &e))
    }
}

/// `NotInstalled` from any call means no install (research "Velopack checks for Wakeshot", point 5); the rest fail
/// that one call and are tried again at the next check.
fn feed_error(call: &'static str, e: &velopack::Error) -> FeedError {
    let code = match e {
        velopack::Error::NotInstalled(_) => return FeedError::NotInstalled,
        velopack::Error::Io(io) => io.raw_os_error().map_or(0, |c| c as u32),
        velopack::Error::Win32(w) => w.code().0 as u32,
        _ => 0,
    };
    FeedError::Failed {
        failure: Failure { call, code },
        detail: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_folder_exe_is_not_installed() {
        // The test exe sits in `target\`, with no `Update.exe` or package manifest above it.
        let mut feed = feed();
        assert_eq!(feed.check_and_download(), Err(FeedError::NotInstalled));
        assert_eq!(feed.apply(), Err(FeedError::NotInstalled));
    }

    #[test]
    fn a_build_folder_exe_gets_no_update_thread_and_no_updated_notice() {
        assert!(!installed());
        let updates = updates();
        assert!(updates.feed.is_none());
        assert_eq!(updates.restarted, None);
    }

    #[test]
    fn startup_returns_when_not_installed() {
        // A hook argument would run a real hook, which writes the real Run value and Path.
        assert!(
            !std::env::args().any(|a| a.starts_with("--veloapp-")),
            "never run with a hook argument"
        );
        startup();
        assert_eq!(RESTARTED.get(), None, "no install, so no restart");
    }

    // The variable is read through `pick`'s reader, not set with `std::env::set_var`: that is `unsafe` in edition
    // 2024, because other test threads read the environment at the same time (`feed()` does).
    fn local_feed(value: &'static str) -> impl Fn(&str) -> Option<OsString> {
        move |name| (name == "WAKESHOT_UPDATE_FEED").then(|| OsString::from(value))
    }

    #[test]
    fn with_the_local_feed_set_a_debug_build_reads_that_folder() {
        let picked = Source::pick(local_feed(r"C:\feed"));
        #[cfg(debug_assertions)]
        assert_eq!(picked, Source::Folder(PathBuf::from(r"C:\feed")));
        #[cfg(not(debug_assertions))]
        assert_eq!(picked, Source::Github, "a release build never reads it");
    }

    #[test]
    fn without_the_local_feed_the_feed_is_github() {
        assert_eq!(Source::pick(|_| None), Source::Github);
        assert_eq!(Source::pick(local_feed("")), Source::Github, "empty");
    }

    fn failure(e: FeedError) -> Failure {
        match e {
            FeedError::Failed { failure, .. } => failure,
            FeedError::NotInstalled => panic!("expected a failure, got NotInstalled"),
        }
    }

    #[test]
    fn not_installed_from_any_call_is_not_installed() {
        let e = velopack::Error::NotInstalled("no manifest".to_owned());
        assert_eq!(feed_error("check_for_updates", &e), FeedError::NotInstalled);
        assert_eq!(feed_error("download_updates", &e), FeedError::NotInstalled);
    }

    #[test]
    fn other_errors_keep_the_call_and_the_os_code() {
        let io = velopack::Error::Io(std::io::Error::from_raw_os_error(5));
        let got = failure(feed_error("download_updates", &io));
        assert_eq!(
            got,
            Failure {
                call: "download_updates",
                code: 5
            }
        );

        let hresult = windows::core::HRESULT(0x8007_0005_u32 as i32);
        let win32 = velopack::Error::Win32(windows::core::Error::from_hresult(hresult));
        let got = failure(feed_error("wait_exit_then_apply_updates", &win32));
        assert_eq!(got.code, 0x8007_0005);

        let other = velopack::Error::Other("feed is broken".to_owned());
        assert_eq!(
            feed_error("check_for_updates", &other),
            FeedError::Failed {
                failure: Failure {
                    call: "check_for_updates",
                    code: 0
                },
                detail: "feed is broken".to_owned(),
            }
        );
    }
}
