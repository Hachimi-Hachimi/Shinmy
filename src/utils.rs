use std::{ffi::CStr, path::{Path, PathBuf}};

use tinyjson::JsonValue;
use windows::Win32::{
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPALL
    },
    UI::Shell::{FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_DEFAULT}
};

pub fn is_dmm_running() -> bool {
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPALL, 0) }) else {
        return false;
    };
    let mut entry = PROCESSENTRY32::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
    let mut res = unsafe { Process32First(snapshot, &mut entry) };

    while res.is_ok() {
        let process_name = unsafe { CStr::from_ptr(entry.szExeFile.as_ptr()) };
        if process_name == c"DMMGamePlayer.exe" {
            return true;
        }

        res = unsafe { Process32Next(snapshot, &mut entry) };
    }

    false
}

pub fn detect_game_install_dir() -> Option<PathBuf> {
    let app_data_dir_wstr = unsafe { SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None).ok()? };
    let app_data_dir_str = unsafe { app_data_dir_wstr.to_string().ok()? };
    let app_data_dir = Path::new(&app_data_dir_str);
    let mut dmm_config_path = app_data_dir.join("dmmgameplayer5");
    dmm_config_path.push("dmmgame.cnf");

    let config_str = std::fs::read_to_string(dmm_config_path).ok()?;
    let JsonValue::Object(config) = config_str.parse().ok()? else {
        return None;
    };
    let JsonValue::Array(config_contents) = &config["contents"] else {
        return None;
    };
    for value in config_contents {
        let JsonValue::Object(game) = value else {
            return None;
        };

        let JsonValue::String(product_id) = &game["productId"] else {
            continue;
        };
        if product_id != "umamusume" {
            continue;
        }

        let JsonValue::Object(detail) = &game["detail"] else {
            return None;
        };
        let JsonValue::String(path_str) = &detail["path"] else {
            return None;
        };

        let path = PathBuf::from(path_str);
        return if path.is_dir() {
            Some(path)
        }
        else {
            None
        }
    }

    None
}