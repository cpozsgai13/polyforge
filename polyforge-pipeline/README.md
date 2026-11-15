# polyforge-pipeline

Market-data pipeline (independent of the server). Two modes.

## Build
```sh
cargo build -p polyforge-pipeline
```

## Run
Network mode (parser → TCP sender → TCP receiver → ring buffer → processor):
```sh
cargo run -p polyforge-pipeline -- polyforge-pipeline/config.ini
```
Paths in the config are relative to the current directory.

File mode:
```sh
cargo run -p polyforge-pipeline -- polyforge-pipeline/test/Symbols.txt polyforge-pipeline/test/TestData.txt
```
