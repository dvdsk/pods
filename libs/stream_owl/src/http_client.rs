use derivative::Derivative;
use http::header::InvalidHeaderValue;
use http::uri::InvalidUri;
use http::{header, HeaderValue, StatusCode};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use tracing::debug;

use crate::network::Network;
mod io;
mod read;
use read::Reader;
mod size;
pub(crate) use size::Size;

mod connection;
use connection::Connection;

use self::connection::HyperResponse;
use self::read::InnerReader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // #[error("Error in connection to stream server, {source}")]
    // Hyper {
    //     #[from]
    //     source: hyper::Error,
    // },
    #[error("Error setting up the stream request, {0}")]
    Http(#[from] http::Error),
    #[error("Error creating socket, {0}")]
    SocketCreation(std::io::Error),
    #[error("Could not restrict traffic to one network interface, {0}")]
    Restricting(std::io::Error),
    #[error("Could not connect to server, {0}")]
    Connecting(std::io::Error),
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
pub(crate) struct ClientStreamingPartial {
    stream: Incoming,
    inner: InnerClient,
    size: Size,
}

impl ClientStreamingPartial {
    #[tracing::instrument(level = "trace")]
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            inner,
            size,
        } = self;
        Reader::PartialData(InnerReader::new(stream, inner, size))
    }

    pub(crate) fn stream_size(&self) -> Size {
        self.size.clone()
    }

    pub(crate) fn builder(&self) -> ClientBuilder {
        ClientBuilder {
            restriction: self.inner.restriction.clone(),
            url: self.inner.url.clone(),
            cookies: self.inner.cookies.clone(),
            size: self.size.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ClientStreamingAll {
    stream: Incoming,
    inner: InnerClient,
    size: Size,
}

impl ClientStreamingAll {
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            inner,
            size,
        } = self;
        Reader::AllData(InnerReader::new(stream, inner, size))
    }

    pub(crate) fn builder(&self) -> ClientBuilder {
        ClientBuilder {
            restriction: self.inner.restriction.clone(),
            url: self.inner.url.clone(),
            cookies: self.inner.cookies.clone(),
            size: self.size.clone(),
        }
    }

    pub(crate) fn stream_size(&self) -> Size {
        self.size.clone()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct InnerClient {
    host: HeaderValue,
    restriction: Option<Network>,
    url: hyper::Uri,
    #[derivative(Debug = "ignore")]
    conn: Connection,
    cookies: Cookies,
}

impl InnerClient {
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
pub struct Client {
    should_support_range: bool,
    size: Size,
    inner: InnerClient,
}

#[derive(Debug)]
pub(crate) enum StreamingClient {
    Partial(ClientStreamingPartial),
    All(ClientStreamingAll),
}

#[derive(Debug, Clone)]
pub(crate) struct ClientBuilder {
    restriction: Option<Network>,
    url: hyper::Uri,
    cookies: Cookies,
    size: Size,
}

impl ClientBuilder {
    #[tracing::instrument(level = "debug")]
    pub(crate) async fn connect(self, start: u64, len: u64) -> Result<StreamingClient, Error> {
        let Self {
            restriction,
            mut url,
            mut cookies,
            mut size,
        } = self;

        let end = if let Some(size) = size.known() {
            (start + len).min(size)
        } else {
            start + len
        };
        let first_range = format!("bytes={start}-{end}");

        let mut conn = Connection::new(&url, &restriction).await?;
        let mut response = conn
            .send_initial_request(&url, &cookies, &first_range)
            .await?;
        size.update_from_headers(&response);
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
            response = conn
                .send_initial_request(&url, &cookies, &first_range)
                .await?;
            size.update_from_headers(&response);
            cookies.get_from(&response);

            debug!("redirecting to: {url}");
            numb_redirect += 1
        }

        let host = url.host().unwrap().parse().unwrap();
        let inner = InnerClient {
            host,
            restriction,
            url,
            conn,
            cookies,
        };
        match response.status() {
            StatusCode::OK => Ok(StreamingClient::All(ClientStreamingAll {
                stream: response.into_body(),
                inner,
                size,
            })),
            StatusCode::PARTIAL_CONTENT => Ok(StreamingClient::Partial(ClientStreamingPartial {
                stream: response.into_body(),
                inner,
                size,
            })),
            StatusCode::RANGE_NOT_SATISFIABLE => {
                tracing::info!("{response:?}");
                todo!("redo without range")
            }
            _ => Err(Error::status_not_ok(response).await),
        }
    }
}

impl StreamingClient {
    #[tracing::instrument(level = "debug", ret)]
    pub(crate) async fn new(
        url: hyper::Uri,
        restriction: Option<Network>,
        init_start: u64,
        init_len: u64,
        size: Size,
    ) -> Result<Self, Error> {
        ClientBuilder {
            restriction,
            url,
            cookies: Cookies::new(),
            size,
        }
        .connect(init_start, init_len)
        .await
    }

    pub(crate) fn stream_size(&self) -> Size {
        match self {
            StreamingClient::Partial(client) => client.stream_size(),
            StreamingClient::All(client) => client.stream_size(),
        }
    }
}

impl Client {
    #[tracing::instrument(level = "debug", err, ret)]
    pub(crate) async fn try_get_range(
        mut self,
        start: u64,
        chunk_size: u64,
    ) -> Result<StreamingClient, Error> {
        assert!(Some(start) < self.stream_size().known());
        let end = start + chunk_size;

        let range = if Some(end) >= self.stream_size().known() {
            format!("bytes={start}-") // download till end of stream
        } else {
            format!("bytes={start}-{end}")
        };

        let response = self.inner.send_range_request(&range).await?;

        self.size.update_from_headers(&response);
        match response.status() {
            StatusCode::OK => Ok(StreamingClient::All(ClientStreamingAll {
                stream: response.into_body(),
                inner: self.inner,
                size: self.size,
            })),
            StatusCode::PARTIAL_CONTENT => Ok(StreamingClient::Partial(ClientStreamingPartial {
                stream: response.into_body(),
                inner: self.inner,
                size: self.size,
            })),
            StatusCode::RANGE_NOT_SATISFIABLE => return Err(Error::InvalidRange),
            _ => Err(Error::status_not_ok(response).await),
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

#[cfg(test)]
mod tests {
    use crate::Appender;

    use super::*;

    #[async_trait::async_trait]
    impl Appender for &mut Vec<u8> {
        async fn append(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            self.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    // feed url: 274- The Age of the Algorithm
    const FEED_URL: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";

    #[tokio::test]
    async fn get_stream_client() {
        let url = hyper::Uri::from_static(FEED_URL);
        let client = StreamingClient::new(url, None, 0, 1024, Size::default())
            .await
            .unwrap();
        assert!(client.stream_size().known().is_some());

        match client {
            StreamingClient::Partial(_) => (),
            StreamingClient::All(client) => {
                let mut buf = Vec::new();
                client
                    .into_reader()
                    .read_to_writer(&mut buf, None)
                    .await
                    .unwrap();
                let len = buf.len();
                dbg!(len);
                panic!("should get partial streaming client")
            }
        };
    }

    #[tokio::test]
    async fn state_machine_works() {
        const CHUNK_SIZE: u64 = 1_000_000; // 1MB
        let url = hyper::Uri::from_static(FEED_URL);
        let mut client = StreamingClient::new(url, None, 0, CHUNK_SIZE, Size::default())
            .await
            .unwrap();
        let mut buffer = Vec::new();
        loop {
            match client {
                StreamingClient::Partial(client_with_stream) => {
                    let content_size = client_with_stream
                        .stream_size()
                        .known()
                        .expect("test stream should provide size");
                    let mut reader = client_with_stream.into_reader();
                    reader
                        .read_to_writer(&mut buffer, Some(CHUNK_SIZE as usize))
                        .await
                        .unwrap();

                    if buffer.len() as u64 >= content_size {
                        return;
                    }

                    client = reader
                        .try_into_client()
                        .unwrap()
                        .try_get_range(buffer.len() as u64, CHUNK_SIZE)
                        .await
                        .unwrap();
                }
                StreamingClient::All(client_with_stream) => {
                    let mut reader = client_with_stream.into_reader();
                    reader.read_to_writer(&mut buffer, None).await.unwrap();
                    break;
                }
            }
        }
    }
}
