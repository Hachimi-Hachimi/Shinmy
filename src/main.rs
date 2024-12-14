#![windows_subsystem = "windows"]

mod utils;

use std::{path::{Path, PathBuf}, str::FromStr, sync::mpsc::Sender};

use notify::{Result, Watcher};
use rand::Rng;
use windows::{core::{w, PCWSTR}, Win32::{Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, MAX_PATH}, Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT}, System::{LibraryLoader::GetModuleFileNameW, Threading::{CreateMutexW, ReleaseMutex}}}};

fn get_dll_paths(game_path: &Path, dll_name: &str) -> (PathBuf, PathBuf) {
    (
        game_path.join(format!("hachimi\\{}", dll_name)),
        game_path.join(format!("umamusume_Data\\Plugins\\x86_64\\{}", dll_name))
    )
}

fn create_watcher(
    mallet_dll: PathBuf, mallet_backup: PathBuf, dest_dll: PathBuf, src_dll: PathBuf, timeout_reset: Sender<()>
) -> Result<(notify::ReadDirectoryChangesWatcher, std::thread::JoinHandle<()>)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = notify::recommended_watcher(tx)?;

    let mallet_dll_lower = mallet_dll.to_string_lossy().to_ascii_lowercase();
    let src_dll_lower = src_dll.to_string_lossy().to_ascii_lowercase();
    let handle = std::thread::spawn(move || {
        while let Ok(res) = rx.recv() {
            let Ok(event) = res else { continue };
            let Some(path) = event.paths.get(0) else { continue };
            let path_lower = path.to_string_lossy().to_ascii_lowercase();

            if event.kind.is_remove() && path_lower == mallet_dll_lower {
                _ = timeout_reset.send(());
                _ = std::fs::copy(&mallet_backup, &mallet_dll);
            }
            else if event.kind.is_modify() && path_lower == src_dll_lower {
                _ = timeout_reset.send(());
                // wait a lil bit so dmm finishes its checks
                std::thread::sleep(std::time::Duration::from_secs(3));
                move_target_dll(&dest_dll, &src_dll);
            }
        }
    });

    Ok((watcher, handle))
}

fn move_target_dll(dest_dll: &Path, src_dll: &Path) {
    if std::fs::create_dir_all(dest_dll.parent().unwrap()).is_ok() && std::fs::copy(&src_dll, &dest_dll).is_ok() {
        _ = std::fs::remove_file(&src_dll);
    }
}

const TIMEOUT_COUNT: u32 = 30;
fn main() -> Result<()> {
    unsafe {
        let mut exec_path = [0u16; MAX_PATH as usize];
        GetModuleFileNameW(None, &mut exec_path);
        _ = MoveFileExW(PCWSTR(exec_path.as_ptr()), None, MOVEFILE_DELAY_UNTIL_REBOOT);
    }

    let Ok(hmutex) = (unsafe { CreateMutexW(None, true, w!("shinmy-fd89fa")) }) else {
        return Ok(())
    };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Ok(())
    }

    let mallet_dll = PathBuf::from_str(&std::env::args().skip(1).next().unwrap()).unwrap();
    let game_path = utils::detect_game_install_dir().unwrap().canonicalize().unwrap();
    let (dest_dll, src_dll) = get_dll_paths(&game_path, "cri_mana_vpx.dll");

    move_target_dll(&dest_dll, &src_dll);

    let mallet_backup_rnd: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    let mallet_backup = std::env::current_exe().unwrap()
        .parent().unwrap()
        .join(format!("{}-{}.dll", mallet_dll.file_stem().unwrap().to_str().unwrap(), mallet_backup_rnd));
    std::fs::copy(&mallet_dll, &mallet_backup).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();

    let (mut watcher, handle) = create_watcher(mallet_dll.clone(), mallet_backup.clone(), dest_dll, src_dll.clone(), tx)?;
    watcher.watch(&src_dll.parent().unwrap(), notify::RecursiveMode::NonRecursive)?;
    watcher.watch(&mallet_dll.parent().unwrap(), notify::RecursiveMode::NonRecursive)?;

    let mut timeout = TIMEOUT_COUNT;
    while timeout != 0 {
        if !utils::is_dmm_running() {
            timeout -= 1;
        }
        else if timeout != TIMEOUT_COUNT {
            timeout = TIMEOUT_COUNT;
        }

        if rx.try_recv().is_ok() {
            timeout = TIMEOUT_COUNT;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    _ = std::fs::remove_file(&mallet_backup);

    std::mem::drop(watcher);
    _ = handle.join();

    unsafe {
        _ = ReleaseMutex(hmutex);
        _ = CloseHandle(hmutex);
    };
    Ok(())
}
