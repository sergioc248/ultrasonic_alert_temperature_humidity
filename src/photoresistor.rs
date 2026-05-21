use embassy_time::{Duration, Timer};
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC1, GPIO34},
};

type PhotoresistorPin = GPIO34<'static>;

pub fn new(
    gpio: PhotoresistorPin,
    adc_peripheral: ADC1<'static>,
) -> (
    Adc<'static, ADC1<'static>, Blocking>,
    AdcPin<PhotoresistorPin, ADC1<'static>>,
) {
    let mut config = AdcConfig::new();
    let pin = config.enable_pin(gpio, Attenuation::_6dB);
    let adc1 = Adc::new(adc_peripheral, config);
    (adc1, pin)
}

#[embassy_executor::task]
pub async fn task(
    mut adc1: Adc<'static, ADC1<'static>, Blocking>,
    mut pin: AdcPin<PhotoresistorPin, ADC1<'static>>,
) {
    loop {
        let value: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
        esp_println::println!("Photoresistor ADC: {}", value);
        *crate::sensor_data::LIGHT.lock().await = Some(value);
        Timer::after(Duration::from_secs(1)).await;
    }
}
