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

macro_rules! MAKEFOURCC {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        ($a as u32) | (($b as u32) << 8) | (($c as u32) << 16) | (($d as u32) << 24)
    };
}

pub fn write_wave_header(
    h_file: HMMIO,
    wfx: *mut WAVEFORMATEX,
    ck_riff: &mut MMCKINFO,
    ck_data: &mut MMCKINFO,
) -> windows::Result<u8> {
    ck_riff.ckid = MAKEFOURCC!(b'R', b'I', b'F', b'F');
    ck_riff.fccType = MAKEFOURCC!(b'W', b'A', b'V', b'E');

    let result = unsafe { mmioCreateChunk(h_file, ck_riff, MMIO_CREATERIFF) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioCreateChunk(RIFF WAVE) failed: MMRESULT: {}",
            result
        )));
    }

    let mut chunk = MMCKINFO::default();
    chunk.ckid = MAKEFOURCC!(b'f', b'm', b't', b' ');
    let result = unsafe { mmioCreateChunk(h_file, &chunk, 0) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioCreateChunk(fmt ) failed: MMRESULT: {}",
            result
        )));
    }

    let bytes_in_wfx = size_of::<WAVEFORMATEX>() as i32 + unsafe { (*wfx).cbSize } as i32;
    mmio_write_any(h_file, wfx, bytes_in_wfx)?;

    let result = unsafe { mmioAscend(h_file, &chunk, 0) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioAscend(fmt ) failed: MMRESULT: {}",
            result
        )));
    }

    chunk.ckid = MAKEFOURCC!(b'f', b'a', b'c', b't');
    let result = unsafe { mmioCreateChunk(h_file, &chunk, 0) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioCreateChunk(fact ) failed: MMRESULT: {}",
            result
        )));
    }

    let frames: u32 = 0;
    mmio_write_any(h_file, &frames, size_of::<u32>() as i32)?;

    let result = unsafe { mmioAscend(h_file, &chunk, 0) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioAscend(fact ) failed: MMRESULT: {}",
            result
        )));
    }

    ck_data.ckid = MAKEFOURCC!(b'd', b'a', b'd', b'a');
    let result = unsafe { mmioCreateChunk(h_file, ck_data, 0) };
    if MMSYSERR_NOERROR != result {
        return Err(message_to_windows_error(&format!(
            "mmioCreateChunk(data) failed: MMRESULT: {}",
            result
        )));
    }

    Ok(0)
}

fn mmio_write_any<T>(h_file: HMMIO, data: *const T, cch: i32) -> windows::Result<u8> {
    let pstr_size = size_of::<PSTR>() as i32;

    let casted_data = data as *const PSTR;

    let mut remain = cch;
    let mut loop_count = 0;
    loop {
        if remain <= 0 {
            break;
        }

        let pstr = unsafe { *casted_data.offset(loop_count as isize) };
        let bytes_written = unsafe { mmioWrite(h_file, &pstr, remain.min(pstr_size)) };
        if remain.min(pstr_size) != bytes_written {
            let e = unsafe { GetLastError() };
            println!(
                "mmioWrite(fact data) wrote {} bytes; expected {} bytes. last_error: {:?}",
                bytes_written,
                remain.min(pstr_size),
                e,
            );

            return Err(windows::Error::from_win32());
        }
        println!("success");
        remain -= pstr_size;
        loop_count += 1;
    }

    Ok(0)
}
