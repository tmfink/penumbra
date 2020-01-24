mod cmdline;
mod config;
mod error;

use std::{
    convert::{Infallible, TryFrom},
    net::SocketAddr,
};

use http::{uri, HeaderValue};
use hyper::{
    header,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Client, Request, Response, Server, Uri,
};
use log::*;

use crate::config::*;
use crate::error::*;

type HttpClient = Client<hyper::client::HttpConnector>;

#[tokio::main]
async fn main() {
    let umbra_options = cmdline::parse_args().unwrap_log();
    debug!("Options: {:?}", umbra_options);

    let listen_addr: SocketAddr = (
        umbra_options.listen_ip,
        umbra_options.http_ports.unwrap().listen,
    )
        .into();
    let client = HttpClient::new();

    let make_service = make_service_fn(move |addr_stream: &AddrStream| {
        let umbra_options = umbra_options.clone();
        let client = client.clone();
        let remote_addr = addr_stream.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                proxy(remote_addr, client.clone(), req, umbra_options.clone())
            }))
        }
    });

    let server = Server::bind(&listen_addr).serve(make_service);

    info!("Listening on http://{}", listen_addr);

    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
}

/// Context with each connection
#[derive(Debug, Clone)]
struct ConnectionCtx {
    remote_addr: SocketAddr,
}

/// Modify the incoming request before forwarding to the internal server
fn tinker_http_request(connection: &mut ConnectionCtx, request: &mut Request<Body>) -> Result<()> {
    let headers = request.headers_mut();
    let remote_ip_str = format!("{}", connection.remote_addr.ip());
    headers.insert(
        header::FORWARDED,
        HeaderValue::from_str(&remote_ip_str).unwrap(),
    );

    Ok(())
}

/// Modify the response before forwarding to the external client
fn tinker_http_response(
    _connection: &mut ConnectionCtx,
    response: &mut Response<Body>,
) -> Result<()> {
    let headers = response.headers_mut();
    headers.insert("X-proxy-shim", HeaderValue::from_static("penumbra"));

    Ok(())
}

/// Proxy between remote client and local server
async fn proxy(
    remote_addr: SocketAddr,
    client: HttpClient,
    mut req: Request<Body>,
    umbra_options: UmbraOptions,
) -> Result<Response<Body>> {
    info!("{} to {:?}", req.method(), req.uri());
    trace!("req: {:?}", req);

    let uri = req.uri().clone();
    let mut connect_uri_parts = uri.into_parts();
    let connect_authority_str = format!(
        "{}:{}",
        umbra_options.connect_ip,
        umbra_options
            .http_ports
            .expect("not http_ports field")
            .connect
    );
    let connect_authority = uri::Authority::try_from(connect_authority_str.as_str())
        .expect("unable to create authority");
    connect_uri_parts.scheme = Some(uri::Scheme::HTTP);
    connect_uri_parts.authority = Some(connect_authority);
    let connect_uri =
        Uri::from_parts(connect_uri_parts).expect("could not create connect URI from parts");
    *req.uri_mut() = connect_uri;

    let mut connection = ConnectionCtx { remote_addr };
    tinker_http_request(&mut connection, &mut req)?;

    let mut response = client.request(req).await?;
    tinker_http_response(&mut connection, &mut response)?;

    Ok(response)
}
