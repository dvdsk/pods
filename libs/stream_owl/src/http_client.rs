use bytes::Bytes;
use http::header::InvalidHeaderValue;
use http::method::Method;
use http::uri::InvalidUri;
use http::{header, HeaderValue, Request, StatusCode};
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;

use crate::network::Network;
mod io;
mod read;
use read::Reader;
mod size_hint;
use size_hint::SizeHint;

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
    #[error("Could not resolve dns, no ip adresses for server")]
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
    #[error("Could now write the recieved data to storage: {0}")]
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

pub(crate) struct Cookies(Vec<String>);
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

pub(crate) struct ClientStreamingPartial {
    stream: Incoming,
    inner: InnerClient,
    size_hint: SizeHint,
}

impl ClientStreamingPartial {
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            inner,
            size_hint,
        } = self;
        Reader::PartialData(InnerReader::new(stream, inner, size_hint))
    }

    pub(crate) fn size_hint(&self) -> SizeHint {
        self.size_hint
    }
}

pub(crate) struct ClientStreamingAll {
    stream: Incoming,
    inner: InnerClient,
    size_hint: SizeHint,
}

impl ClientStreamingAll {
    pub(crate) fn into_reader(self) -> Reader {
        let Self {
            stream,
            inner,
            size_hint,
        } = self;
        Reader::AllData(InnerReader::new(stream, inner, size_hint))
    }

    pub(crate) fn size_hint(&self) -> SizeHint {
        todo!()
    }
}

pub(crate) struct InnerClient {
    host: HeaderValue,
    url: hyper::Uri,
    conn: Connection,
}

pub struct Client {
    should_support_range: bool,
    size_hint: SizeHint,
    inner: InnerClient,
}

pub(crate) enum StreamingClient {
    Partial(ClientStreamingPartial),
    All(ClientStreamingAll),
}

impl StreamingClient {
    pub(crate) async fn new(
        mut url: hyper::Uri,
        restriction: Option<Network>,
        init_start: u64,
        init_len: u64,
    ) -> Result<Self, Error> {
        let mut size_hint = SizeHint::default();
        let mut conn = Connection::new(&url, &restriction).await?;
        let mut cookies = Cookies(Vec::new());
        let first_range = format!("bytes={init_start}-{init_len}");
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
                conn = Connection::new(&url, &restriction).await?;
            }
            response = conn
                .send_initial_request(&url, &cookies, &first_range)
                .await?;
            size_hint.update_from_headers(response.headers());
            cookies.get_from(&response);

            println!("redirecting to: {url}");
            numb_redirect += 1
        }

        let host = url.host().unwrap().parse().unwrap();
        let inner = InnerClient { host, url, conn };
        match response.status() {
            StatusCode::OK => Ok(StreamingClient::All(ClientStreamingAll {
                stream: response.into_body(),
                inner,
                size_hint,
            })),
            StatusCode::PARTIAL_CONTENT => Ok(StreamingClient::Partial(ClientStreamingPartial {
                stream: response.into_body(),
                inner,
                size_hint,
            })),
            StatusCode::RANGE_NOT_SATISFIABLE => todo!("redo without range"),
            _ => Err(Error::status_not_ok(response).await),
        }
    }

    pub(crate) fn size_hint(&self) -> SizeHint {
        match self {
            StreamingClient::Partial(client) => client.size_hint(),
            StreamingClient::All(client) => client.size_hint(),
        }
    }
}

impl Client {
    /// Panics if pos_1 is smaller then pos_2
    pub(crate) async fn try_get_range(
        mut self,
        start: u64,
        len: u64,
    ) -> Result<StreamingClient, Error> {
        let range = format!("bytes={start}-{}", start + len);
        let request = Request::builder()
            .method(Method::GET)
            .uri(self.inner.url.clone())
            .header(header::HOST, self.inner.host.clone())
            .header(header::USER_AGENT, "stream-owl")
            .header(header::ACCEPT, "*/*")
            .header(header::CONNECTION, "keep-alive")
            .header(header::RANGE, range)
            .body(Empty::<Bytes>::new())?;

        let response = self
            .inner
            .conn
            .request_sender
            .send_request(request)
            .await
            .map_err(Error::SendingRequest)
            .unwrap();

        self.size_hint.update_from_headers(response.headers());
        match response.status() {
            StatusCode::OK => Ok(StreamingClient::All(ClientStreamingAll {
                stream: response.into_body(),
                inner: self.inner,
                size_hint: self.size_hint,
            })),
            StatusCode::PARTIAL_CONTENT => Ok(StreamingClient::Partial(ClientStreamingPartial {
                stream: response.into_body(),
                inner: self.inner,
                size_hint: self.size_hint,
            })),
            StatusCode::RANGE_NOT_SATISFIABLE => return Err(Error::InvalidRange),
            _ => Err(Error::status_not_ok(response).await),
        }
    }

    pub(crate) fn size_hint(&self) -> SizeHint {
        self.size_hint
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
    use super::*;

    // feed url: 274- The Age of the Algorithm
    const FEED_URL: &str = "https://dts.podtrac.com/redirect.mp3/chrt.fm/track/288D49/stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3?aid=rss_feed&awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&feed=BqbsxVfO";
    const REDIR_URL: &str = "http://stitcher.simplecastaudio.com/3bb687b0-04af-4257-90f1-39eef4e631b6/episodes/c660ce6b-ced1-459f-9535-113c670e83c9/audio/128/default.mp3/default.mp3_ywr3ahjkcgo_c288ef3e9f147075ce20a657c0c05108_20203379.mp3?aid=rss_feed&amp;awCollectionId=3bb687b0-04af-4257-90f1-39eef4e631b6&amp;awEpisodeId=c660ce6b-ced1-459f-9535-113c670e83c9&amp;feed=BqbsxVfO&hash_redirect=1&x-total-bytes=20203379&x-ais-classified=unclassified&listeningSessionID=0CD_382_295__75b258bb6b5c08fb4943101f0901735a80c29237";

    #[tokio::test]
    async fn get_stream_client() {
        let url = hyper::Uri::from_static(FEED_URL);
        let client = StreamingClient::new(url, None, 0, 1024).await.unwrap();
        dbg!(client.size_hint());
        assert!(client.size_hint().highest_estimate().is_some());

        match client {
            StreamingClient::Partial(_) => (),
            StreamingClient::All(client) => {
                let mut buf = Vec::new();
                client.into_reader().read(&mut buf, None).await.unwrap();
                let len = buf.len();
                dbg!(len);
                panic!("should get partial streaming client")
            }
        };
    }

    #[tokio::test]
    async fn state_machine_works() {
        const RANGE_LEN: u64 = 1_000_000; // 1MB
        let url = hyper::Uri::from_static(FEED_URL);
        let mut client = StreamingClient::new(url, None, 0, RANGE_LEN).await.unwrap();
        let mut buffer = Vec::new();
        loop {
            let content_size = client.size_hint().highest_estimate().unwrap();
            dbg!(content_size);
            dbg!(buffer.len());
            if buffer.len() as u64 >= content_size {
                return;
            }

            match client {
                StreamingClient::Partial(client_with_stream) => {
                    let mut reader = client_with_stream.into_reader();
                    reader
                        .read(&mut buffer, Some(RANGE_LEN as usize))
                        .await
                        .unwrap();
                    client = reader
                        .into_client()
                        .await
                        .unwrap()
                        .try_get_range(buffer.len() as u64, RANGE_LEN)
                        .await
                        .unwrap();
                }
                StreamingClient::All(client_with_stream) => {
                    let mut reader = client_with_stream.into_reader();
                    reader.read(buffer, None).await.unwrap();
                    break;
                }
            }
        }
    }
}
