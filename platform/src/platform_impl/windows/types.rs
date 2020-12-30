#![allow(bad_style, overflowing_literals, dead_code)]

use crate::declare_handle;
use crate::ctypes::*;
use super::externs::*;

declare_handle!{HWND, HWND__}
declare_handle!{HINSTANCE, HINSTANCE__}
declare_handle!{HICON, HICON__}
declare_handle!{HBRUSH, HBRUSH__}
declare_handle!{HMENU, HMENU__}

pub type HANDLE = *mut c_void;
pub type HMODULE = HINSTANCE;
pub type HCURSOR = HICON;
pub type wchar_t = u16;
pub type BOOL = c_int;
pub type CHAR = c_char;
pub type WCHAR = wchar_t;
pub type WORD = c_ushort;
pub type DWORD = c_ulong;
pub type INT = c_int;
pub type UINT = c_uint;
pub type LONG = c_long;
pub type UINT_PTR = usize;
pub type LONG_PTR = isize;
pub type ATOM = WORD;
pub type LPCSTR = *const CHAR;
pub type LPCWSTR = *const WCHAR;
pub type WPARAM = UINT_PTR;
pub type LPARAM = LONG_PTR;
pub type LRESULT = LONG_PTR;
pub type LPVOID = *mut ::std::ffi::c_void;
pub type LPMSG = *mut MSG;

pub enum __some_function {}
/// Pointer to a function with unknown type signature.
pub type FARPROC = *mut __some_function;

pub const CS_VREDRAW: UINT = 0x0001;
pub const CS_HREDRAW: UINT = 0x0002;
pub const CS_OWNDC: UINT = 0x0020;

pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub const WS_OVERLAPPEDWINDOW: DWORD = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME
    | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
pub const WS_VISIBLE: DWORD = 0x10000000;

pub const WM_DESTROY: UINT = 0x0002;
pub const WM_MOVE: UINT = 0x0003;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_ACTIVATE: UINT = 0x0006;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_QUIT: UINT = 0x0012;
pub const WM_NCDESTROY: UINT = 0x0082;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_SIZING: UINT = 0x0214;
pub const WM_MOVING: UINT = 0x0216;

pub const PM_NOREMOVE: UINT = 0x0000;
pub const PM_REMOVE: UINT = 0x0001;
pub const PM_NOYIELD: UINT = 0x0002;

pub const LOAD_LIBRARY_SEARCH_SYSTEM32: DWORD = 0x00000800;

#[repr(C)] 
#[derive(Clone, Copy)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: c_int,
    pub cbWndExtra: c_int,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: LPCWSTR,
    pub lpszClassName: LPCWSTR
}

#[repr(C)] 
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)] 
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}


