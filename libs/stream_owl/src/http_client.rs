use std::ops::Range;

use derivative::Derivative;
use http::header::InvalidHeaderValue;
use http::uri::InvalidUri;
use http::{header, HeaderValue, StatusCode};
use hyper::body::Incoming;
use tracing::{debug, info, warn};

use crate::network::{Network, BandwidthLim};
use crate::target::StreamTarget;
mod io;
mod read;
use read::Reader;
mod headers;
mod response;
mod size;
pub(crate) use size::Size;

mod connection;
use connection::Connection;

use self::connection::HyperResponse;
use self::read::InnerReader;
use self::response::ValidResponse;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    Response(#[from] response::Error),
    #[error("Error setting up the stream request, {0}")]
    Http(#[from] http::Error),
    #[error("Error creating socket, {0}")]
    SocketCreation(std::io::Error),
    #[error("Could not restrict traffic to one network interface, {0}")]
    Restricting(std::io::Error),
    #[error("Could not connect to server, {0}")]
    Connecting(std::io::Error),
    #[error("Could not configure socket, {0}")]
    SocketConfig(std::io::Error),
    #[error("Could not resolve dns, resolve error, {0}")]
    DnsResolve(#[from] hickory_resolver::error::ResolveError),
    #[error("Could not resolve dns, no ip addresses for server")]
    DnsEmpty,
    #[error("Url had no server part")]
    UrlWithoutHost,
    #[error("server returned error,\n\tcode: {code}\n\tbody: {body:?}")]
    StatusNotOk {
        code: StatusCode,
        body: Option<String>,
    },
    #[error("server contained invalid characters, {0}")]
    InvalidHost(InvalidHeaderValue),
    #[error("server does not report we can seek in streams")]
    RangesNotAccepted,
    #[error("Invalid range")]
    InvalidRange,
    #[error("server redirected us however did not send location")]
    MissingRedirectLocation,
    #[error("The redirect location contained invalid characters, {0}")]
    BrokenRedirectLocation(header::ToStrError),
    #[error("The redirect location is not a url, {0}")]
    InvalidUriRedirectLocation(InvalidUri),
    #[error("server redirected us more then 10 times")]
    TooManyRedirects,
    #[error("Server did not send any data")]
    MissingFrame,
    #[error("Server send a PARTIAL_CONTENT response without range header or we did not understand the range")]
    MissingRange,
    #[error("Could not send request to server: {0}")]
    SendingRequest(hyper::Error),
    #[error("Could not set up connection to server: {0}")]
    Handshake(hyper::Error),
    #[error("Could not read response body: {0}")]
    ReadingBody(hyper::Error),
    #[error("Could now write the received data to storage: {0}")]
    WritingData(std::io::Error),
    #[error("Could not throw away body: {0}")]
    EmptyingBody(hyper::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct Cookies(Vec<String>);
impl Cookies {
    fn new() -> Self {
        Self(Vec::new())
    }

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

/// A client that is currently streaming partial content
/// (the result of a range request)
#[derive(Debug)]
pub(crate) struct RangeSupported {
    range: Range<u64>,
    stream: Incoming,
    client: Client,
}

impl RangeSupported {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            client,
            range,
        } = self;
        Reader::PartialData {
            inner: InnerReader::new(stream, client),
            range,
        }
    }

    pub(crate) fn stream_size(&self) -> Size {
        self.client.size.clone()
    }

    pub(crate) fn builder(&self) -> ClientBuilder {
        ClientBuilder {
            restriction: self.client.restriction.clone(),
            bandwidth_lim: self.client.bandwidth_lim.clone(),
            url: self.client.url.clone(),
            cookies: self.client.cookies.clone(),
            size: self.client.size.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RangeRefused {
    stream: Incoming,
    size: Option<u64>,
    client: Client,
}

impl RangeRefused {
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            client,
            size,
        } = self;
        info!("Seeking not supported by server, receiving all data in one stream.");
        if let Some(size) = size {
            info!("Total stream size: {size}");
        }
        Reader::AllData {
            inner: InnerReader::new(stream, client),
        }
    }

    pub(crate) fn builder(&self) -> ClientBuilder {
        ClientBuilder {
            restriction: self.client.restriction.clone(),
            bandwidth_lim: self.client.bandwidth_lim.clone(),
            url: self.client.url.clone(),
            cookies: self.client.cookies.clone(),
            size: self.client.size.clone(),
        }
    }

    pub(crate) fn stream_size(&self) -> Size {
        self.client.size.clone()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Client {
    host: HeaderValue,
    restriction: Option<Network>,
    #[derivative(Debug = "ignore")]
    bandwidth_lim: BandwidthLim,
    url: hyper::Uri,
    #[derivative(Debug = "ignore")]
    conn: Connection,
    size: Size,
    cookies: Cookies,
}

impl Client {
    #[tracing::instrument(level = "trace", skip(self), ret)]
    async fn send_range_request(
        &mut self,
        range: &str,
    ) -> Result<hyper::Response<Incoming>, Error> {
        let response = self
            .conn
            .send_range_request(&self.url, &self.host, &self.cookies, range)
            .await?;
        self.cookies.get_from(&response);
        Ok(response)
    }
}

#[derive(Debug)]
pub(crate) enum StreamingClient {
    RangesSupported(RangeSupported),
    RangesRefused(RangeRefused),
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub(crate) struct ClientBuilder {
    restriction: Option<Network>,
    #[derivative(Debug = "ignore")]
    bandwidth_lim: BandwidthLim,
    url: hyper::Uri,
    cookies: Cookies,
    size: Size,
}

impl ClientBuilder {
    #[tracing::instrument(level = "debug")]
    pub(crate) async fn connect(self, target: &StreamTarget) -> Result<StreamingClient, Error> {
        debug!("(re)connecting, will try to start stream at: {}", target.pos());
        let Self {
            restriction,
            bandwidth_lim,
            mut url,
            mut cookies,
            mut size,
        } = self;

        let Range { start, end } = target
            .next_range(&size)
            .await
            .expect("should be a range to get after seek or on connect");
        let first_range = format!("bytes={start}-{end}");

        let mut conn = Connection::new(&url, &restriction, &bandwidth_lim).await?;
        let mut response = conn
            .send_initial_request(&url, &cookies, &first_range)
            .await?;
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
                conn = Connection::new(&url, &restriction, &bandwidth_lim).await?;
            }
            response = conn
                .send_initial_request(&url, &cookies, &first_range)
                .await?;
            cookies.get_from(&response);

            debug!("redirecting to: {url}");
            numb_redirect += 1
        }

        use ValidResponse::*;
        let response = ValidResponse::try_from(response)?;
        let host = url.host().unwrap().parse().unwrap();
        size.update(&response);

        let client = Client {
            host,
            restriction,
            bandwidth_lim,
            url,
            conn,
            cookies,
            size,
        };

        match response {
            Ok { stream, size } => Ok(StreamingClient::RangesRefused(RangeRefused {
                stream,
                size,
                client,
            })),
            PartialContent { stream, range, .. } => {
                Ok(StreamingClient::RangesSupported(RangeSupported {
                    range,
                    stream,
                    client,
                }))
            }

            RangeNotSatisfiable { size } => {
                tracing::info!("{response:?}, {size:?}");
                todo!("redo without range")
            }
        }
    }
}

impl StreamingClient {
    #[tracing::instrument(level = "debug", skip(bandwidth_lim), ret)]
    pub(crate) async fn new(
        url: hyper::Uri,
        restriction: Option<Network>,
        bandwidth_lim: BandwidthLim,
        size: Size,
        target: &StreamTarget,
    ) -> Result<Self, Error> {
        ClientBuilder {
            restriction,
            bandwidth_lim,
            url,
            cookies: Cookies::new(),
            size,
        }
        .connect(target)
        .await
    }

    pub(crate) fn stream_size(&self) -> Size {
        match self {
            StreamingClient::RangesSupported(client) => client.stream_size(),
            StreamingClient::RangesRefused(client) => client.stream_size(),
        }
    }
}

impl Client {
    #[tracing::instrument(level = "debug", err, ret)]
    pub(crate) async fn try_get_range(
        mut self,
        Range { start, end }: Range<u64>,
    ) -> Result<StreamingClient, Error> {
        assert!(Some(start) < self.stream_size().known());

        let range = format!("bytes={start}-{end}");
        let response = self.send_range_request(&range).await?;
        let response = ValidResponse::try_from(response)?;

        self.size.update(&response);
        match response {
            ValidResponse::Ok { stream, size } => {
                Ok(StreamingClient::RangesRefused(RangeRefused {
                    stream,
                    size,
                    client: self,
                }))
            }
            ValidResponse::PartialContent { stream, range, .. } => {
                Ok(StreamingClient::RangesSupported(RangeSupported {
                    range,
                    stream,
                    client: self,
                }))
            }
            ValidResponse::RangeNotSatisfiable { .. } => todo!(),
        }
    }

    pub(crate) fn stream_size(&self) -> Size {
        self.size.clone()
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
