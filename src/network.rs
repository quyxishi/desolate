use pnet::{
    datalink::{channel, Channel::Ethernet, NetworkInterface},
    packet::{
        arp::{ArpHardwareTypes, ArpOperation, ArpPacket, MutableArpPacket},
        ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket},
        MutablePacket,
    },
    util::MacAddr,
};
use std::{
    io,
    net::Ipv4Addr,
    time::{Duration, Instant},
};

use crate::recli;

pub fn send_arp_packet(
    operation: ArpOperation,
    iface: &NetworkInterface,
    source: &(Ipv4Addr, MacAddr),
    target: &(Ipv4Addr, MacAddr),
) -> Option<io::Result<()>> {
    let (mut tx, _) = match channel(&iface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => recli::panicln!("*unhandled channel type*"),
        Err(err) => recli::panicln!("*unable to create datalink channel*: ~{}~", err),
    };

    let mut ether_buffer: [u8; 42] = [0; 42];
    let mut ether_packet: MutableEthernetPacket =
        MutableEthernetPacket::new(&mut ether_buffer).unwrap();

    ether_packet.set_destination(target.1);
    ether_packet.set_source(source.1);
    ether_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer: [u8; 28] = [0; 28];
    let mut arp_packet: MutableArpPacket = MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);

    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);

    arp_packet.set_operation(operation);

    arp_packet.set_sender_hw_addr(source.1);
    arp_packet.set_sender_proto_addr(source.0);

    arp_packet.set_target_hw_addr(target.1);
    arp_packet.set_target_proto_addr(target.0);

    ether_packet.set_payload(arp_packet.packet_mut());

    // *

    tx.send_to(ether_packet.packet_mut(), None)
}

pub fn receive_arp_packet(
    iface: &NetworkInterface,
    sender: &Ipv4Addr,
    timeout: Duration,
) -> Option<MacAddr> {
    let (_, mut rx) = match channel(&iface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => recli::panicln!("*unhandled channel type*"),
        Err(err) => recli::panicln!("*unable to create datalink channel*: ~{}~", err),
    };

    let entry_time: Instant = Instant::now();

    while entry_time.elapsed() < timeout {
        let ether_buffer: &[u8] = match rx.next() {
            Ok(packet_buffer) => packet_buffer,
            Err(err) => match err.kind() {
                io::ErrorKind::TimedOut => continue,
                _ => recli::panicln!("*unable to receive arp request*: ~{}~", err),
            },
        };

        let ether_packet: EthernetPacket = match EthernetPacket::new(ether_buffer) {
            Some(ether_packet) => ether_packet,
            _ => continue,
        };

        if !matches!(ether_packet.get_ethertype(), EtherTypes::Arp) {
            continue;
        }

        let arp_buffer: &[u8] = &ether_buffer[EthernetPacket::minimum_packet_size()..];
        let arp_packet: ArpPacket = match ArpPacket::new(arp_buffer) {
            Some(arp_packet) => arp_packet,
            _ => continue,
        };

        // *

        if arp_packet.get_sender_proto_addr() != *sender {
            continue;
        }

        return Some(arp_packet.get_sender_hw_addr());
    }

    None
}
