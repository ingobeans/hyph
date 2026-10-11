use winapi::{
    shared::windef::HWND__,
    um::winuser::{EnumWindows, GetWindowTextW, IsWindowVisible, WNDENUMPROC},
};

unsafe fn get_title(hwnd: *mut HWND__) -> Option<String> {
    let mut buffer = [0u16; 128];
    let len = unsafe {
        let t = buffer.as_mut_ptr();
        GetWindowTextW(hwnd, t, buffer.len() as _)
    };
    if len == 0 {
        return None;
    }
    let string = String::from_utf16_lossy(&buffer[..len as usize]);
    Some(string)
}

unsafe extern "system" fn try_get_name(hwnd: *mut HWND__, pointer: isize) -> i32 {
    if unsafe { IsWindowVisible(hwnd) != 0 } {
        let Some(title) = (unsafe { get_title(hwnd) }) else {
            return 1;
        };
        if title.is_empty() {
            return 1;
        }

        let names: *mut Vec<String> = pointer as _;
        if let Some(names) = unsafe { names.as_mut() } {
            names.push(title);
        }
    }

    1
}

pub fn get_open_windows() -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    unsafe {
        let lparam = (&mut names as *mut Vec<String>) as isize;
        let enum_func = WNDENUMPROC::Some(try_get_name);
        let result = EnumWindows(enum_func, lparam);

        if result == 0 {
            return None;
        }
    }
    Some(names)
}
