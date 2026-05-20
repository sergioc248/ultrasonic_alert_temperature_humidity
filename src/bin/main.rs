#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use static_cell::StaticCell;
use ultrasonic_alert_temperature_humidity::{
    buzzer, dht11, http_client, photoresistor, ultrasonic, wifi,
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

static DISTANCE_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, u32>> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let distance_signal = DISTANCE_SIGNAL.init(Signal::new());

    // Sensors + actuators
    let sensor = dht11::new(peripherals.GPIO32);
    let (adc1, adc_pin) = photoresistor::new(peripherals.GPIO33, peripherals.ADC1);
    let buzzer_out = buzzer::new(peripherals.GPIO14);
    let (trig, echo) = ultrasonic::new(peripherals.GPIO16, peripherals.GPIO18);

    // WiFi + network stack
    let rng = Rng::new();
    let (wifi_controller, stack, runner) = wifi::new(peripherals.WIFI, &rng);

    spawner.spawn(wifi::connection_task(wifi_controller).unwrap());
    spawner.spawn(wifi::net_task(runner).unwrap());

    wifi::wait_for_connection(stack).await;

    // Sensor tasks
    spawner.spawn(dht11::task(sensor).unwrap());
    spawner.spawn(photoresistor::task(adc1, adc_pin).unwrap());
    spawner.spawn(ultrasonic::task(trig, echo, distance_signal).unwrap());
    spawner.spawn(buzzer::task(buzzer_out, distance_signal).unwrap());
    spawner.spawn(http_client::task(stack, http_client::SEND_INTERVAL).unwrap());

    loop {
        Timer::after_secs(3600).await;
    }
}
