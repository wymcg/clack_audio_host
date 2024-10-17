# Clack Audio Host
This is a very basic CLAP host which supplies plugins with a note event and pipes audio output to a JACK server.

## Build/Run
After installing Rust (see [here](https://rustup.rs)):
```bash
git clone https://github.com/wymcg/clack_audio_host.git
cd clack_audio_host
cargo run -- <PATH_TO_PLUGIN>
```
