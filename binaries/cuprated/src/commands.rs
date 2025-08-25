//! Commands
//!
//! `cuprated` [`Command`] definition and handling.
use std::{io, thread::sleep, time::Duration};

use clap::{builder::TypedValueParser, Parser, ValueEnum};
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use crate::p2p::NetworkInterfaces;
use crate::{
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    logging::{self, CupratedTracingFilter},
    statics,
};
use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
};
use cuprate_helper::time::secs_to_hms;
use cuprate_p2p_core::services::{AddressBookRequest, AddressBookResponse};

/// A command received from [`io::stdin`].
#[derive(Debug, Parser)]
#[command(
    multicall = true,
    subcommand_required = true,
    rename_all = "snake_case",
    help_template = "{all-args}",
    arg_required_else_help = true,
    disable_help_flag = true
)]
pub enum Command {
    /// Change the log output.
    #[command(arg_required_else_help = true)]
    SetLog {
        /// The minimum log level that will be displayed.
        #[arg(
          short, long,
          value_parser = clap::builder::PossibleValuesParser::new(["off", "trace", "debug", "info", "warn", "error"])
            .map(|s| s.parse::<LevelFilter>().unwrap()),
        )]
        level: Option<LevelFilter>,
        /// The logging output target to change.
        #[arg(value_enum, default_value_t)]
        output_target: OutputTarget,
    },

    /// Print status information on `cuprated`.
    Status,

    /// Print the height of first block not contained in the fast sync hashes.
    FastSyncStopHeight,

    AddressBook {
        #[arg(value_enum, default_value_t)]
        zone: NetZone,
    },
}

/// The log output target.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputTarget {
    /// The stdout logging output.
    #[default]
    Stdout,
    /// The file appender logging output.
    File,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum NetZone {
    #[default]
    Clearnet,
    Tor
}

/// The [`Command`] listener loop.
pub fn command_listener(incoming_commands: mpsc::Sender<Command>) -> ! {
    let mut stdin = io::stdin();
    let mut line = String::new();

    loop {
        line.clear();

        if let Err(e) = stdin.read_line(&mut line) {
            eprintln!("Failed to read from stdin: {e}");
            sleep(Duration::from_secs(1));
            continue;
        }

        match Command::try_parse_from(line.split_whitespace()) {
            Ok(command) => drop(
                incoming_commands
                    .blocking_send(command)
                    .inspect_err(|err| eprintln!("Failed to send command: {err}")),
            ),
            Err(err) => err.print().unwrap(),
        }
    }
}

/// The [`Command`] handler loop.
pub async fn io_loop(
    mut incoming_commands: mpsc::Receiver<Command>,
    mut context_service: BlockchainContextService,
    mut network_interfaces: NetworkInterfaces,
) {
    loop {
        let Some(command) = incoming_commands.recv().await else {
            tracing::warn!("Shutting down io_loop command channel closed.");
            return;
        };

        match command {
            Command::SetLog {
                level,
                output_target,
            } => {
                let modify_output = |filter: &mut CupratedTracingFilter| {
                    if let Some(level) = level {
                        filter.level = level;
                    }
                    println!("NEW LOG FILTER: {filter}");
                };

                match output_target {
                    OutputTarget::File => logging::modify_file_output(modify_output),
                    OutputTarget::Stdout => logging::modify_stdout_output(modify_output),
                }
            }
            Command::Status => {
                let context = context_service.blockchain_context();

                let uptime = statics::START_INSTANT.elapsed().unwrap_or_default();

                let (h, m, s) = secs_to_hms(uptime.as_secs());
                let height = context.chain_height;
                let top_hash = hex::encode(context.top_hash);

                println!("STATUS:\n  uptime: {h}h {m}m {s}s,\n  height: {height},\n  top_hash: {top_hash}");
            }
            Command::FastSyncStopHeight => {
                let stop_height = cuprate_fast_sync::fast_sync_stop_height();

                println!("{stop_height}");
            }
            Command::AddressBook {
                zone: NetZone::Clearnet,
            } => {
                let mut book = network_interfaces.clearnet_network_interface.address_book();

                let AddressBookResponse::Peerlist(peer_list) = book
                    .ready()
                    .await
                    .unwrap()
                    .call(AddressBookRequest::Peerlist)
                    .await
                    .unwrap()
                else {
                    unreachable!()
                };

                println!("{peer_list:#?}");
                println!(
                    "len: anchor: {}, white: {}, grey: {}",
                    peer_list.anchors.len(),
                    peer_list.white.len(),
                    peer_list.grey.len()
                );
            }
            Command::AddressBook {
                zone: NetZone::Tor,
            } => {
                let mut book = network_interfaces.tor_network_interface.as_mut().unwrap().address_book();

                let AddressBookResponse::Peerlist(peer_list) = book
                    .ready()
                    .await
                    .unwrap()
                    .call(AddressBookRequest::Peerlist)
                    .await
                    .unwrap()
                else {
                    unreachable!()
                };

                println!("{peer_list:#?}");
                println!(
                    "len: anchor: {}, white: {}, grey: {}",
                    peer_list.anchors.len(),
                    peer_list.white.len(),
                    peer_list.grey.len()
                );
            }
        }
    }
}
