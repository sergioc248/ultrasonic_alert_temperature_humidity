use embassy_time::{Duration, Timer};
use embedded_dht_rs::dht11::Dht11;
use esp_hal::{
    delay::Delay,
    gpio::{DriveMode, Flex, OutputConfig, Pin, Pull},
};

pub fn new(gpio: impl Pin + 'static) -> Dht11<Flex<'static>, Delay> {
    let mut pin = Flex::new(gpio);
    pin.apply_output_config(
        &OutputConfig::default()
            .with_drive_mode(DriveMode::OpenDrain)
            .with_pull(Pull::Up),
    );
    pin.set_output_enable(true);
    pin.set_input_enable(true);
    pin.set_high();
    Dht11::new(pin, Delay::new())
}

#[embassy_executor::task]
pub async fn task(mut dht11: Dht11<Flex<'static>, Delay>) {
    Timer::after(Duration::from_secs(2)).await; // sensor warm-up
    loop {
        match dht11.read() {
            Ok(r) => {
                esp_println::println!(
                    "DHT11 - Temp: {} °C, Humidity: {} %",
                    r.temperature,
                    r.humidity
                );
                *crate::sensor_data::DHT.lock().await =
                    Some(crate::sensor_data::DhtData {
                        temperature: r.temperature,
                        humidity: r.humidity,
                    });
            }
            Err(e) => esp_println::println!("DHT11 read error: {:?}", e),
        }
        Timer::after(Duration::from_secs(2)).await;
    }
}
