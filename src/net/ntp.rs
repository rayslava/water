use chrono::{DateTime, TimeDelta, Utc};
use core::net::{IpAddr, SocketAddr};
use embassy_net::{Stack, udp::UdpSocket};
use embassy_time::{Duration, Instant, Timer};
use smoltcp::{storage::PacketMetadata, wire::DnsQueryType};
use sntpc::{NtpContext, NtpTimestampGenerator, get_time};

use crate::{display::update_status, error::SysError, io::rtc::set_time};

const NTP_SERVER: &str = "pool.ntp.org";

#[derive(Copy, Clone)]
struct Timestamp {
    duration: Duration,
    offset: DateTime<Utc>,
}

impl Timestamp {
    fn new(offset: DateTime<Utc>) -> Timestamp {
        Timestamp {
            duration: Duration::default(),
            offset,
        }
    }
}

impl<'a> NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        self.duration = Duration::from_micros(
            (self.offset + TimeDelta::milliseconds(Instant::now().as_millis().try_into().unwrap()))
                .timestamp_micros()
                .try_into()
                .unwrap(),
        );
        log::info!("duration: {}ms", self.duration.as_millis());
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        (self.duration.as_micros() - self.duration.as_secs() * 1000000)
            .try_into()
            .unwrap()
    }
}

pub struct NtpClient<'a> {
    stack: &'a Stack<'a>,
    context: NtpContext<Timestamp>,
}

impl<'a> NtpClient<'a> {
    pub fn new(stack: &'a Stack<'a>) -> NtpClient<'a> {
        NtpClient {
            stack,
            context: NtpContext::new(Timestamp::new(DateTime::from_timestamp_nanos(0))),
        }
    }

    pub async fn sync(&self) -> Result<(), SysError> {
        let stack = self.stack;

        let mut udp_rx_meta = [PacketMetadata::EMPTY; 16];
        let mut udp_rx_buffer = [0; 1024];
        let mut udp_tx_meta = [PacketMetadata::EMPTY; 16];
        let mut udp_tx_buffer = [0; 1024];

        let mut socket = UdpSocket::new(
            *stack,
            &mut udp_rx_meta,
            &mut udp_rx_buffer,
            &mut udp_tx_meta,
            &mut udp_tx_buffer,
        );

        socket.bind(123).unwrap();

        let ntp_addrs = stack
            .dns_query(NTP_SERVER, DnsQueryType::A)
            .await
            .expect("Failed to resolve DNS");
        if ntp_addrs.is_empty() {
            log::error!("Failed to resolve DNS");
        }
        let addr: IpAddr = ntp_addrs[0].into();
        let result = get_time(SocketAddr::from((addr, 123)), &socket, self.context).await;

        match result {
            Ok(time) => {
                let datetime = DateTime::from_timestamp(
                    time.sec().into(),
                    (time.sec_fraction() as u64 * 1_000_000_000 / 4_294_967_296) as u32,
                )
                .unwrap();

                Ok(set_time(datetime.timestamp_micros() as u64).await?)
            }
            Err(e) => {
                log::error!("Error getting time: {:?}", e);
                Err(SysError::TimerSetup)
            }
        }
    }

    pub fn get_date_time(&self) -> DateTime<Utc> {
        let mut context = self.context.clone();
        context.timestamp_gen.init();
        DateTime::from_timestamp(
            context.timestamp_gen.timestamp_sec().try_into().unwrap(),
            context.timestamp_gen.timestamp_subsec_micros() * 1000,
        )
        .unwrap()
    }
}

const NTP_REFRESH_TIME: Duration = Duration::from_secs(3600);

#[embassy_executor::task]
pub async fn ntp_task(client: NtpClient<'static>) {
    loop {
        update_status("Syncing NTP").await.ok();
        if let Ok(()) = client.sync().await {
            update_status("Time synced").await.ok();
        } else {
            update_status("NTP failed, proceeding").await.ok();
        };
        Timer::after(NTP_REFRESH_TIME).await;
    }
}
