use std::mem::size_of;

use bindings::Windows::Win32::{
    Foundation::PSTR,
    Media::Multimedia::{
        mmioAscend, mmioCreateChunk, mmioOpenW, mmioWrite, HMMIO, MMCKINFO, MMIOINFO, MMIO_CREATE,
        MMIO_CREATERIFF, MMIO_WRITE, MMSYSERR_NOERROR, WAVEFORMATEX,
    },
    System::Diagnostics::Debug::GetLastError,
};

use crate::utils::message_to_windows_error;

pub fn open_file(filename: &str) -> windows::Result<HMMIO> {
    let mut mi = MMIOINFO::default();

    let h_file = unsafe { mmioOpenW(filename, &mut mi, MMIO_WRITE | MMIO_CREATE) };
    if h_file == HMMIO(0) {
        return Err(message_to_windows_error("mmioOpen failed. "));
    }
    Ok(h_file)
}
