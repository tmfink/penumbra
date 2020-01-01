use std::{
    cmp::{max, min},
    fs::File,
    io::Read,
    net::IpAddr,
    process::exit,
};

use chrono;
use clap::{self, crate_version, App, Arg};
use fern;
use log::*;

use crate::error::*;
use crate::*;

const LISTEN_IP_ARG: &str = "listen-ip";
const CONNECT_IP_ARG: &str = "connect-ip";
const HTTP_LISTEN_PORT_ARG: &str = "http-listen-port";
const HTTPS_LISTEN_PORT_ARG: &str = "https-listen-port";
const HTTP_CONNECT_PORT_ARG: &str = "http-connect-port";
const HTTPS_CONNECT_PORT_ARG: &str = "https-connect-port";
const TLS_CERT_ARG: &str = "tls-cert";
const TLS_KEY_ARG: &str = "tls-key";
const CONFIG_ARG: &str = "config";
const VERBOSITY_ARG: &str = "v";
const QUIET_ARG: &str = "q";

impl ProtoPorts {
    fn new(
        matches: &clap::ArgMatches,
        listen_arg: &str,
        connect_arg: &str,
    ) -> Result<Option<ProtoPorts>, String> {
        let listen = matches.value_of(listen_arg);
        let connect = matches.value_of(connect_arg);

        let (listen_str, connect_str) = match (listen, connect) {
            (None, None) => return Ok(None),
            (Some(listen), Some(connect)) => (listen, connect),
            _ => return Err(format!("Must pass both --{}/--{}", listen_arg, connect_arg)),
        };

        let err = |err: std::num::ParseIntError| format!("{}", err);
        let listen_port: u16 = listen_str.parse().map_err(err)?;
        let connect_port: u16 = connect_str.parse().map_err(err)?;
        Ok(Some(ProtoPorts {
            listen: listen_port,
            connect: connect_port,
        }))
    }
}

const LOG_LEVELS: &[LevelFilter] = &[
    LevelFilter::Off,
    LevelFilter::Error,
    LevelFilter::Warn,
    LevelFilter::Info,
    LevelFilter::Debug,
    LevelFilter::Trace,
];

/// Initialize logger
fn setup_logger(level: LevelFilter) -> Result<(), fern::InitError> {
    //chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                record.level(),
                chrono::Local::now().to_rfc3339(),
                record.target(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn log_level(verbose_count: usize, quiet_count: usize) -> LevelFilter {
    let log_index: i32 = 2 + verbose_count as i32 - quiet_count as i32;
    LOG_LEVELS[max(0, min(log_index, (LOG_LEVELS.len() - 1) as i32)) as usize]
}

fn cmd_set_specified_all_or_nothing(
    matches: &clap::ArgMatches,
    name: &str,
    options: &[&str],
) -> Result<(), String> {
    let missing: Vec<&str> = options.iter().cloned().filter(|opt| matches.is_present(opt)).clone().collect();
    if missing.len() == options.len() || missing.len() == 0 {
        Ok(())
    } else {
        eprintln!("For {} set, missing arguments: {}", name, cmd_list_str(&missing));
        exit(1);
    }

}

fn cmd_list_str(args: &[&str]) -> String {
    args.iter()
        .map(|x| format!("--{}", x))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse commandline arguments
pub(crate) fn parse_args() -> Result<UmbraOptions, String> {
    let http_options = [HTTP_LISTEN_PORT_ARG, HTTP_CONNECT_PORT_ARG];
    let https_options = [
        HTTPS_LISTEN_PORT_ARG,
        HTTPS_CONNECT_PORT_ARG,
        TLS_CERT_ARG,
        TLS_KEY_ARG,
    ];

    let option_sets_warning: String = format!(
        "Must specify at least the HTTP or HTTPS set of options:\n    HTTP: {}\n    HTTPS: {}",
        cmd_list_str(&http_options),
        cmd_list_str(&https_options)
    );

    let matches = App::new("Umbra Firewall")
        .version(crate_version!())
        .author("Travis Finkenauer <tmfinken@gmail.com>")
        .about("Acts as an HTTP firewall between a webserver and the outside world")
        .arg(
            Arg::with_name(LISTEN_IP_ARG)
                .long(LISTEN_IP_ARG)
                .default_value("0.0.0.0")
                .value_name("IP")
                .help("IP address to listen to default")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(CONNECT_IP_ARG)
                .long(CONNECT_IP_ARG)
                .default_value("127.0.0.1")
                .value_name("IP")
                .help("IP address to connect (which hosts actual webserver)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTP_LISTEN_PORT_ARG)
                .long(HTTP_LISTEN_PORT_ARG)
                .value_name("PORT")
                .help("HTTP port on which shim should listen")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTPS_LISTEN_PORT_ARG)
                .long(HTTPS_LISTEN_PORT_ARG)
                .value_name("PORT")
                .help("HTTPS port on which shim should listen")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTP_CONNECT_PORT_ARG)
                .long(HTTP_CONNECT_PORT_ARG)
                .value_name("PORT")
                .help("HTTP port on which shim should connect (where main server is listening)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTPS_CONNECT_PORT_ARG)
                .long(HTTPS_CONNECT_PORT_ARG)
                .value_name("PORT")
                .help("HTTPS port on which shim should connect (where main server is listening)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(TLS_CERT_ARG)
                .long(TLS_CERT_ARG)
                .value_name("PEM_FILE")
                .help("PEM file with TLS certificate chain")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(TLS_KEY_ARG)
                .long(TLS_KEY_ARG)
                .value_name("PEM_FILE")
                .help("PEM file with server private key")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(CONFIG_ARG)
                .short("c")
                .long(CONFIG_ARG)
                .value_name("FILE")
                .help("configuration file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(VERBOSITY_ARG)
                .short(VERBOSITY_ARG)
                .multiple(true)
                .help("Increases logging verbosity"),
        )
        .arg(
            Arg::with_name(QUIET_ARG)
                .short(QUIET_ARG)
                .multiple(true)
                .help("Decreases logging verbosity"),
        )
        .after_help(option_sets_warning.as_str())
        .get_matches();


    if !(matches.is_present(HTTP_LISTEN_PORT_ARG) || matches.is_present(HTTPS_LISTEN_PORT_ARG)) {
        eprintln!("{}", option_sets_warning);
        exit(1);
    }
    cmd_set_specified_all_or_nothing(&matches, "HTTP", &http_options)?;
    cmd_set_specified_all_or_nothing(&matches, "HTTPS", &https_options)?;

    // Logging
    let verbose_count = matches.occurrences_of(VERBOSITY_ARG) as usize;
    let quiet_count = matches.occurrences_of(QUIET_ARG) as usize;
    let verbosity_level = log_level(verbose_count, quiet_count);
    println!("level = {}", verbosity_level);
    setup_logger(verbosity_level).map_string_error()?;
    info!("level = {}", verbosity_level);

    // Ports
    let http_ports = ProtoPorts::new(&matches, HTTP_LISTEN_PORT_ARG, HTTP_CONNECT_PORT_ARG)?;
    let https_ports = ProtoPorts::new(&matches, HTTPS_LISTEN_PORT_ARG, HTTPS_CONNECT_PORT_ARG)?;

    // Config
    let config_filename = expect_log!(matches.value_of_os(CONFIG_ARG), "Must pass {}", CONFIG_ARG);
    let mut config_file = File::open(config_filename).map_err(|err| {
        format!(
            "Unable to open config file \"{}\": {}",
            config_filename.to_string_lossy(),
            err
        )
    })?;
    let mut config: Vec<u8> = Vec::new();
    config_file
        .read(&mut config)
        .map_string_error()
        .map_err(|err| format!("Failed to read config file: {}", err))?;

    let https: Option<HTTPSConfig> = if let Some(https_ports) = https_ports {
        let tls_cert_filename = expect_log!(
            matches.value_of_os(TLS_CERT_ARG),
            "Must specify {} when configuring HTTPS",
            TLS_CERT_ARG
        );
        let mut tls_cert_file = File::open(tls_cert_filename).map_err(|err| {
            format!(
                "Unable to open TLS cert file \"{}\": {}",
                tls_cert_filename.to_string_lossy(),
                err
            )
        })?;
        let mut tls_cert: Vec<u8> = Vec::new();
        tls_cert_file
            .read(&mut tls_cert)
            .map_string_error()
            .map_err(|err| format!("Failed to read TLS cert file: {}", err))?;

        let tls_key_filename = expect_log!(
            matches.value_of_os(TLS_KEY_ARG),
            "Must specify {} when configuring HTTPS",
            TLS_KEY_ARG
        );
        let mut tls_key_file = File::open(tls_key_filename).map_err(|err| {
            format!(
                "Unable to open TLS key file \"{}\": {}",
                tls_key_filename.to_string_lossy(),
                err
            )
        })?;
        let mut tls_key: Vec<u8> = Vec::new();
        tls_key_file
            .read(&mut tls_key)
            .map_string_error()
            .map_err(|err| format!("Failed to read TLS key file: {}", err))?;

        Some(HTTPSConfig {
            ports: https_ports,
            _tls_cert: tls_cert,
            _tls_key: tls_key,
        })
    } else {
        None
    };

    let listen_ip_str = matches.value_of(LISTEN_IP_ARG).unwrap();
    let listen_ip: IpAddr = listen_ip_str.parse().map_err(|err| format!("Failed to parse listen IP \"{}\": {}", listen_ip_str, err))?;
    let connect_ip_str = matches.value_of(CONNECT_IP_ARG).unwrap();
    let connect_ip: IpAddr = connect_ip_str.parse().map_err(|err| format!("Failed to parse connect IP \"{}\": {}", connect_ip_str, err))?;

    Ok(UmbraOptions {
        http_ports,
        https,
        listen_ip,
        connect_ip,
        _config: config,
    })
}
