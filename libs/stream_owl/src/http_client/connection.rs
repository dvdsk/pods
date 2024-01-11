use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bytes::Bytes;
use http::method::Method;
use http::{header, HeaderValue, Request};
use http_body_util::Empty;
use hyper::body::Incoming;
use hyper::client::conn::http1::{self, SendRequest};
use tokio::net::{TcpSocket, TcpStream};
use tokio::task::JoinSet;
use tracing::instrument;

use crate::network::{Network, BandwidthLim};

use super::io::ThrottlableIo;
use super::{Cookies, Error};

#[derive(Debug)]
pub(crate) struct Connection {
    pub request_sender: SendRequest<Empty<Bytes>>,
    // when the joinset drops the connection is ended
    _connection: JoinSet<()>,
}

pub(crate) type HyperResponse = hyper::Response<Incoming>;
impl Connection {
    pub(crate) async fn new(
        url: &hyper::Uri,
        restriction: &Option<Network>,
        bandwidth_lim: &BandwidthLim,
    ) -> Result<Self, Error> {
        let tcp = new_tcp_stream(&url, &restriction).await?;
        let io = ThrottlableIo::new(tcp, bandwidth_lim).map_err(Error::SocketConfig)?;
        let (request_sender, conn) = http1::handshake(io).await.map_err(Error::Handshake)?;

        let mut connection = JoinSet::new();
        connection.spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Error in connection: {}", e);
            }
        });
        Ok(Self {
            request_sender,
            _connection: connection,
        })
    }

    #[instrument(level = "Debug", skip(self), ret, err)]
    pub(crate) async fn send_initial_request(
        &mut self,
        url: &hyper::Uri,
        cookies: &Cookies,
        range: &str,
    ) -> Result<HyperResponse, Error> {
        let host = url.host().ok_or(Error::UrlWithoutHost)?;
        let host = HeaderValue::from_str(host).map_err(Error::InvalidHost)?;
        let mut request = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header(header::HOST, host.clone())
            .header(header::USER_AGENT, "stream-owl")
            .header(header::ACCEPT, "*/*")
            .header(header::CONNECTION, "keep-alive")
            .header(header::RANGE, range);

        cookies.add_to(&mut request);
        let request = request.body(Empty::<Bytes>::new())?;

        let response = self
            .request_sender
            .send_request(request)
            .await
            .map_err(Error::SendingRequest)?;
        Ok(response)
    }

    pub(crate) async fn send_range_request(
        &mut self,
        url: &hyper::Uri,
        host: &HeaderValue,
        cookies: &Cookies,
        range: &str,
    ) -> Result<HyperResponse, Error> {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri(url.clone())
            .header(header::HOST, host.clone())
            .header(header::USER_AGENT, "stream-owl")
            .header(header::ACCEPT, "*/*")
            .header(header::CONNECTION, "keep-alive")
            .header(header::RANGE, range);

        cookies.add_to(&mut request);
        let request = request.body(Empty::<Bytes>::new())?;

        let response = self
            .request_sender
            .send_request(request)
            .await
            .map_err(Error::SendingRequest)?;
        Ok(response)
    }
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

async fn new_tcp_stream(
    url: &hyper::Uri,
    restriction: &Option<Network>,
) -> Result<TcpStream, Error> {
    let bind_addr = restriction
        .as_ref()
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
