use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

#[derive(Clone, Copy)]
pub struct DhtData {
    pub temperature: u8,
    pub humidity: u8,
}

pub static DHT: Mutex<CriticalSectionRawMutex, Option<DhtData>> = Mutex::new(None);
pub static LIGHT: Mutex<CriticalSectionRawMutex, Option<u16>> = Mutex::new(None);
pub static DISTANCE_CM: Mutex<CriticalSectionRawMutex, Option<u32>> = Mutex::new(None);
