// #![no_std]
#![no_main]
#![feature(lang_items)]
use std::arch::asm;
// #![feature(asm)]

type HINTERNET = *mut u8;
type DWORD = u32;
type LPCWSTR = *const u16;
type LPVOID = *mut u8;

#[link(name = "winhttp")]
extern "system" {
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

struct UrlInjector {
    base_address: usize,
    url_buffer: [u16; 2048],
}

unsafe fn get_module_base() -> usize {
    unsafe{
        let mut base;
        asm!(
            "mov {}, gs:[0x60]",
            "mov {}, [{} + 0x10]",
            out(reg) base,
            options(nostack),
        );
        base
    }
}


