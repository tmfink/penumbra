mod cmdline;
mod config;
mod error;

use std::{
    net::SocketAddr,
    process::exit,
    thread::{JoinHandle, spawn},
};

use hyper::{Body, Request, Response, Server};
use hyper::service::service_fn_ok;
use hyper::rt::{self, Future};
use log::*;

use crate::config::*;
use crate::error::*;


fn serve_http(umbra_options: &UmbraOptions) -> Option<JoinHandle<()>> {
    let http_ports = if let Some(http_ports) = umbra_options.http_ports {
        http_ports
    } else {
        return None;
    };

    let listen_addr: SocketAddr = (umbra_options.listen_ip, http_ports.listen).into();

    let server = Server::bind(&listen_addr)
        .serve(|| {
            // This is the `Service` that will handle the connection.
            // `service_fn_ok` is a helper to convert a function that
            // returns a Response into a `Service`.
            service_fn_ok(move |_: Request<Body>| {
                Response::new(Body::from("Hello World!"))
            })
        })
        .map_err(|e| error!("server error: {}", e));

    info!("Listening on http://{}", listen_addr);

    let thread = spawn(|| rt::run(server));
    Some(thread)
}

fn main() {
    let umbra_options = cmdline::parse_args().unwrap_log();
    debug!("Options: {:?}", umbra_options);

    let thread_http = serve_http(&umbra_options);

    let mut threads = vec![thread_http];
    for thread in threads.drain(..).filter_map(|x| x) {
        if thread.join().is_err() {
            error!("Serve thread failed");
            exit(1);
        }
    }
}
