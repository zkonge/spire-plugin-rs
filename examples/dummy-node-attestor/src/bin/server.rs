use std::{
    future::{ready, Ready},
    sync::{Arc, Mutex},
};

use futures::{stream, StreamExt};
use pluginx::{
    handshake::HandshakeConfig, server::config::ServerConfig, NamedService, Request, Response,
    Status, Streaming,
};
use spire_plugin::spire::{
    plugin::server::nodeattestor::v1::{
        attest_request, attest_response,
        node_attestor_server::{NodeAttestor, NodeAttestorServer},
        AgentAttributes, AttestRequest, AttestResponse,
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
struct DummyNodeAttestor {
    trust_domain: Mutex<String>,
}

#[pluginx::async_trait]
impl Config for DummyNodeAttestor {
    async fn configure(
        &self,
        request: Request<ConfigureRequest>,
    ) -> Result<Response<ConfigureResponse>, Status> {
        match request
            .into_inner()
            .core_configuration
            .map(|x| x.trust_domain)
        {
            Some(trust_domain) => {
                *self.trust_domain.lock().unwrap() = trust_domain;
                Ok(Response::new(ConfigureResponse {}))
            }
            None => Err(Status::invalid_argument("trust_domain is required")),
        }
    }
}

#[pluginx::async_trait]
impl NodeAttestor for DummyNodeAttestor {
    type AttestStream = stream::Once<Ready<Result<AttestResponse, Status>>>;

    async fn attest(
        &self,
        request: Request<Streaming<AttestRequest>>,
    ) -> Result<Response<Self::AttestStream>, Status> {
        let user_id = match request.into_inner().next().await {
            Some(Ok(AttestRequest {
                request: Some(attest_request::Request::Payload(x)),
            })) => x,
            _ => {
                return Err(Status::invalid_argument(
                    "invalid request, should be user provided id in Payload type",
                ))
            }
        };
        let user_id = match String::from_utf8(user_id) {
            Ok(user_id) if user_id.is_ascii() => user_id,
            _ => {
                return Err(Status::invalid_argument(
                    "invalid user id, user id should be valid ascii string",
                ))
            }
        };

        // spiffe://<trust-domain>/spire/agent/<plugin-name>/<unique-suffix>
        // spiffe://example.org/spire/agent/dummy/zkonge
        let spiffe_id = format!(
            "spiffe://{}/spire/agent/dummy/{}",
            self.trust_domain.lock().unwrap(),
            &user_id
        );

        let response = AttestResponse {
            response: Some(attest_response::Response::AgentAttributes(
                AgentAttributes {
                    spiffe_id,
                    selector_values: vec![user_id],
                    can_reattest: true,
                },
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
        .add_plugin(ConfigServer::from_arc(attestor.clone()))
        .await
        .add_plugin(NodeAttestorServer::from_arc(attestor.clone()))
        .await
        .add_plugin(InitServer::from_arc(attestor))
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
