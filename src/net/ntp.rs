use chrono::{DateTime, TimeDelta, Utc};
use core::net::{IpAddr, SocketAddr};
use embassy_net::{Stack, udp::UdpSocket};
use embassy_net::{IpEndpoint, IpAddress};
use embassy_time::{Duration, Instant, Timer};
use smoltcp::{storage::PacketMetadata, wire::DnsQueryType};
use sntpc::{NtpContext, NtpTimestampGenerator, NtpUdpSocket, get_time};

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

impl NtpTimestampGenerator for Timestamp {
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
        (self.duration.as_micros() - self.duration.as_secs() * 1_000_000)
            .try_into()
            .unwrap()
    }
}

/// Wrapper around embassy-net 0.9 UdpSocket that implements sntpc's NtpUdpSocket trait.
struct UdpSocketWrapper<'a> {
    socket: UdpSocket<'a>,
}

impl<'a> UdpSocketWrapper<'a> {
    fn new(socket: UdpSocket<'a>) -> Self {
        Self { socket }
    }
}

fn to_endpoint(addr: SocketAddr) -> IpEndpoint {
    IpEndpoint::new(
        match addr.ip() {
            IpAddr::V4(addr) => IpAddress::Ipv4(addr),
            IpAddr::V6(_) => unreachable!(),
        },
        addr.port(),
    )
}

fn from_endpoint(ep: IpEndpoint) -> SocketAddr {
    let IpAddress::Ipv4(val) = ep.addr;
    SocketAddr::new(IpAddr::V4(val), ep.port)
}

impl NtpUdpSocket for UdpSocketWrapper<'_> {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> sntpc::Result<usize> {
        let endpoint = to_endpoint(addr);
        match self.socket.send_to(buf, endpoint).await {
            Ok(()) => Ok(buf.len()),
            Err(_) => Err(sntpc::Error::Network),
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> sntpc::Result<(usize, SocketAddr)> {
        match self.socket.recv_from(buf).await {
            Ok((len, meta)) => Ok((len, from_endpoint(meta.endpoint))),
            Err(_) => Err(sntpc::Error::Network),
        }
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

        let socket = UdpSocket::new(
            *stack,
            &mut udp_rx_meta,
            &mut udp_rx_buffer,
            &mut udp_tx_meta,
            &mut udp_tx_buffer,
        );
        let mut socket = UdpSocketWrapper::new(socket);
        socket.socket.bind(123).unwrap();

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
        let timeout;
        update_status("Syncing NTP").await.ok();
        if let Ok(()) = client.sync().await {
            update_status("Time synced").await.ok();
            timeout = NTP_REFRESH_TIME;
        } else {
            update_status("NTP failed, proceeding").await.ok();
            timeout = Duration::from_secs(5);
        };
        Timer::after(timeout).await;
    }
}
