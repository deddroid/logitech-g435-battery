//! "Start with the session" toggle: an XDG autostart entry on Linux, a
//! HKCU Run value on Windows.

#[cfg(target_os = "linux")]
mod imp {
    use std::path::PathBuf;
    use std::{env, fs, io};

    fn entry_path() -> Option<PathBuf> {
        let config = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(config.join("autostart").join("g435-tray.desktop"))
    }

    pub fn is_enabled() -> bool {
        entry_path().is_some_and(|p| p.exists())
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let path = entry_path().ok_or_else(|| io::Error::other("HOME non impostata"))?;
        if !enabled {
            return match fs::remove_file(&path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                r => r,
            };
        }
        let exe = env::current_exe()?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(
            &path,
            format!(
                "[Desktop Entry]\nType=Application\nName=G435 Tray\n\
                 Comment=Batteria Logitech G435\nExec=\"{}\"\nIcon=audio-headset\n\
                 X-GNOME-Autostart-enabled=true\n",
                exe.display()
            ),
        )
    }
}

#[cfg(windows)]
mod imp {
    use std::{env, io, ptr};
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE: &str = "G435Tray";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn open(access: u32) -> io::Result<HKEY> {
        let mut key: HKEY = ptr::null_mut();
        let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), 0, access, &mut key) };
        if rc != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(rc as i32));
        }
        Ok(key)
    }

    pub fn is_enabled() -> bool {
        let Ok(key) = open(KEY_QUERY_VALUE) else { return false };
        let rc = unsafe {
            RegQueryValueExW(key, wide(VALUE).as_ptr(), ptr::null(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
        };
        unsafe { RegCloseKey(key) };
        rc == ERROR_SUCCESS
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let key = open(KEY_SET_VALUE)?;
        let name = wide(VALUE);
        let rc = if enabled {
            let cmd = wide(&format!("\"{}\"", env::current_exe()?.display()));
            unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, cmd.as_ptr().cast(), (cmd.len() * 2) as u32) }
        } else {
            unsafe { RegDeleteValueW(key, name.as_ptr()) }
        };
        unsafe { RegCloseKey(key) };
        if rc == ERROR_SUCCESS || !enabled {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(rc as i32))
        }
    }
}

pub use imp::{is_enabled, set_enabled};
