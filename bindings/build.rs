fn main() {
    windows::build!(
        Windows::Foundation::Collections::IVector,
        Windows::Foundation::{IAsyncOperationWithProgress, Uri},

        Windows::Web::Syndication::{
            ISyndicationText, RetrievalProgress, SyndicationClient, SyndicationFeed, SyndicationItem,
        },

        Windows::Win32::Media::Audio::CoreAudio::IMMDevice,

        Windows::Win32::System::Com::{
            COINIT, CLSCTX,
            CLSIDFromProgID, CoInitializeEx, CoUninitialize, CoCreateInstance,
        },

        Windows::Win32::Foundation::{
            PWSTR, BSTR,
            SysFreeString,
        },

        Windows::Win32::Media::Audio::CoreAudio::{MMDeviceEnumerator, IMMDeviceEnumerator, IMMDevice},
    );
}
