#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embedded_dht_rs::dht11::Dht11;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{DriveMode, Flex, Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::time::{Duration, Instant};

use esp_println::println;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // generator version: 1.0.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // DHT11 sensor setup (GPIO32)
    let mut dht11_pin = Flex::new(peripherals.GPIO32);
    dht11_pin.apply_output_config(
        &OutputConfig::default()
            .with_drive_mode(DriveMode::OpenDrain)
            .with_pull(Pull::None),
    );
    dht11_pin.set_output_enable(true);
    dht11_pin.set_input_enable(true);
    dht11_pin.set_high();

    let mut dht11 = Dht11::new(dht11_pin, Delay::new());

    // For KY-018 photoresistor module (connected to GPIO33
    // Configure ADC for photoresistor reading
    let adc_pin = peripherals.GPIO33;
    let mut adc2_config = AdcConfig::new();
    let mut pin = adc2_config.enable_pin(adc_pin, Attenuation::_11dB);
    let mut adc2 = Adc::new(peripherals.ADC2, adc2_config);

    // For buzzer
    let mut buzzer = Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default());

    // For HC-SR04 Ultrasonic
    let mut trig = Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default());
    let echo = Input::new(
        peripherals.GPIO18,
        InputConfig::default().with_pull(Pull::Down),
    );

    loop {
        blocking_delay(Duration::from_millis(1000));

        // Read DHT11 sensor data
        match dht11.read() {
            Ok(sensor_reading) => {
                esp_println::println!(
                    "DHT11 - Temperature: {} °C, humidity: {} %",
                    sensor_reading.temperature,
                    sensor_reading.humidity
                );
            }
            Err(error) => {
                esp_println::dbg!("Failed to read DHT11 sensor: {:?}", error);
            }
        }

        // KY-018 one-shot read of the Photoresistor value, blocking until conversion is complete
        let pin_value: u16 = nb::block!(adc2.read_oneshot(&mut pin)).unwrap();
        println!("Photoresistor ADC Value: {}", pin_value);

        // Trigger ultrasonic waves
        trig.set_low();
        blocking_delay(Duration::from_micros(2));
        trig.set_high();
        blocking_delay(Duration::from_micros(10));
        trig.set_low();

        // Measure the duration the signal remains high
        let timeout = Duration::from_millis(30);
        'measure: {
            let wait_start = Instant::now();
            while echo.is_low() {
                if wait_start.elapsed() > timeout {
                    break 'measure;
                }
            }
            let time1 = Instant::now();
            while echo.is_high() {
                // If echo doesnt come back to low after 30ms, it commonly means no object was detected within range.
                // So we break out of the loop to avoid an infinite loop.
                if time1.elapsed() > timeout {
                    break;
                }
            }
            let pulse_width = time1.elapsed().as_micros();

            // distance_cm = pulse_width_us * 343 / 20000  (speed of sound: 343 m/s)
            let distance_cm = pulse_width * 343 / 20000;
            esp_println::println!("Pulse Width: {}", pulse_width);
            esp_println::println!("Distance: {} cm", distance_cm);

            if distance_cm < 30 {
                buzzer.set_high();
                println!("Object detected within 30 cm! Buzzer ON.");
            } else {
                buzzer.set_low();
                println!("No object detected within 30 cm. Buzzer OFF.");
            }
        }

        blocking_delay(Duration::from_millis(60));
    }
}

fn blocking_delay(duration: Duration) {
    let delay_start = Instant::now();
    while delay_start.elapsed() < duration {}
}
