use embassy_net::{DhcpConfig, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_println::println;
use esp_radio::wifi::{Config, Interface, WifiController, sta::StationConfig};

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

pub fn new(
    wifi: esp_hal::peripherals::WIFI<'static>,
    rng: &Rng,
) -> (
    WifiController<'static>,
    Stack<'static>,
    Runner<'static, Interface<'static>>,
) {
    let (controller, interfaces) =
        esp_radio::wifi::new(wifi, Default::default()).expect("Failed to init WiFi");

    let net_seed = u64::from(rng.random()) | (u64::from(rng.random()) << 32);
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        embassy_net::Config::dhcpv4(DhcpConfig::default()),
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );

    (controller, stack, runner)
}

pub async fn wait_for_connection(stack: Stack<'static>) {
    println!("Waiting for WiFi link...");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    println!("Waiting for IP...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
pub async fn connection_task(mut controller: WifiController<'static>) {
    let config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );
    controller.set_config(&config).unwrap();

    loop {
        println!("Connecting to WiFi...");
        match controller.connect_async().await {
            Ok(_) => {
                println!("WiFi connected!");
                controller.wait_for_disconnect_async().await.ok();
                println!("WiFi disconnected, retrying in 5s...");
            }
            Err(e) => println!("WiFi connect failed: {:?}", e),
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
