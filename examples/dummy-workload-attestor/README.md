# Dummy Workload

## Build
```bash
RUSTFLAGS="-Zlocation-detail=none" cargo +nightly build -Z build-std=std,panic_abort --target x86_64-unknown-linux-gnu --release
```

## Config example
```hcl
WorkloadAttestor "dummy" {
    plugin_cmd = "/<some-path>/spire-dummy-workload-attestor"
}
```
