fn main() {
    tonic_build::configure()
        .include_file("spire_plugin.rs")
        // .bytes(["."])
        .emit_rerun_if_changed(true)
        .compile_protos(
            &[
                // common protos
                "proto/spire/service/common/config/v1/config.proto",
                "proto/spire/service/private/init/v1/init.proto",
                // agent protos
                #[cfg(feature = "agent-keymanager")]
                "proto/spire/plugin/agent/keymanager/v1/keymanager.proto",
                #[cfg(feature = "agent-nodeattestor")]
                "proto/spire/plugin/agent/nodeattestor/v1/nodeattestor.proto",
                #[cfg(feature = "agent-svidstore")]
                "proto/spire/plugin/agent/svidstore/v1/svidstore.proto",
                #[cfg(feature = "agent-workloadattestor")]
                "proto/spire/plugin/agent/workloadattestor/v1/workloadattestor.proto",
                // server protos
                #[cfg(feature = "server-bundlepublisher")]
                "proto/spire/plugin/server/bundlepublisher/v1/bundlepublisher.proto",
                #[cfg(feature = "server-credentialcomposer")]
                "proto/spire/plugin/server/credentialcomposer/v1/credentialcomposer.proto",
                #[cfg(feature = "server-keymanager")]
                "proto/spire/plugin/server/keymanager/v1/keymanager.proto",
                #[cfg(feature = "server-nodeattestor")]
                "proto/spire/plugin/server/nodeattestor/v1/nodeattestor.proto",
                #[cfg(feature = "server-notifier")]
                "proto/spire/plugin/server/notifier/v1/notifier.proto",
                #[cfg(feature = "server-upstreamauthority")]
                "proto/spire/plugin/server/upstreamauthority/v1/upstreamauthority.proto",
            ],
            &["proto"],
        )
        .unwrap();
}
