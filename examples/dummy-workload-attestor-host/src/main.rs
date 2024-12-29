use std::{env::args, error::Error};

use futures_util::stream::StreamExt;
use pluginx::{
    client::{Channel, ClientBuilder, StdioData, config::ClientConfig},
    handshake::HandshakeConfig,
    plugin::PluginClient,
};
use spire_plugin::spire::{
    plugin::agent::workloadattestor::v1::{
        AttestRequest, workload_attestor_client::WorkloadAttestorClient,
    },
    service::{
        common::config::v1::{ConfigureRequest, CoreConfiguration, config_client::ConfigClient},
        private::init::v1::{DeinitRequest, InitRequest, init_client::InitClient},
    },
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    spawn,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Init;

impl PluginClient for Init {
    type Client = InitClient<Channel>;

    async fn client(&self, channel: Channel) -> Self::Client {
        Self::Client::new(channel)
    }
}

struct Config;

impl PluginClient for Config {
    type Client = ConfigClient<Channel>;

    async fn client(&self, channel: Channel) -> Self::Client {
        Self::Client::new(channel)
    }
}

struct WorkloadAttestor;

impl PluginClient for WorkloadAttestor {
    type Client = WorkloadAttestorClient<Channel>;

    async fn client(&self, channel: Channel) -> Self::Client {
        Self::Client::new(channel)
    }
}

async fn amain() -> Result<()> {
    // usage: cargo run --bin dummy-workload-attestor-host -- <plugin path> <trust domain> <pid> [config]
    let mut args = args().skip(1);

    let command = args.next().expect("specify plugin binary path");
    let trust_domain = args.next().expect("specify trust domain");
    let pid: i32 = args.next().expect("specify pid").parse().expect("bad pid");
    let config = args.next().unwrap_or_default();

    let mut client = ClientBuilder::new(ClientConfig {
        handshake_config: HandshakeConfig {
            protocol_version: 1,
            magic_cookie_key: "WorkloadAttestor".into(),
            magic_cookie_value: "WorkloadAttestor".into(),
        },
        cmd: Command::new(command),
        broker_multiplex: false,
        port_range: None,
    })
    .await?;

    client
        .add_plugin(Config)
        .await
        .add_plugin(Init)
        .await
        .add_plugin(WorkloadAttestor)
        .await;

    let mut client = client.build();

    // stdout
    let stderr_handler = client.raw_stdout().unwrap();
    spawn(async move {
        let mut buf = BufReader::new(stderr_handler);
        let mut line = String::new();

        while buf.read_line(&mut line).await.unwrap() > 0 {
            println!("[raw_stderr] {}", line);
            line.clear();
        }
    });

    // stderr
    let stderr_handler = client.raw_stderr().unwrap();
    spawn(async move {
        let mut buf = BufReader::new(stderr_handler);
        let mut line = String::new();

        while buf.read_line(&mut line).await.unwrap() > 0 {
            println!("[raw_stderr] {}", line);
            line.clear();
        }
    });

    // logger
    let stdio_handler = client.stdio().unwrap();
    let logger = spawn(async move {
        let Ok(mut stdio) = stdio_handler.read().await else {
            println!("failed to read from stdio");
            return;
        };

        while let Some(data) = stdio.next().await {
            match data {
                StdioData::Invalid => println!("[invalid data]"),
                StdioData::Stdout(vec) => {
                    let stdout = String::from_utf8(vec).unwrap();
                    println!("[stdout] {}", stdout);
                }
                StdioData::Stderr(vec) => {
                    let stderr = String::from_utf8(vec).unwrap();
                    println!("[stderr] {}", stderr);
                }
            }
        }
    });

    // init plugin
    let mut init_client = client.dispense::<Init>().unwrap();
    init_client
        .init(InitRequest {
            host_service_names: Vec::new(),
        })
        .await?;

    // configure plugin
    let mut config_client = client.dispense::<Config>().unwrap();
    config_client
        .configure(ConfigureRequest {
            core_configuration: Some(CoreConfiguration { trust_domain }),
            hcl_configuration: config,
        })
        .await?;

    // attest
    let mut workload_attestor_client = client.dispense::<WorkloadAttestor>().unwrap();
    let selectors = workload_attestor_client
        .attest(AttestRequest { pid })
        .await?
        .into_inner()
        .selector_values;

    dbg!(selectors);

    // gracefully shutdown
    init_client.deinit(DeinitRequest {}).await?;

    // abort logger task, or it will hang the hyper gracefully shutdown
    logger.abort();

    // wait for plugin to exit
    client.shutdown().await;

    Ok(())
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(amain())
        .unwrap();
}
