use clap::{Args, Subcommand};

use super::generate;

#[derive(Args)]
pub struct VideoArgs {
  #[command(subcommand)]
  command: VideoCommands,
}

#[derive(Subcommand)]
enum VideoCommands {
  /// Generate a video from a text prompt (and optional first-frame image)
  Generate {
    prompt: String,
    #[arg(short, long)]
    model: Option<String>,
    #[arg(short, long)]
    output: String,
    #[arg(long)]
    image: Option<String>,
    #[arg(long)]
    aspect_ratio: Option<String>,
    #[arg(long)]
    duration: Option<u32>,
    #[arg(long)]
    resolution: Option<String>,
    #[arg(long)]
    seed: Option<i64>,
    #[arg(long, default_value_t = 10)]
    poll_interval: u64,
    #[arg(long, default_value_t = 360)]
    max_wait: u64,
  },
}

pub async fn dispatch(args: VideoArgs) -> anyhow::Result<()> {
  match args.command {
    VideoCommands::Generate {
      prompt,
      model,
      output,
      image,
      aspect_ratio,
      duration,
      resolution,
      seed,
      poll_interval,
      max_wait,
    } => {
      generate::execute(
        prompt,
        model,
        output,
        image,
        aspect_ratio,
        duration,
        resolution,
        seed,
        poll_interval,
        max_wait,
      )
      .await
    }
  }
}
