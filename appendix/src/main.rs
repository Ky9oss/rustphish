// #![no_std]
#![no_main]
#![feature(lang_items)]
use core::arch::asm;
use core::ptr::null;
use core::ptr::null_mut;

// 使用 MaybeUninit 避免默认初始化破坏二进制特征
use core::mem::MaybeUninit;
static mut CONFIG: MaybeUninit<Config> = MaybeUninit::uninit();

type HINTERNET = *mut u8;
type DWORD = u32;
type LPCWSTR = *const u16;
type LPVOID = *mut u8;

#[repr(C)]
struct Config {
    url: [u8; 256],
}

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        pszAgentW: LPCWSTR,
        dwAccessType: DWORD,
        pszProxyW: LPCWSTR,
        pszProxyBypassW: LPCWSTR,
        dwFlags: DWORD,
    ) -> HINTERNET;

    fn WinHttpConnect(
        hSession: HINTERNET,
        pswzServerName: LPCWSTR,
        nServerPort: DWORD,
        dwReserved: DWORD,
    ) -> HINTERNET;

    fn WinHttpOpenRequest(
        hConnect: HINTERNET,
        pwszVerb: LPCWSTR,
        pwszObjectName: LPCWSTR,
        pwszVersion: LPCWSTR,
        pwszReferrer: LPCWSTR,
        ppwszAcceptTypes: *mut LPCWSTR,
        dwFlags: DWORD,
    ) -> HINTERNET;

    fn WinHttpSendRequest(
        hRequest: HINTERNET,
        pwszHeaders: LPCWSTR,
        dwHeadersLength: DWORD,
        lpOptional: LPVOID,
        dwOptionalLength: DWORD,
        dwTotalLength: DWORD,
        dwContext: DWORD,
    ) -> bool;
        
}

#[unsafe(no_mangle)]
pub extern "system" fn main() -> i32 {
    init_config();
    let url = get_url();
    unsafe{

        let utf16_url: Vec<u16> = url.encode_utf16().collect();
        let url_ptr: *const u16 = utf16_url.as_ptr();
        let h_session = WinHttpOpen(null(), 0, null(), null(), 0);
        let h_connect = WinHttpConnect(h_session, url_ptr, 80, 0);
        let h_request = WinHttpOpenRequest(h_connect, "GET\0".encode_utf16().collect::<Vec<_>>().as_ptr(), null(), null(), null(), null_mut(), 0);
        WinHttpSendRequest(h_request, null(), 0, null_mut(), 0, 0, 0);
        0
    }
}


// 初始化函数（实际可能被优化，但二进制中会保留缓冲区空间）
pub fn init_config() {
    let url = b"https://default-url.com\0"; // 默认值，注意留有终止符
    unsafe {
        let config = CONFIG.as_mut_ptr();
        core::ptr::copy_nonoverlapping(
            url.as_ptr(),
            (*config).url.as_mut_ptr(),
            url.len(),
        );
    }
}

// 获取 URL 的 &str 引用
pub fn get_url() -> &'static str {
    unsafe {
        let config = CONFIG.as_ptr();
        let bytes = &(*config).url;
        // 查找第一个 null 字节作为终止符
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(256);
        std::str::from_utf8_unchecked(&bytes[..len])
    }
}

