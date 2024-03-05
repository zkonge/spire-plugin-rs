# Dummy Node

## Build
```bash
RUSTFLAGS="-Zlocation-detail=none" cargo +nightly build -Z build-std=std,panic_abort --target x86_64-unknown-linux-gnu --release
```

## Config example

### spire-server
```hcl
NodeAttestor "dummy" {
    plugin_cmd = "/<path-to>/spire-dummy-node-attestor/server"
}
```

### spire-agent
```hcl
NodeAttestor "dummy" {
    plugin_cmd = "/<path-to>/spire-dummy-node-attestor/client"
    plugin_data {
        id = "zkonge"
    }
}
```

server side plugin will generates SPIFFE ID:
```
spiffe://example.org/spire/agent/dummy/zkonge
```
