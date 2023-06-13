//
// Copyright (C) 2020 Jonas Zaddach.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 2 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
// MA 02110-1301, USA.
//

use std::fmt;
use std::result;
use std::os;
use std::error;

#[cfg(windows)]
use bindings::Windows::{
    Win32::System::Diagnostics::Debug::GetLastError,
    Win32::Storage::FileSystem::ReadFile,
    Win32::System::Diagnostics::Debug::ERROR_HANDLE_EOF,
    Win32::System::SystemServices::{
        OVERLAPPED,
        HANDLE,
    },
};

use clamav_sys::{
    cl_fmap_t,
    cl_fmap_open_handle,
    cl_fmap_open_memory,
    cl_fmap_close,
};

#[cfg(windows)]
pub type RawOsHandle = std::os::windows::io::RawHandle;
#[cfg(unix)]
pub type RawOsHandle = std::os::unix::io::RawFd;

#[derive(Debug, Clone)]
pub struct MapError;

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to open mapping")
    }
}

impl error::Error for MapError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl MapError {
    pub fn new() -> MapError { MapError{} }
}

pub type Result<T> = result::Result<T, MapError>;

#[cfg(windows)]
extern fn cl_pread(handle: *mut os::raw::c_void, buf: *mut os::raw::c_void, count: os::raw::c_ulonglong, offset: os::raw::c_long) -> os::raw::c_long {
    let mut read_bytes = 0;

    unsafe {
        let mut overlapped: OVERLAPPED = std::mem::MaybeUninit::zeroed().assume_init();
        overlapped.InternalHigh = (offset as usize) >> 32;
        overlapped.Internal = (offset as usize) & 0xffffffff;

        if !ReadFile(std::mem::transmute::<_, HANDLE>(handle), buf, count as u32, &mut read_bytes, &mut overlapped).as_bool() {
            let err = GetLastError();
            if err != ERROR_HANDLE_EOF {
                return -1;
            }
        }
    }

    read_bytes as i32
}

#[cfg(unix)]
extern fn cl_pread(handle: *mut os::raw::c_void, buf: *mut os::raw::c_void, count: usize, offset: os::raw::c_long) -> os::raw::c_long {
    use std::convert::TryInto;
    unsafe {
        libc::pread(handle as i32, buf, count.try_into().unwrap(), offset).try_into().unwrap()
    }
}

#[allow(dead_code)]
pub struct Fmap(*mut cl_fmap_t);

impl Fmap {
    pub fn new_from_memory(start: *const u8, len: u64) -> Result< Fmap > {
        let map = unsafe { cl_fmap_open_memory(start as *const os::raw::c_void, len as usize) };
        if map.is_null() {
            Err(MapError::new())
        }
        else {
            Ok(Fmap(map))
        }
    }

    pub fn new_from_handle(handle: RawOsHandle, offset: u64, len: u64, use_ageing: bool) -> Result< Fmap > {
        let map = unsafe { cl_fmap_open_handle(handle as *mut os::raw::c_void, offset as usize, len as usize, Some(cl_pread), use_ageing.into() ) };
        if map.is_null() {
            Err(MapError::new())
        }
        else {
            Ok(Fmap(map))
        }
    }

    pub fn raw(& self) -> *mut cl_fmap_t {self.0}
}

impl Drop for Fmap {
    fn drop(&mut self) -> () {
        unsafe {cl_fmap_close(self.0)};
    }
}
