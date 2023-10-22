use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bytes::Bytes;
use http::header::InvalidHeaderValue;
use http::method::Method;
use http::uri::InvalidUri;
use http::{header, HeaderValue, Request, StatusCode};
use http_body_util::Empty;
use hyper::client::conn;
use hyper::client::conn::http1::SendRequest;
use tokio::net::{TcpSocket, TcpStream};
use tokio::task::JoinSet;

use crate::network::Network;
mod io;
use io::ThrottlableIo;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error in connection to stream host, {source}")]
    Hyper {
        #[from]
        source: hyper::Error,
        // backtrace: Backtrace,
    },
    #[error("Error setting up the stream request, {0}")]
    Http(#[from] http::Error),
    #[error("Error creating socket, {0}")]
    SocketCreation(std::io::Error),
    #[error("Could not restrict traffic to one network interface, {0}")]
    Restricting(std::io::Error),
    #[error("Could not connect to host, {0}")]
    Connecting(std::io::Error),
    #[error("Could not resolve dns, resolve error, {0}")]
    DnsResolve(#[from] hickory_resolver::error::ResolveError),
    #[error("Could not resolve dns, no ip adresses for host")]
    DnsEmpty,
    #[error("Url had no host part")]
    UrlWithoutHost,
    #[error("Host returned error, {0}")]
    StatusNotOk(StatusCode),
    #[error("Host contained invalid characters, {0}")]
    InvalidHost(InvalidHeaderValue),
    #[error("Host does not report we can seek in streams")]
    RangesNotAccepted,
    #[error("Invalid range")]
    InvalidRange,
    #[error("Host redirected us however did not send location")]
    MissingRedirectLocation,
    #[error("The redirect location contained invalid characters, {0}")]
    BrokenRedirectLocation(header::ToStrError),
    #[error("The redirect location is not a url, {0}")]
    InvalidUriRedirectLocation(InvalidUri),
    #[error("Host redirected us more then 10 times")]
    TooManyRedirects,
}

pub(crate) struct Client {
    host: HeaderValue,
    request_sender: SendRequest<Empty<Bytes>>,
    // when the joinset drops the connection is ended
    connection: JoinSet<()>,
}

impl Client {
    pub(crate) async fn new(
        mut url: hyper::Uri,
        restriction: Option<Network>,
    ) -> Result<Client, Error> {
        let mut numb_redirect = 0;
        let mut sender = None;
        let mut connection = None;
        let mut host: HeaderValue;
        let mut prev_host = HeaderValue::from_str("").unwrap();

        let response = loop {
            if numb_redirect > 10 {
                return Err(Error::TooManyRedirects);
            }

            host = url
                .host()
                .ok_or(Error::UrlWithoutHost)?
                .parse()
                .map_err(Error::InvalidHost)?;

            if host != prev_host {
                prev_host = host.clone();
                let tcp = new_tcp_stream(&url, &restriction).await?;
                let io = ThrottlableIo::new(tcp);
                let (request_sender, http_conn) = conn::http1::handshake(io).await?;
                sender = Some(request_sender);

                connection = Some(JoinSet::new());
                connection.as_mut().unwrap().spawn(async move {
                    if let Err(e) = http_conn.await {
                        eprintln!("Error in connection: {}", e);
                    }
                });
            }

            let request = Request::builder()
                .method(Method::GET)
                .uri(url)
                .header(header::HOST, host.clone())
                .body(Empty::<Bytes>::new())?;
            dbg!(&request);

            let response = sender.as_mut().unwrap().send_request(request).await?;
            if response.status() == StatusCode::FOUND {
                url = redirect_url(response)?;
                println!("redirecting to: {url}");
                numb_redirect += 1
            } else {
                break response;
            }
        };

        dbg!(&response);
        if !response.status().is_success() {
            return Err(Error::StatusNotOk(response.status()));
        }

        let (parts, body) = response.into_parts();
        if !parts
            .headers
            .get(header::ACCEPT_RANGES)
            .is_some_and(|val| val == "bytes")
        {
            return Err(Error::RangesNotAccepted);
        }
        println!("headers: {:#?}", parts.headers);
        println!("body: {:#?}", body);

        Ok(Client {
            host,
            request_sender: sender.unwrap(),
            connection: connection.unwrap(),
        })
    }

    /// Panics if pos_1 is smaller then pos_2
    pub(crate) async fn get_range(&mut self, pos_1: u64, pos_2: u64) -> Result<Response, Error> {
        assert!(pos_1 < pos_2);

        let range = format!("Range: bytes={pos_1}-{pos_2}");
        let request = Request::builder()
            .header(header::HOST, self.host.clone())
            .method(Method::GET)
            .header(header::RANGE, range)
            .body(Empty::<Bytes>::new())?;

        let response = self.request_sender.send_request(request).await?;
        match response.status() {
            StatusCode::PARTIAL_CONTENT => Ok(Response::Range(response.into_body())),
            StatusCode::RANGE_NOT_SATISFIABLE => Err(Error::InvalidRange),
            // entire body is send at once
            StatusCode::OK => Ok(Response::All(response.into_body())),
            status => Err(Error::StatusNotOk(status)),
        }
    }
}

fn redirect_url<T>(redirect: hyper::Response<T>) -> Result<hyper::Uri, Error> {
    redirect
        .headers()
        .get(header::LOCATION)
        .ok_or(Error::MissingRedirectLocation)?
        .to_str()
        .map_err(Error::BrokenRedirectLocation)?
        .parse()
        .map_err(Error::InvalidUriRedirectLocation)
}

pub(crate) enum Response {
    Range(hyper::body::Incoming),
    All(hyper::body::Incoming),
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

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    // 274- The Age of the Algorithm
    const URL1: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";

    #[tokio::test]
    async fn stream_works() {
        let url = hyper::Uri::from_static(URL1);
        let mut client = Client::new(url, None).await.unwrap();
        let response = client.get_range(0, 1024).await;

        match response {
            Ok(Response::All(incoming)) | Ok(Response::Range(incoming)) => {
                incoming.collect().await.unwrap().to_bytes()
            }
            Err(Error::InvalidRange) => todo!(),
            Err(Error::RangesNotAccepted) => todo!(),
            Err(e) => unimplemented!("cant handle error: {e}"),
        };
    }

    #[tokio::test]
    async fn basic_http_works() {
        let url = hyper::Uri::from_static("www.example.org");
        let mut client = match Client::new(url, None).await {
            Ok(client) => client,
            Err(Error::RangesNotAccepted) => return,
            Err(e) => panic!("{e:?}"),
        };
        let response = client.get_range(0, 1024).await;

        match response {
            Ok(Response::All(incoming)) | Ok(Response::Range(incoming)) => {
                incoming.collect().await.unwrap().to_bytes()
            }
            Err(Error::InvalidRange) => todo!(),
            Err(Error::RangesNotAccepted) => todo!(),
            Err(e) => unimplemented!("cant handle error: {e}"),
        };
    }
}
