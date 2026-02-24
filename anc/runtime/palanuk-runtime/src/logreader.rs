use cu29::prelude::*;
use cu29_logviz::*;
use cu29_export::*;
use anc_pub::{ObstacleDetected, Distance};
use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};
use zsrc_merger::{OddOpenLoopSpeed, OddLoopMode, OddOpenLoopDriveState, OddOpenLoopForcepan};

use std::{any::Any, path::PathBuf};
use clap::Parser;

gen_cumsgs!("taskdag.ron");

#[derive(Debug, Parser)]
#[command(author, version, about = "")]
struct Args {
    unifiedlog_base: PathBuf,
    #[arg(long, default_value_t = 9876)]
    grpc_port: u16,
    #[arg(long, default_value_t = 9090)]
    web_port: u16,
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Start a gRPC server as the rerun data transport
    let rec = rerun::RecordingStreamBuilder::new("palanuk-logviz")
        .serve_grpc_opts(
            args.bind.clone(),
            args.grpc_port,
        rerun::GrpcServerOptions {
            bind_ip: args.bind.clone(),
            port: args.grpc_port,
        })?;

    let connect_to = format!("rerun+http://{}:{}/proxy", args.bind, args.grpc_port);
    let _server_guard = rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        bind_ip: args.bind.clone(),
        web_port: WebViewerServerPort(args.web_port),
        connect_to: vec![connect_to],
        ..Default::default()
    })?;

    eprintln!("rerunviewer at {}:{}", args.bind, args.web_port);

    let logger = UnifiedLoggerBuilder::new()
        .file_base_name(&args.unifiedlog_base)
        .build()
        .map_err(|e| format!("failed to open: {e}"))?;
    let dl = match logger {
        UnifiedLogger::Read(dl) => dl,
        UnifiedLogger::Write(_) => return Err("expect read only log".into()),
    };
    let mut reader = UnifiedLoggerIOReader::new(dl, UnifiedLogType::CopperList);
    for culist in copperlists_reader::<CuStampedDataSet>(&mut reader) {
        logviz_emit_dataset(&culist.msgs, &rec)?;
    }

    std::thread::sleep(std::time::Duration::from_secs(u64::MAX));
    Ok(())
}
