use core::fmt::Write as FmtWrite;

use embassy_net::{
    IpAddress, IpEndpoint, Stack,
    dns::{DnsQueryType, DnsSocket},
    tcp::TcpSocket,
};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::String;

const IOT_SERVER_HOST: &str = env!("IOT_SERVER_HOST");
const IOT_SERVER_PATH: &str = env!("IOT_SERVER_PATH");
const IOT_SERVER_PORT: u16 = 80;

pub const SEND_INTERVAL: Duration = Duration::from_secs(1);

#[embassy_executor::task]
pub async fn task(stack: Stack<'static>, interval: Duration) {
    let mut cached_addr: Option<IpAddress> = None;

    loop {
        Timer::after(interval).await;

        // Resolve DNS once, reuse across ticks
        if cached_addr.is_none() {
            let dns = DnsSocket::new(stack);
            match dns.query(IOT_SERVER_HOST, DnsQueryType::A).await {
                Ok(addrs) if !addrs.is_empty() => cached_addr = Some(addrs[0]),
                Ok(_) => {
                    esp_println::println!("DNS: no result for {}", IOT_SERVER_HOST);
                    continue;
                }
                Err(e) => {
                    esp_println::println!("DNS error: {:?}", e);
                    continue;
                }
            }
        }
        let addr = match cached_addr {
            Some(a) => a,
            None => continue,
        };

        let dht = crate::sensor_data::DHT.lock().await.take();
        let light = crate::sensor_data::LIGHT.lock().await.take();
        let dist = crate::sensor_data::DISTANCE_CM.lock().await.take();

        if dht.is_none() && light.is_none() && dist.is_none() {
            continue;
        }

        // Build one combined payload with only the fields that have new data
        let mut fields: String<128> = String::new();
        let mut first = true;
        if let Some(d) = dht {
            write!(fields, "\"temperature\":{},\"humidity\":{}", d.temperature, d.humidity).ok();
            first = false;
        }
        if let Some(l) = light {
            if !first { write!(fields, ",").ok(); }
            write!(fields, "\"light\":{}", l).ok();
            first = false;
        }
        if let Some(d) = dist {
            if !first { write!(fields, ",").ok(); }
            write!(fields, "\"distance_cm\":{}", d).ok();
        }

        let mut body: String<160> = String::new();
        write!(body, "{{\"payload\":{{{}}}}}", fields).ok();
        post_once(stack, addr, &body).await;
    }
}

async fn post_once(stack: Stack<'static>, addr: IpAddress, body: &str) {
    let mut rx_buf = [0u8; 256];
    let mut tx_buf = [0u8; 512];
    let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    socket.set_timeout(Some(Duration::from_secs(5)));

    if let Err(e) = socket.connect(IpEndpoint::new(addr, IOT_SERVER_PORT)).await {
        esp_println::println!("TCP: {:?}", e);
        return;
    }

    let mut request: String<512> = String::new();
    write!(
        request,
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        IOT_SERVER_PATH, IOT_SERVER_HOST, body.len(), body,
    )
    .ok();

    if let Err(e) = socket.write_all(request.as_bytes()).await {
        esp_println::println!("HTTP write: {:?}", e);
        return;
    }

    let mut resp = [0u8; 64];
    match socket.read(&mut resp).await {
        Ok(n) => esp_println::println!(
            "POST → {}",
            core::str::from_utf8(&resp[..n])
                .unwrap_or("?")
                .lines()
                .next()
                .unwrap_or("?")
        ),
        Err(e) => esp_println::println!("HTTP read: {:?}", e),
    }
}
