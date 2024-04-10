use std::sync::Arc;

use pluginx::{
    handshake::HandshakeConfig,
    plugin::PluginServer,
    server::{config::ServerConfig, Server},
    NamedService, Request, Response, Status,
};
use spire_plugin::spire::{
    plugin::agent::workloadattestor::v1::{
        workload_attestor_server::{WorkloadAttestor, WorkloadAttestorServer},
        AttestRequest, AttestResponse,
    },
    service::{
        common::config::v1::{
            config_server::{Config, ConfigServer},
            ConfigureRequest, ConfigureResponse,
        },
        private::init::v1::{
            init_server::{Init, InitServer},
            DeinitRequest, DeinitResponse, InitRequest, InitResponse,
        },
    },
};

#[derive(Default, Debug)]
struct DummyAttestor;

#[pluginx::async_trait]
impl Config for DummyAttestor {
    async fn configure(
        &self,
        _: Request<ConfigureRequest>,
    ) -> Result<Response<ConfigureResponse>, Status> {
        Ok(Response::new(ConfigureResponse {}))
    }
}

#[pluginx::async_trait]
impl WorkloadAttestor for DummyAttestor {
    async fn attest(&self, _: Request<AttestRequest>) -> Result<Response<AttestResponse>, Status> {
        Ok(Response::new(AttestResponse {
            selector_values: vec!["id:0".to_string()],
        }))
    }
}

#[pluginx::async_trait]
impl Init for DummyAttestor {
    async fn init(&self, _: Request<InitRequest>) -> Result<Response<InitResponse>, Status> {
        Ok(Response::new(InitResponse {
            plugin_service_names: [
                <ConfigServer<DummyAttestor> as NamedService>::NAME.into(),
                <WorkloadAttestorServer<DummyAttestor> as NamedService>::NAME.into(),
            ]
            .into(),
        }))
    }

    async fn deinit(&self, _: Request<DeinitRequest>) -> Result<Response<DeinitResponse>, Status> {
        Ok(Response::new(DeinitResponse {}))
    }
}

struct ConfigPlugin(Arc<DummyAttestor>);

impl PluginServer for ConfigPlugin {
    type Server = ConfigServer<DummyAttestor>;

    async fn server(&self) -> Self::Server {
        ConfigServer::from_arc(self.0.clone())
    }
}

struct WorkloadAttestorPlugin(Arc<DummyAttestor>);

impl PluginServer for WorkloadAttestorPlugin {
    type Server = WorkloadAttestorServer<DummyAttestor>;

    async fn server(&self) -> Self::Server {
        WorkloadAttestorServer::from_arc(self.0.clone())
    }
}

struct InitPlugin(Arc<DummyAttestor>);

impl PluginServer for InitPlugin {
    type Server = InitServer<DummyAttestor>;

    async fn server(&self) -> Self::Server {
        InitServer::from_arc(self.0.clone())
    }
}

async fn amain() {
    let mut server = Server::new(ServerConfig {
        handshake_config: HandshakeConfig {
            protocol_version: 1,
            magic_cookie_key: "WorkloadAttestor".into(),
            magic_cookie_value: "WorkloadAttestor".into(),
        },
    })
    .await
    .unwrap();

    let attestor = Arc::new(DummyAttestor);

    server
        .add_plugin(ConfigPlugin(attestor.clone()))
        .await
        .add_plugin(WorkloadAttestorPlugin(attestor.clone()))
        .await
        .add_plugin(InitPlugin(attestor.clone()))
        .await;

    server.run().await.unwrap();
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(amain());
}
