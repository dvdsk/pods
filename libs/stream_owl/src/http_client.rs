use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bytes::Bytes;
use http::header::InvalidHeaderValue;
use http::method::Method;
use http::uri::InvalidUri;
use http::{header, HeaderValue, Request, StatusCode};
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use hyper::client::conn::http1::{self, SendRequest};
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
    #[error("Host returned error,\n\tcode: {code}\n\tbody: {body:?}")]
    StatusNotOk {
        code: StatusCode,
        body: Option<String>,
    },
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

impl Error {
    async fn status_not_ok(response: hyper::Response<Incoming>) -> Self {
        let code = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .ok()
            .map(|body| body.to_bytes().to_vec())
            .map(|bytes| String::from_utf8(bytes).ok())
            .flatten();
        return Self::StatusNotOk { code, body };
    }
}

pub(crate) struct Client {
    host: HeaderValue,
    url: hyper::Uri,
    conn: Connection,
}

struct Connection {
    request_sender: SendRequest<Empty<Bytes>>,
    // when the joinset drops the connection is ended
    connection: JoinSet<()>,
}

type HyperResponse = hyper::Response<Incoming>;
impl Connection {
    async fn new(url: &hyper::Uri, restriction: &Option<Network>) -> Result<Self, Error> {
        let tcp = new_tcp_stream(&url, &restriction).await?;
        let io = ThrottlableIo::new(tcp);
        let (request_sender, conn) = http1::handshake(io).await?;

        let mut connection = JoinSet::new();
        connection.spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Error in connection: {}", e);
            }
        });
        Ok(Self {
            request_sender,
            connection,
        })
    }

    async fn send_request(
        &mut self,
        url: &hyper::Uri,
        cookies: &Cookies,
    ) -> Result<HyperResponse, Error> {
        let host = url.host().ok_or(Error::UrlWithoutHost)?;
        let host = HeaderValue::from_str(host).map_err(Error::InvalidHost)?;
        // todo!("url encoded stuff must become headers for some reason")
        let mut request = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header(header::HOST, host.clone())
            .header(header::USER_AGENT, "stream-owl")
            .header(header::ACCEPT, "*/*")
            .header(header::CONNECTION, "keep-alive");
        cookies.add_to(&mut request);
        let request = request.body(Empty::<Bytes>::new())?;
        let response = self.request_sender.send_request(dbg!(request)).await?;
        Ok(response)
    }
}

struct Cookies(Vec<String>);
impl Cookies {
    fn get_from(&mut self, response: &HyperResponse) {
        let new = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|line| line.to_str().ok())
            .filter_map(|line| line.split_once(";"))
            .map(|(cookie, _meta)| cookie.to_string());
        self.0.extend(new);
    }

    fn add_to(&self, request: &mut http::request::Builder) {
        let headers = request.headers_mut().expect("builder never has an error");
        for cookie in &self.0 {
            let cookie =
                HeaderValue::from_str(cookie.as_str()).expect("was a valid header value before");
            headers.insert(header::COOKIE, cookie.clone());
        }
    }
}

impl Client {
    pub(crate) async fn new(
        mut url: hyper::Uri,
        restriction: Option<Network>,
    ) -> Result<Client, Error> {
        let mut conn = Connection::new(&url, &restriction).await?;
        let mut cookies = Cookies(Vec::new());
        let mut response = conn.send_request(&url, &cookies).await?;
        cookies.get_from(&response);

        let mut numb_redirect = 0;
        let mut prev_url = url.clone();

        while response.status() == StatusCode::FOUND {
            if numb_redirect > 10 {
                return Err(Error::TooManyRedirects);
            }
            url = redirect_url(response)?;
            if url.host() != prev_url.host() {
                prev_url = url.clone();
                conn = Connection::new(&url, &restriction).await?;
            }
            response = conn.send_request(&url, &cookies).await?;
            cookies.get_from(&response);

            println!("redirecting to: {url}");
            numb_redirect += 1
        }

        if !response.status().is_success() {
            return Err(Error::status_not_ok(response).await);
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
        println!("body: {:#?}", body.collect().await);

        Ok(Client {
            host: url.host().unwrap().parse().unwrap(),
            url,
            conn,
        })
    }

    /// Panics if pos_1 is smaller then pos_2
    pub(crate) async fn get_range(&mut self, pos_1: u64, pos_2: u64) -> Result<Response, Error> {
        assert!(pos_1 < pos_2);

        let range = format!("Range: bytes={pos_1}-{pos_2}");
        let request = Request::builder()
            .method(Method::GET)
            .uri(self.url.clone())
            .header(header::HOST, self.host.clone())
            .header(header::USER_AGENT, "stream-owl")
            .header(header::ACCEPT, "*/*")
            .header(header::CONNECTION, "keep-alive")
            .header(header::RANGE, range)
            .body(Empty::<Bytes>::new())?;

        let response = self.conn.request_sender.send_request(request).await?;
        match response.status() {
            StatusCode::PARTIAL_CONTENT => Ok(Response::Range(response.into_body())),
            StatusCode::RANGE_NOT_SATISFIABLE => Err(Error::InvalidRange),
            // entire body is send at once
            StatusCode::OK => Ok(Response::All(response.into_body())),
            _ => Err(Error::status_not_ok(response).await),
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
    dbg!(&connect_addr);
    Ok(socket
        .connect(connect_addr)
        .await
        .map_err(Error::Connecting)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    // feed url: 274- The Age of the Algorithm
    const FEED_URL: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";
    const REDIR_URL: &str = "http://stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3/default.mp3_ywr3ahjkcgo_c288ef3e9f147075ce20a657c0c05108_20203379.mp3?aid=rss_feed&amp;awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&amp;awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&amp;feed=BqbsxVfO&hash_redirect=1&x-total-bytes=20203379&x-ais-classified=unclassified&listeningSessionID=0CD_382_295__75b258bb6b5c08fb4943101f0901735a80c29237";

    #[test]
    fn wtf() {
        let url = hyper::Uri::from_static(REDIR_URL);
        let query = url.query().unwrap_or("");
        let headers: Vec<(String, String)> = serde_urlencoded::from_str(query).unwrap();
        dbg!(headers);
        panic!();
    }

    #[tokio::test]
    async fn stream_works() {
        let url = hyper::Uri::from_static(FEED_URL);
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
        let url = hyper::Uri::from_static("http://www.example.org");
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
