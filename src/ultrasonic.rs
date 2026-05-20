use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, InputPin, Level, Output, OutputConfig, OutputPin, Pull},
    time::{Duration as HalDuration, Instant},
};

pub fn new(
    trig_gpio: impl OutputPin + 'static,
    echo_gpio: impl InputPin + 'static,
) -> (Output<'static>, Input<'static>) {
    let trig = Output::new(trig_gpio, Level::Low, OutputConfig::default());
    let echo = Input::new(echo_gpio, InputConfig::default().with_pull(Pull::Down));
    (trig, echo)
}

#[embassy_executor::task]
pub async fn task(
    mut trig: Output<'static>,
    echo: Input<'static>,
    distance_signal: &'static Signal<CriticalSectionRawMutex, u32>,
) {
    let delay = Delay::new();
    let timeout = HalDuration::from_millis(30);

    'outer: loop {
        trig.set_low();
        delay.delay_micros(2);
        trig.set_high();
        delay.delay_micros(10);
        trig.set_low();

        let wait_start = Instant::now();
        while echo.is_low() {
            if wait_start.elapsed() > timeout {
                Timer::after(Duration::from_millis(100)).await;
                continue 'outer;
            }
        }

        let time1 = Instant::now();
        while echo.is_high() {
            if time1.elapsed() > timeout {
                break;
            }
        }

        let pulse_width = time1.elapsed().as_micros();
        let distance_cm = (pulse_width * 343 / 20000) as u32;
        esp_println::println!("Distance: {} cm", distance_cm);
        distance_signal.signal(distance_cm);

        Timer::after(Duration::from_millis(100)).await;
    }
}
