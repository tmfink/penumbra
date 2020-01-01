mod cmdline;
mod config;
mod error;

use std::{
    convert::{Infallible, TryFrom},
    net::SocketAddr,
};

use http::uri;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Uri};
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

    let make_service = make_service_fn(move |_| {
        let umbra_options = umbra_options.clone();
        let client = client.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                proxy(client.clone(), req, umbra_options.clone())
            }))
        }
    });

    let server = Server::bind(&listen_addr).serve(make_service);

    info!("Listening on http://{}", listen_addr);

    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
}
async fn proxy(
    client: HttpClient,
    mut req: Request<Body>,
    umbra_options: UmbraOptions,
) -> Result<Response<Body>, hyper::Error> {
    info!("{} to {:?}", req.method(), req.uri());
    trace!("req: {:?}", req);

    let uri = req.uri().clone();
    let mut connect_uri_parts = dbg!(uri.into_parts());
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

    client.request(req).await
}
