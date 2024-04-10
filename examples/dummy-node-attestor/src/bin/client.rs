use std::{
    future::{ready, Ready},
    sync::{Arc, Mutex},
};

use futures::stream;
use pluginx::{
    handshake::HandshakeConfig, plugin::PluginServer, server::config::ServerConfig, NamedService,
    Request, Response, Status, Streaming,
};
use serde::Deserialize;
use spire_plugin::spire::{
    plugin::agent::nodeattestor::v1::{
        node_attestor_server::{NodeAttestor, NodeAttestorServer},
        payload_or_challenge_response, Challenge, PayloadOrChallengeResponse,
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

#[derive(Deserialize, Debug)]
struct DummyNodeAttestorConfig {
    id: String,
}

#[derive(Default, Debug)]
struct DummyNodeAttestor {
    id: Mutex<String>,
}

#[pluginx::async_trait]
impl Config for DummyNodeAttestor {
    async fn configure(
        &self,
        request: Request<ConfigureRequest>,
    ) -> Result<Response<ConfigureResponse>, Status> {
        let id = hcl::from_str::<DummyNodeAttestorConfig>(&request.into_inner().hcl_configuration)
            .map(|x| x.id)
            .map_err(|e| {
                Status::invalid_argument(format!("failed to parse HCL configuration: {e}"))
            })?;

        *self.id.lock().unwrap() = id;

        Ok(Response::new(ConfigureResponse {}))
    }
}

#[pluginx::async_trait]
impl NodeAttestor for DummyNodeAttestor {
    type AidAttestationStream = stream::Once<Ready<Result<PayloadOrChallengeResponse, Status>>>;

    async fn aid_attestation(
        &self,
        _: Request<Streaming<Challenge>>,
    ) -> Result<Response<Self::AidAttestationStream>, Status> {
        let id = self.id.lock().unwrap().clone();

        let response = PayloadOrChallengeResponse {
            data: Some(payload_or_challenge_response::Data::Payload(
                id.clone().into_bytes(),
            )),
        };

        Ok(Response::new(stream::once(ready(Ok(response)))))
    }
}

#[pluginx::async_trait]
impl Init for DummyNodeAttestor {
    async fn init(&self, _: Request<InitRequest>) -> Result<Response<InitResponse>, Status> {
        Ok(Response::new(InitResponse {
            plugin_service_names: [
                <ConfigServer<DummyNodeAttestor> as NamedService>::NAME.into(),
                <NodeAttestorServer<DummyNodeAttestor> as NamedService>::NAME.into(),
            ]
            .into(),
        }))
    }

    async fn deinit(&self, _: Request<DeinitRequest>) -> Result<Response<DeinitResponse>, Status> {
        Ok(Response::new(DeinitResponse {}))
    }
}

struct ConfigPlugin(Arc<DummyNodeAttestor>);

impl PluginServer for ConfigPlugin {
    type Server = ConfigServer<DummyNodeAttestor>;

    async fn server(&self) -> Self::Server {
        ConfigServer::from_arc(self.0.clone())
    }
}

struct NodeAttestorPlugin(Arc<DummyNodeAttestor>);

impl PluginServer for NodeAttestorPlugin {
    type Server = NodeAttestorServer<DummyNodeAttestor>;

    async fn server(&self) -> Self::Server {
        NodeAttestorServer::from_arc(self.0.clone())
    }
}

struct InitPlugin(Arc<DummyNodeAttestor>);

impl PluginServer for InitPlugin {
    type Server = InitServer<DummyNodeAttestor>;

    async fn server(&self) -> Self::Server {
        InitServer::from_arc(self.0.clone())
    }
}

async fn amain() {
    let mut server = pluginx::server::Server::new(ServerConfig {
        handshake_config: HandshakeConfig {
            protocol_version: 1,
            magic_cookie_key: "NodeAttestor".into(),
            magic_cookie_value: "NodeAttestor".into(),
        },
    })
    .await
    .unwrap();

    let attestor = Arc::new(DummyNodeAttestor::default());

    server
        .add_plugin(ConfigPlugin(attestor.clone()))
        .await
        .add_plugin(NodeAttestorPlugin(attestor.clone()))
        .await
        .add_plugin(InitPlugin(attestor))
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
