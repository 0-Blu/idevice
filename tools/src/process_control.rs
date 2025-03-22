// Jackson Coxson

use clap::{Arg, Command};
use idevice::{core_device_proxy::CoreDeviceProxy, xpc::XPCDevice, IdeviceService};

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("process_control")
        .about("Query process control")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("IP address of the device"),
        )
        .arg(
            Arg::new("pairing_file")
                .long("pairing-file")
                .value_name("PATH")
                .help("Path to the pairing file"),
        )
        .arg(
            Arg::new("udid")
                .value_name("UDID")
                .help("UDID of the device (overrides host/pairing file)")
                .index(2),
        )
        .arg(
            Arg::new("about")
                .long("about")
                .help("Show about information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bundle_id")
                .value_name("Bundle ID")
                .help("Bundle ID of the app to launch")
                .index(1),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("process_control - launch and manage processes on the device");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let host = matches.get_one::<String>("host");

    let provider =
        match common::get_provider(udid, host, pairing_file, "heartbeat_client-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
    let bundle_id = matches
        .get_one::<String>("bundle_id")
        .expect("No bundle ID specified");

    let proxy = CoreDeviceProxy::connect(&*provider)
        .await
        .expect("no core proxy");
    let rsd_port = proxy.handshake.server_rsd_port;

    let mut adapter = proxy.create_software_tunnel().expect("no software tunnel");
    adapter.connect(rsd_port).await.expect("no RSD connect");

    // Make the connection to RemoteXPC
    let client = XPCDevice::new(Box::new(adapter)).await.unwrap();

    // Get the debug proxy
    let service = client
        .services
        .get(idevice::dvt::SERVICE_NAME)
        .expect("Client did not contain DVT service")
        .to_owned();

    let mut adapter = client.into_inner();
    adapter.connect(service.port).await.unwrap();

    let mut rs_client =
        idevice::dvt::remote_server::RemoteServerClient::new(Box::new(adapter)).unwrap();
    rs_client.read_message(0).await.expect("no read??");
    let mut pc_client = idevice::dvt::process_control::ProcessControlClient::new(&mut rs_client)
        .await
        .unwrap();

    let pid = pc_client
        .launch_app(bundle_id, None, None, true, false)
        .await
        .expect("no launch??");
    pc_client
        .disable_memory_limit(pid)
        .await
        .expect("no disable??");
    println!("PID: {pid}");
}
