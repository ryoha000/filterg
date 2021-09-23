use bindings::Windows::Win32::Media::Multimedia::HMMIO;
use bindings::Windows::Win32::{Foundation::HANDLE, Media::Audio::CoreAudio::IMMDevice};

pub struct Args {
    pub hr: windows::HRESULT,
    pub mm_device: IMMDevice,
    pub b_int16: bool,
    pub h_file: HMMIO,
    pub h_started_event: HANDLE,
    pub h_stop_event: HANDLE,
    pub n_frames: u32,
}
