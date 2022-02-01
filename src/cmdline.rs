use clap::Parser;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Parser, Clone)]
#[clap(version = built_info::PKG_VERSION, author = built_info::PKG_AUTHORS)]
pub struct Opts {
    /// Disable controller emulation (only enable connecting and streaming)
    #[clap(long)]
    pub no_controller: bool,

    /// Directory containing vulcast.conf and other config
    #[clap(long, default_value = concat!(env!("HOME"), "/.vulcast"))]
    pub config_dir: String,
}
