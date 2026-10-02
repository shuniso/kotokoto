//! Windows 部分: グローバルキーフックと、終了用のトレイアイコン。

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::{mem, ptr};

use windows_sys::w;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_BACK, VK_RETURN, VK_SPACE};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::sound;

const WM_TRAY: u32 = WM_APP + 1;
const CMD_QUIT: usize = 1;
/// 同じキーの keydown がこの間隔以内で続いたらオートリピートとみなす（リピート開始の遅延は最大 1 秒）
const REPEAT_MS: u32 = 1500;

static TX: OnceLock<Sender<usize>> = OnceLock::new();
/// キーごとの「押下中なら最後の keydown 時刻（0 以外）、離していれば 0」
static HELD: [AtomicU32; 256] = [const { AtomicU32::new(0) }; 256];
/// エクスプローラー再起動の通知。トレイアイコンを登録し直すのに使う
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn run(tx: Sender<usize>) {
    unsafe {
        // 二重起動すると音が重なるので、後から起動した方は黙って終わる
        CreateMutexW(ptr::null(), 0, w!("kotokoto-single-instance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        let _ = TX.set(tx);

        let instance = GetModuleHandleW(ptr::null());
        let class = w!("kotokoto");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class,
            ..mem::zeroed()
        };
        RegisterClassW(&wc);
        // トレイのメッセージを受けるだけの、表示しないウィンドウ
        let hwnd = CreateWindowExW(
            0,
            class,
            class,
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        );

        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), instance, 0);
        if hwnd.is_null() || hook.is_null() {
            MessageBoxW(
                ptr::null_mut(),
                w!("起動に失敗しました。"),
                class,
                MB_ICONERROR,
            );
            return;
        }

        TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Relaxed);
        let nid = tray_icon(hwnd);
        Shell_NotifyIconW(NIM_ADD, &nid);

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(hook);
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

unsafe fn tray_icon(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
    for (dst, src) in nid.szTip.iter_mut().zip("kotokoto".encode_utf16()) {
        *dst = src;
    }
    nid
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let key = &*(lp as *const KBDLLHOOKSTRUCT);
        let held = &HELD[(key.vkCode & 0xFF) as usize];
        match wp as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // 押しっぱなしのリピートでは鳴らさない。
                // 時刻も見るのは、Win+L や UAC で keyup を取り逃がしたキーが鳴らなくなるのを防ぐため。
                let prev = held.swap(key.time | 1, Relaxed);
                if prev == 0 || key.time.wrapping_sub(prev) >= REPEAT_MS {
                    let big = matches!(key.vkCode as u16, VK_SPACE | VK_RETURN | VK_BACK);
                    if let Some(tx) = TX.get() {
                        let _ = tx.send(sound::variant(key.vkCode, big));
                    }
                }
            }
            WM_KEYUP | WM_SYSKEYUP => held.store(0, Relaxed),
            _ => {}
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wp, lp)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY if matches!(lp as u32, WM_LBUTTONUP | WM_RBUTTONUP) => {
            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, CMD_QUIT, w!("終了"));
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            // これが無いとメニューの外をクリックしても閉じない
            SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                pt.x,
                pt.y,
                0,
                hwnd,
                ptr::null(),
            );
            DestroyMenu(menu);
            if cmd as usize == CMD_QUIT {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ if msg == TASKBAR_CREATED.load(Relaxed) && msg != 0 => {
            Shell_NotifyIconW(NIM_ADD, &tray_icon(hwnd));
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
