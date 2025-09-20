use chrono::{DateTime, TimeDelta, Utc};
use core::net::{IpAddr, SocketAddr};
use embassy_net::{Stack, udp::UdpSocket};
use embassy_time::{Duration, Instant, Timer};
use smoltcp::{storage::PacketMetadata, wire::DnsQueryType};
use sntpc::{NtpContext, NtpTimestampGenerator, get_time};

use crate::{
    display::update_status,
    error::{NetError, SysError},
    io::rtc::set_time,
};

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

        let ntp_addrs = stack.dns_query(NTP_SERVER, DnsQueryType::A).await?;
        if ntp_addrs.is_empty() {
            return Err(SysError::Net(NetError::Resolve));
        };
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
            Err(_) => Err(SysError::TimerSetup),
        }
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
