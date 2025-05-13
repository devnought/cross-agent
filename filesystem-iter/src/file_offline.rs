use std::fs::Metadata;

pub trait FileOffline {
    fn is_offline(&self) -> bool;
}

#[cfg(target_os = "windows")]
impl FileOffline for Metadata {
    fn is_offline(&self) -> bool {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        };

        let attributes = self.file_attributes();

        attributes & FILE_ATTRIBUTE_OFFLINE.0 == FILE_ATTRIBUTE_OFFLINE.0
            || attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0
                == FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0
            || attributes & FILE_ATTRIBUTE_RECALL_ON_OPEN.0 == FILE_ATTRIBUTE_RECALL_ON_OPEN.0
    }
}

#[cfg(not(target_os = "windows"))]
impl FileOffline for Metadata {
    fn is_offline(&self) -> bool {
        false
    }
}
