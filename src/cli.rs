use clap::Parser;
use pnet::{
    datalink::{interfaces, NetworkInterface},
    util::MacAddr,
};
use std::net::Ipv4Addr;

#[derive(Parser)]
#[clap(
    about = format!(
        "\x1b[1m{}\x1b[0m \x1b[1;36m{}\x1b[0m\n- Restrict network access for specific target in LAN via ARP",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ),
    version,
)]
pub struct DesolateArgs {
    #[clap(
        short,
        long,
        value_parser = validate_interface,
        value_name = "INTERFACE",
        long_help = "Specify network interface"
    )]
    pub iface: Option<NetworkInterface>,

    #[clap(
        short,
        long,
        value_parser,
        value_name = "IPV4",
        long_help = "Gateway IPv4"
    )]
    pub gateway: Option<Ipv4Addr>,

    #[clap(
        short,
        long,
        value_parser,
        value_name = "MAC",
        long_help = "Force specific gateway MAC"
    )]
    pub force_gateway_mac: Option<MacAddr>,

    #[clap(
        short,
        long,
        value_parser,
        value_name = "IPV4",
        long_help = "Target IPv4"
    )]
    pub target: Option<Ipv4Addr>,

    #[clap(
        short = 'F',
        long,
        value_parser,
        value_name = "MAC",
        long_help = "Force specific target MAC"
    )]
    pub force_target_mac: Option<MacAddr>,

    #[clap(
        short = 'd',
        long,
        value_parser,
        value_name = "SECONDS",
        long_help = "Duration of attack"
    )]
    pub attack_duration: Option<usize>,
}

fn validate_interface(iface_name: &str) -> Result<NetworkInterface, String> {
    let ifaces: Vec<NetworkInterface> = interfaces();
    let iface: Option<NetworkInterface> = ifaces.into_iter().find(|x| &*x.name == iface_name);

    if let Some(iface) = iface {
        return Ok(iface);
    } else {
        return Err(String::from("Specified interface does not exists"));
    }
}
