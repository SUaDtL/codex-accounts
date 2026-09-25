//! Child-process interruption, NOT physical power-loss or directory durability.
//! Four workers own distinct homes, vaults and children; no shared mutable fixture.
use super::*;
use std::{
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const CHILD: &str = "native::lifecycle::target_tests::restart::native_target_restart_child";
fn hex(id: Id) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
fn id(name: &str) -> Id {
    let value = std::env::var(name).unwrap();
    assert!(value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit()));
    let mut id = [0; 16];
    for (i, b) in id.iter_mut().enumerate() {
        *b = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).unwrap();
    }
    id
}
fn child(root: &Path, home: &Path, request: Request, mode: &str, point: usize) -> i32 {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", CHILD, "--nocapture", "--test-threads=1"])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    for name in [
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "WINDIR",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "LOCALAPPDATA",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("CA04C_RESTART_ROOT", root)
        .env("CA04C_RESTART_HOME", home)
        .env("CA04C_RESTART_MODE", mode)
        .env("CA04C_RESTART_POINT", point.to_string())
        .env("CA04C_SOURCE_PROFILE", hex(request.source.profile))
        .env("CA04C_SOURCE_GENERATION", hex(request.source.generation))
        .env("CA04C_TARGET_PROFILE", hex(request.target.profile))
        .env("CA04C_TARGET_GENERATION", hex(request.target.generation));
    let mut child = command
        .spawn()
        .expect("create only the owned synthetic test child");
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap_or(-1);
        }
        if start.elapsed() > Duration::from_secs(75) {
            // Watchdog cleanup owns Child's process handle. Never signal another PID.
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("synthetic restart child exceeded its execution bound");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn native_target_restart_child() {
    let Some(root) = std::env::var_os("CA04C_RESTART_ROOT") else {
        return;
    };
    let root_path = PathBuf::from(root);
    let home = PathBuf::from(std::env::var_os("CA04C_RESTART_HOME").unwrap());
    assert_eq!(root_path.file_name().unwrap(), "ca03c-test-vault");
    assert_eq!(home.file_name().unwrap(), "ca04c-synthetic-home");
    assert_eq!(home.parent(), root_path.parent());
    let mode = std::env::var("CA04C_RESTART_MODE").unwrap();
    assert!(matches!(mode.as_str(), "forward" | "restore" | "repair"));
    let point: usize = std::env::var("CA04C_RESTART_POINT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(point <= 64);
    let mut disk = Disk::at(&root_path, false).unwrap();
    let root = bootstrap(&mut disk, false).unwrap();
    let mut store = Storage::open(disk, &root).unwrap();
    let mut fx = NativeEffects {
        target: Target::open(&home, 7).unwrap(),
        home,
        helper: None,
    };
    let operation = if mode == "forward" {
        let request = Request {
            source: GenerationRef {
                profile: id("CA04C_SOURCE_PROFILE"),
                generation: id("CA04C_SOURCE_GENERATION"),
            },
            target: GenerationRef {
                profile: id("CA04C_TARGET_PROFILE"),
                generation: id("CA04C_TARGET_GENERATION"),
            },
            binding: BINDING,
            online: false,
        };
        store.begin_switch(&root, &mut fx, request).unwrap()
    } else {
        let status = store.switch_status().unwrap();
        assert_eq!(status.len(), 1);
        let operation = status[0].operation_id;
        if mode != "repair" {
            store
                .recover_switch(&root, &mut fx, operation, Choice::Restore)
                .unwrap();
        }
        operation
    };
    fx.target.fail_at = (point != 0).then_some(point);
    fx.target.crash = true;
    if mode == "repair" {
        store
            .repair_registered_staging(&root, &mut fx, operation)
            .unwrap();
        assert!(point == 0, "requested repair interruption was not executed");
        assert_eq!(store.recovery(), Recovery::SwitchPending);
        assert!((1..=64).contains(&fx.target.boundary));
        std::process::exit(100 + fx.target.boundary as i32);
    }
    for _ in 0..100 {
        let phase = store.switch_status().unwrap()[0].phase;
        if matches!(phase, SwitchPhase::Finished | SwitchPhase::Restored) {
            assert!(point == 0, "requested native boundary was not executed");
            assert!((1..=64).contains(&fx.target.boundary));
            // Only a bounded non-secret count crosses the child boundary.
            std::process::exit(100 + fx.target.boundary as i32);
        }
        store.advance_switch(&root, &mut fx, operation).unwrap();
    }
    panic!("native restart scenario did not terminate");
}
fn run_case(mode: &str, point: usize) -> usize {
    let mut f = if mode == "repair" {
        repair::interrupted().0
    } else {
        Fixture::new(1, 2)
    };
    let request = f.request();
    if mode == "restore" {
        let operation = f.begin();
        f.until(operation, SwitchPhase::TargetInstalled);
        f.store
            .cancel_switch(&f.root, &mut f.fx, operation)
            .unwrap();
    }
    let Fixture {
        store,
        fx,
        root,
        a,
        b: _,
        home,
        dir,
    } = f;
    drop(store);
    drop(fx);
    drop(root);
    let code = child(&dir.0, &home, request, mode, point);
    if point == 0 {
        assert!(
            (101..=164).contains(&code),
            "baseline child did not report executed boundaries"
        );
        return (code - 100) as usize;
    }
    assert_eq!(code, 86, "required native interruption did not occur");
    // Reopen the real protected key, encrypted control records, journals and files.
    let mut disk = Disk::at(&dir.0, false).unwrap();
    let root = bootstrap(&mut disk, false).unwrap();
    let mut store = Storage::open(disk, &root).unwrap();
    let mut fx = NativeEffects {
        target: Target::open(&home, 7).unwrap(),
        home,
        helper: None,
    };
    let status = store.switch_status().unwrap();
    assert_eq!(status.len(), 1);
    assert!(!matches!(
        status[0].phase,
        SwitchPhase::Finished | SwitchPhase::Restored
    ));
    assert_eq!(
        store
            .read_latest(&root, a)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(A1)
    );
    let operation = status[0].operation_id;
    if mode == "repair" {
        assert_eq!(plain(&fx.target), values(A1, 1));
        store
            .repair_registered_staging(&root, &mut fx, operation)
            .unwrap();
        assert_eq!(store.recovery(), Recovery::SwitchPending);
        assert_eq!(plain(&fx.target), values(A1, 1));
    }
    let mut outcome = store.recover_switch(&root, &mut fx, operation, Choice::Restore);
    for _ in 0..100 {
        if outcome.is_err()
            || matches!(
                store.switch_status().unwrap()[0].phase,
                SwitchPhase::Cancelled | SwitchPhase::Restored
            )
        {
            break;
        }
        outcome = store.advance_switch(&root, &mut fx, operation);
    }
    let torn = mode != "repair" && (point == 2 || (mode == "restore" && point == 9));
    if torn {
        assert!(
            outcome.is_err(),
            "torn unverified staging must not be silently discarded"
        );
        let status = store.switch_status().unwrap();
        assert_eq!(status[0].phase, SwitchPhase::Conflict);
        assert_eq!(status[0].restoration_error, Some("E_EXTERNAL_CHANGE"));
        let source = values(A1, 1);
        let target = values(B0, 2);
        for (i, bytes) in plain(&fx.target).iter().enumerate() {
            assert!(
                bytes == &source[i] || bytes == &target[i],
                "only recorded live resource states may remain"
            );
        }
        // Normal recovery still refuses unknown staging. The newly implemented
        // explicit archive/repair action is separate and never silently selected.
        store
            .repair_registered_staging(&root, &mut fx, operation)
            .unwrap();
        assert_eq!(store.recovery(), Recovery::SwitchPending);
        store
            .recover_switch(&root, &mut fx, operation, Choice::Restore)
            .unwrap();
        for _ in 0..100 {
            if matches!(
                store.switch_status().unwrap()[0].phase,
                SwitchPhase::Cancelled | SwitchPhase::Restored
            ) {
                break;
            }
            store.advance_switch(&root, &mut fx, operation).unwrap();
        }
        assert!(matches!(
            store.switch_status().unwrap()[0].phase,
            SwitchPhase::Cancelled | SwitchPhase::Restored
        ));
        assert_eq!(plain(&fx.target), values(A1, 1));
        assert_eq!(std::fs::read_dir(&fx.home).unwrap().count(), 2);
    } else {
        outcome.unwrap();
        assert!(matches!(
            store.switch_status().unwrap()[0].phase,
            SwitchPhase::Cancelled | SwitchPhase::Restored
        ));
        assert_eq!(plain(&fx.target), values(A1, 1));
        assert_eq!(std::fs::read_dir(&fx.home).unwrap().count(), 2);
    }
    if mode == "restore" {
        assert_eq!(
            store.switch_status().unwrap()[0].primary_error,
            Some("E_CANCELLED")
        );
    }
    // Explicit destruction order: all handles/children before the owned sandbox.
    drop(fx);
    drop(store);
    drop(root);
    drop(dir);
    usize::from(torn)
}
pub(super) fn exercise(mode: &'static str) {
    let boundaries = run_case(mode, 0);
    assert_eq!(
        boundaries,
        if mode == "repair" { 5 } else { 17 },
        "native boundary inventory changed; review and update proof"
    );
    let torn: usize = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|lane| {
                scope.spawn(move || {
                    (1..=boundaries)
                        .filter(|point| point % 4 == lane)
                        .map(|point| run_case(mode, point))
                        .sum::<usize>()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .sum()
    });
    assert_eq!(
        torn,
        match mode {
            "forward" => 1,
            "restore" => 2,
            "repair" => 0,
            _ => panic!("invalid test mode"),
        }
    );
}
