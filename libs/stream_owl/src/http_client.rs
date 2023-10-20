use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bytes::Bytes;
use http::{Request, StatusCode};
use http_body_util::Empty;
use hyper::client::conn;
use tokio::net::{TcpSocket, TcpStream};

use crate::network::Network;
mod io;
use io::ThrottlableIo;

pub(crate) struct Client {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error in connection to stream host")]
    Hyper(#[from] hyper::Error),
    #[error("Error setting up the stream request")]
    Http(#[from] http::Error),
    #[error("Error creating socket")]
    SocketCreation(std::io::Error),
    #[error("Could not restrict traffic to one network interface")]
    Restricting(std::io::Error),
    #[error("Could not connect to host")]
    Connecting(std::io::Error),
    #[error("Could not resolve dns, resolve error")]
    DnsResolve(#[from] hickory_resolver::error::ResolveError),
    #[error("Could not resolve dns, no ip adresses for host")]
    DnsEmpty,
    #[error("Url had no host part")]
    UrlWithoutHost,
}

async fn resolve_dns(host: &str) -> Result<IpAddr, Error> {
    use hickory_resolver::config::{ResolverConfig, ResolverOpts};
    use hickory_resolver::TokioAsyncResolver;
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    resolver
        .lookup_ip(host)
        .await?
        .iter()
        .next()
        .ok_or(Error::DnsEmpty)
}

async fn new_tcp_stream(url: &hyper::Uri, restriction: Option<Network>) -> Result<TcpStream, Error> {
    let bind_addr = restriction
        .map(Network::addr)
        .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
    let bind_addr = SocketAddr::new(bind_addr, 0);

    let host = url.host().expect("stream urls always have a host");
    let host = resolve_dns(host).await?;
    let port = url.port().map(|p| p.as_u16()).unwrap_or(80);
    let connect_addr = SocketAddr::new(host, port);

    let socket = TcpSocket::new_v4().map_err(Error::SocketCreation)?;
    socket.bind(bind_addr).map_err(Error::Restricting)?;
    Ok(socket
        .connect(connect_addr)
        .await
        .map_err(Error::Connecting)?)
}

struct Io {}

impl Client {
    pub async fn new(url: hyper::Uri, restriction: Option<Network>) -> Result<Client, Error> {
        let tcp = new_tcp_stream(&url, restriction).await?;
        let io = ThrottlableIo::new(tcp);
        let (mut request_sender, connection) = conn::http1::handshake(io).await?;

        if let Err(e) = connection.await {
            eprintln!("Error in connection: {}", e);
        }

        let host = url.host().ok_or(Error::UrlWithoutHost)?;
        let request = Request::builder()
            // We need to manually add the host header because SendRequest does not
            .header("Host", host)
            .method("GET")
            .body(Empty::<Bytes>::new())?;

        let response = request_sender.send_request(request).await?;
        assert!(response.status() == StatusCode::OK);

        Ok(Client {})
    }

    pub(crate) fn get_range(&self, pos_1: u64, pos_2: u64) -> Result<(), Error> {
        todo!()
    }
}
