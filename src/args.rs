use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version)]
pub(crate) struct ClackAudioHostArgs {
    pub path: String,

    #[arg(short, long)]
    pub verbose: bool,
}
