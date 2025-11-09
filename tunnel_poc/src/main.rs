use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UdpSocket,
    signal,
    sync::watch,
    task::JoinSet,
    time,
};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use tun::{AsyncDevice, Configuration, Layer};

#[derive(Parser, Debug)]
#[command(version, about = "Minimal secure tunnel proof-of-concept")]
struct Cli {
    /// Path to the TOML configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    logging: LoggingConfig,
    tun: TunConfig,
    tunnel: TunnelConfig,
}

#[derive(Debug, Deserialize)]
struct LoggingConfig {
    #[serde(default = "LoggingConfig::default_filter")]
    filter: String,
}

#[derive(Debug, Deserialize)]
struct TunConfig {
    name: Option<String>,
    address: Option<String>,
    netmask: Option<String>,
    destination: Option<String>,
    #[serde(default = "TunConfig::default_mtu")]
    mtu: u16,
}

#[derive(Debug, Deserialize)]
struct TunnelConfig {
    bind: String,
    peer: String,
    #[serde(default)]
    keepalive_interval_secs: Option<u64>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            filter: Self::default_filter(),
        }
    }
}

impl LoggingConfig {
    fn default_filter() -> String {
        String::from("info")
    }
}

impl TunConfig {
    const fn default_mtu() -> u16 {
        1400
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config).context("loading config file")?;
    init_tracing(&config.logging)?;

    info!("starting tunnel with config {:?}", sanitize_config(&config));

    let mut shutdown = Shutdown::new();

    let tun = create_tun(&config.tun).context("creating TUN device")?;
    let socket = bind_socket(&config.tunnel)
        .await
        .with_context(|| format!("binding UDP socket on {}", config.tunnel.bind))?;

    let mtu = config.tun.mtu as usize;
    let keepalive = config.tunnel.keepalive_interval_secs;

    let mut tasks = JoinSet::new();

    let (tun_reader, tun_writer) = tokio::io::split(tun);
    let socket = Arc::new(socket);

    tasks.spawn(reader_task(
        tun_reader,
        socket.clone(),
        mtu,
        keepalive,
        shutdown.subscribe(),
    ));
    tasks.spawn(writer_task(tun_writer, socket, mtu, shutdown.subscribe()));
    tasks.spawn(stats_task(shutdown.subscribe()));

    tokio::select! {
        result = tasks.join_next() => {
            if let Some(res) = result {
                match res {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        error!("task returned error: {err:?}");
                    }
                    Err(err) => {
                        error!("task panicked: {err:?}");
                    }
                }
            }
        }
        _ = signal::ctrl_c() => {
            info!("interrupt received, shutting down");
        }
    }

    shutdown.trigger();

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                error!("task returned error: {err:?}");
            }
            Err(err) => {
                error!("task panicked: {err:?}");
            }
        }
    }

    info!("tunnel stopped");
    Ok(())
}

fn load_config(path: &PathBuf) -> Result<Config> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut config: Config = toml::from_str(&raw).context("parsing TOML")?;

    if config.tunnel.keepalive_interval_secs == Some(0) {
        config.tunnel.keepalive_interval_secs = None;
    }

    Ok(config)
}

fn init_tracing(logging: &LoggingConfig) -> Result<()> {
    let subscriber = fmt()
        .with_env_filter(EnvFilter::builder().parse(logging.filter.clone())?)
        .with_target(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("setting global tracing subscriber")?;
    Ok(())
}

fn create_tun(config: &TunConfig) -> Result<AsyncDevice> {
    let mut cfg = Configuration::default();
    cfg.layer(Layer::L3).up();

    if let Some(name) = &config.name {
        cfg.name(name);
    }

    if let Some(address) = parse_ipv4_opt(config.address.as_deref())? {
        cfg.address(address);
    }

    if let Some(destination) = parse_ipv4_opt(config.destination.as_deref())? {
        cfg.destination(destination);
    }

    if let Some(netmask) = parse_ipv4_opt(config.netmask.as_deref())? {
        cfg.netmask(netmask);
    }

    cfg.mtu(i32::from(config.mtu));

    let device =
        tun::create(&cfg).context("creating TUN device (requires CAP_NET_ADMIN / root)")?;
    AsyncDevice::new(device).context("creating async wrapper over TUN device")
}

async fn bind_socket(config: &TunnelConfig) -> Result<UdpSocket> {
    let bind_addr: SocketAddr = config
        .bind
        .parse()
        .with_context(|| format!("invalid bind address {}", config.bind))?;
    let peer_addr: SocketAddr = config
        .peer
        .parse()
        .with_context(|| format!("invalid peer address {}", config.peer))?;

    let socket = UdpSocket::bind(bind_addr).await?;
    socket.connect(peer_addr).await?;
    Ok(socket)
}

async fn reader_task(
    mut tun_reader: tokio::io::ReadHalf<AsyncDevice>,
    socket: Arc<UdpSocket>,
    mtu: usize,
    keepalive: Option<u64>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let mut buf = vec![0u8; mtu];
    let keepalive_dur = keepalive.map(Duration::from_secs);
    let mut keepalive_interval = keepalive_dur.map(time::interval);

    loop {
        tokio::select! {
            biased;
            res = shutdown.changed() => {
                if res.is_err() {
                    info!("reader_task shutdown channel closed");
                } else {
                    info!("reader_task received shutdown");
                }
                break;
            }
            res = tun_reader.read(&mut buf) => {
                let n = match res {
                    Ok(0) => {
                        warn!("TUN interface closed");
                        break;
                    }
                    Ok(n) => n,
                    Err(err) => {
                        error!("error reading from TUN: {err:?}");
                        return Err(err.into());
                    }
                };
                if let Err(err) = socket.send(&buf[..n]).await {
                    error!("error sending to UDP socket: {err:?}");
                    return Err(err.into());
                }
            }
            _ = async {
                if let Some(interval) = keepalive_interval.as_mut() {
                    interval.tick().await;
                }
            }, if keepalive_dur.is_some() => {
                if let Err(err) = socket.send(&[]).await {
                    warn!("failed to send keepalive: {err:?}");
                }
            }
        }
    }

    Ok(())
}

async fn writer_task(
    mut tun_writer: tokio::io::WriteHalf<AsyncDevice>,
    socket: Arc<UdpSocket>,
    mtu: usize,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let mut buf = vec![0u8; mtu.max(65535)];

    loop {
        tokio::select! {
            biased;
            res = shutdown.changed() => {
                if res.is_err() {
                    info!("writer_task shutdown channel closed");
                } else {
                    info!("writer_task received shutdown");
                }
                break;
            }
            res = socket.recv(&mut buf) => {
                let n = match res {
                    Ok(0) => {
                        warn!("received empty packet");
                        continue;
                    }
                    Ok(n) => n,
                    Err(err) => {
                        error!("error receiving from UDP socket: {err:?}");
                        return Err(err.into());
                    }
                };

                if let Err(err) = tun_writer.write_all(&buf[..n]).await {
                    error!("error writing to TUN: {err:?}");
                    return Err(err.into());
                }
            }
        }
    }

    Ok(())
}

async fn stats_task(mut shutdown: watch::Receiver<bool>) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                info!("stats_task shutting down");
                break;
            }
            _ = interval.tick() => {
                info!("stats_task heartbeat");
            }
        }
    }

    Ok(())
}

fn parse_ipv4_opt(value: Option<&str>) -> Result<Option<Ipv4Addr>> {
    value
        .map(|v| {
            v.parse()
                .with_context(|| format!("invalid IPv4 address {v}"))
        })
        .transpose()
}

fn sanitize_config(config: &Config) -> Config {
    Config {
        logging: LoggingConfig {
            filter: config.logging.filter.clone(),
        },
        tun: TunConfig {
            name: config.tun.name.clone(),
            address: config.tun.address.clone(),
            netmask: config.tun.netmask.clone(),
            destination: config.tun.destination.clone(),
            mtu: config.tun.mtu,
        },
        tunnel: TunnelConfig {
            bind: config.tunnel.bind.clone(),
            peer: config.tunnel.peer.clone(),
            keepalive_interval_secs: config.tunnel.keepalive_interval_secs,
        },
    }
}

struct Shutdown {
    tx: watch::Sender<bool>,
}

impl Shutdown {
    fn new() -> Self {
        let (tx, _) = watch::channel(false);
        Self { tx }
    }

    fn subscribe(&self) -> watch::Receiver<bool> {
        self.tx.subscribe()
    }

    fn trigger(&mut self) {
        if let Err(err) = self.tx.send(true) {
            warn!("failed to notify shutdown: {err:?}");
        }
    }
}
