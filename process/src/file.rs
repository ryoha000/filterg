use bindings::Windows::Win32::Media::Multimedia::{
    mmioOpenW, HMMIO, MMIOINFO, MMIO_CREATE, MMIO_WRITE,
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
