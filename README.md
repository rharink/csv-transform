# CSV-transform

CSV-transform transforms CSV files according to a configuration.toml containing column functions (lua).

## Headers
Headers must be unique, headers are converted to snake case

## Features
- [x] Columns can be transformed using lua functions 
- [X]  Columns can be created
- [X] Change delimiter
- [X] Handle escaping
- [X] Filter rows using lua functions
- [ ] Headers can be transformed using lua functions
- [ ] Exclude columns

## Run the example
```bash
cargo run -- -c fixtures/example-config.toml < fixtures/test.csv
```
