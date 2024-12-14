pub mod proxy;
mod utils;

use std::{io::Write, os::{raw::{c_ulong, c_void}, windows::process::CommandExt}};

use rand::Rng;
use widestring::Utf16Str;
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::{BOOL, HMODULE, MAX_PATH},
        System::{LibraryLoader::GetModuleFileNameW, Threading::DETACHED_PROCESS},
        UI::WindowsAndMessaging::{MessageBoxW, MB_OK}
    }
};

const DLL_PROCESS_ATTACH: c_ulong = 1;
//const DLL_PROCESS_DETACH: c_ulong = 0;

fn start_shinmy(hmodule: HMODULE) -> std::io::Result<()> {
    let temp_dir_rnd: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    let exec_path = std::env::temp_dir().join(format!("shinmy-{}.exe", temp_dir_rnd));

    let mut exec_file = std::fs::File::create(&exec_path)?;
    exec_file.write_all(include_bytes!(concat!(env!("OUT_DIR"), "/../../../shinmy.exe")))?;
    exec_file.sync_all()?;
    std::mem::drop(exec_file);

    let mut slice = [0u16; MAX_PATH as usize];
    let length = unsafe { GetModuleFileNameW(hmodule, &mut slice) } as usize;
    let mallet_path_str = unsafe { Utf16Str::from_slice_unchecked(&slice[..length]) };
    let res = std::process::Command::new("cmd")
        .args(["/C", "start", exec_path.to_str().unwrap(), &mallet_path_str.to_string()])
        .creation_flags(DETACHED_PROCESS.0)
        .spawn();

    if let Err(e) = res {
        unsafe { MessageBoxW(None, &HSTRING::from(e.to_string()), w!("Shinmy Error"), MB_OK); }
    }

    Ok(())
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn DllMain(hmodule: HMODULE, call_reason: c_ulong, _reserved: *mut c_void) -> BOOL {
    if call_reason == DLL_PROCESS_ATTACH {
        // Init
        let system_dir = utils::get_system_directory();
        proxy::version::init(&system_dir);
        proxy::winhttp::init(&system_dir);

        _ = start_shinmy(hmodule);
    }
    true.into()
}