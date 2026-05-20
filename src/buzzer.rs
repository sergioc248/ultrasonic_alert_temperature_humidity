use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};

pub fn new(gpio: impl OutputPin + 'static) -> Output<'static> {
    Output::new(gpio, Level::Low, OutputConfig::default())
}

#[embassy_executor::task]
pub async fn task(
    mut buzzer: Output<'static>,
    distance_signal: &'static Signal<CriticalSectionRawMutex, u32>,
) {
    loop {
        let distance_cm = distance_signal.wait().await;
        if distance_cm < 30 {
            buzzer.set_high();
            esp_println::println!("Buzzer ON — object at {} cm", distance_cm);
        } else {
            buzzer.set_low();
        }
    }
}
