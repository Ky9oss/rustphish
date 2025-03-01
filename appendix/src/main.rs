#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![no_std]
#![no_main]
#![feature(abi_thiscall)]
#![allow(named_asm_labels)]

// 在main.rs顶部添加以下实现
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
    System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_READWRITE},
    Networking::WinHttp::{WinHttpOpen, WinHttpConnect, WinHttpOpenRequest, WinHttpSendRequest, WinHttpCloseHandle, INTERNET_DEFAULT_HTTP_PORT, WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_FLAG_BYPASS_PROXY_CACHE}
};
extern crate alloc;
use alloc::vec::Vec;
use alloc::alloc::alloc;
use alloc::alloc::Layout;
use core::ffi::c_void;
use core::cell::UnsafeCell;
use core::ptr::{null, null_mut};
use windows_sys::Win32::Networking::WinHttp::*;

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

// 使用自定义内存分配的动态版本
fn encode_utf16_heap(s: &str) -> &'static mut [u16] {
    let len = s.encode_utf16().count() + 1;
    let layout = Layout::array::<u16>(len).unwrap();
    let ptr = unsafe { alloc(layout) } as *mut u16;
    
    let mut i = 0;
    for c in s.encode_utf16() {
        unsafe { *ptr.add(i) = c };
        i += 1;
    }
    unsafe { *ptr.add(i) = 0 }; // null终止
    
    unsafe { core::slice::from_raw_parts_mut(ptr, len) }
}

#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> i32 {
    let method = encode_utf16_heap("GET");
    let ip_or_domain = encode_utf16_heap("10.111.16.10");
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
            8080 as u16, // port
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
        
        if result == 0 {
            WinHttpCloseHandle(h_request);
            WinHttpCloseHandle(h_connect);
            WinHttpCloseHandle(h_session);
            return 0;
        }

        WinHttpCloseHandle(h_request);
        WinHttpCloseHandle(h_connect);
        WinHttpCloseHandle(h_session);
        0
    }
}


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 实现结构化panic处理（可记录到共享内存）
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(0xDEAD) };
}