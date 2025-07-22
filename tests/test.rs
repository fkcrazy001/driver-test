#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate bare_test;

#[bare_test::tests]
mod tests {
    use bare_test::time::spin_delay;
    use core::{marker::PhantomData, time::Duration};
    use smoltcp::{
        iface::{Config, Interface, SocketSet},
        phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
        socket::icmp::{self, Socket as IcmpSocket},
        time::Instant,
        wire::{
            EthernetAddress, HardwareAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr,
            Ipv4Address,
        },
    };

    use bare_test::{
        fdt_parser::PciSpace,
        globals::{PlatformInfoKind, global_val},
        mem::iomap,
        println,
    };
    use igb::{Igb, Pkt, impl_trait, misc::Kernel};
    use log::{debug, info};
    use pcie::{CommandRegister, RootComplexGeneric, SimpleBarAllocator};
    const PACKET_SIZE: u32 = 2048;
    const QPN: usize = 0x100;
    const IP: IpAddress = IpAddress::v4(10, 0, 2, 15);
    const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);

    struct IgbDevice {
        device: Igb,
    }

    impl IgbDevice {
        fn new(device: Igb) -> Self {
            Self { device }
        }
    }
    struct IgbTxToken<'a> {
        device: &'a mut Igb,
    }
    struct IgbRxToken<'a> {
        pkt: Pkt,
        _phantom: PhantomData<&'a i32>,
    }
    impl<'a> RxToken for IgbRxToken<'a> {
        fn consume<R, F>(self, f: F) -> R
        where
            F: FnOnce(&[u8]) -> R,
        {
            debug!("rcv one");
            let r = f(&self.pkt);
            r
        }
    }
    impl<'a> TxToken for IgbTxToken<'a> {
        fn consume<R, F>(self, len: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
        {
            let mut buff = alloc::vec![0u8;len];
            let r = f(&mut buff);
            let pkt = igb::Pkt::new_tx(buff);
            self.device.transmit(0, pkt).unwrap();
            r
        }
    }
    impl Device for IgbDevice {
        type RxToken<'a> = IgbRxToken<'a>;
        type TxToken<'a> = IgbTxToken<'a>;
        fn receive(
            &mut self,
            _timestamp: Instant,
        ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
            self.device.receive(0).map(|pkt| {
                (
                    IgbRxToken {
                        pkt,
                        _phantom: PhantomData,
                    },
                    IgbTxToken {
                        device: &mut self.device,
                    },
                )
            })
        }

        fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
            // 释放已完成的发送请求
            Some(IgbTxToken {
                device: &mut self.device,
            })
        }

        fn capabilities(&self) -> DeviceCapabilities {
            let mut caps = DeviceCapabilities::default();
            caps.max_transmission_unit = 1500;
            caps.max_burst_size = Some(1);
            caps.medium = Medium::Ethernet;
            caps
        }
    }

    fn now() -> Instant {
        let ms = bare_test::time::since_boot().as_millis() as u64;
        Instant::from_millis(ms as i64)
    }

    #[test]
    fn it_works() {
        let mut igb = get_igb().unwrap();
        info!("status before open {:?}", igb.status());
        info!("{:?}", igb.open());

        let mac = igb.mac_addr();
        info!("mac {mac:#}");
        while !igb.status().link_up {
            spin_delay(Duration::from_secs(1));

            info!("status: {:#?}", igb.status());
        }
        info!("status after open {:?}", igb.status());

        igb.alloc_new_qeueu(0, QPN, PACKET_SIZE).unwrap();

        let mut device = IgbDevice::new(igb);
        // 设置网络配置
        let config = Config::new(HardwareAddress::Ethernet(EthernetAddress::from_bytes(
            &mac.bytes(),
        )));
        let mut iface = Interface::new(config, &mut device, now());

        // 配置 IP 地址
        let ip_addr = IpCidr::new(IP, 8);
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs.push(ip_addr).unwrap();
        });
        iface.routes_mut().add_default_ipv4_route(GATEWAY).unwrap();

        // 创建 ICMP socket
        let icmp_rx_buffer = icmp::PacketBuffer::new(
            alloc::vec![icmp::PacketMetadata::EMPTY],
            alloc::vec![0; 256],
        );
        let icmp_tx_buffer = icmp::PacketBuffer::new(
            alloc::vec![icmp::PacketMetadata::EMPTY],
            alloc::vec![0; 256],
        );

        let icmp_socket = icmp::Socket::new(icmp_rx_buffer, icmp_tx_buffer);

        let mut socket_set = SocketSet::new(alloc::vec![]);
        let icmp_handle = socket_set.add(icmp_socket);

        // 执行 ping 测试
        let ping_result = ping_gw(&mut iface, &mut device, &mut socket_set, icmp_handle);

        if ping_result {
            info!("✓ Ping test passed! Successfully pinged 127.0.0.1");
        } else {
            info!("✗ Ping test failed!");
        }
    }

    fn ping_gw(
        iface: &mut Interface,
        device: &mut IgbDevice,
        socket_set: &mut SocketSet,
        icmp_handle: smoltcp::iface::SocketHandle,
    ) -> bool {
        let target_addr = GATEWAY;
        let mut ping_sent = false;
        let mut ping_received = false;
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 1000;
        let ident = 0x22b;

        while attempts < MAX_ATTEMPTS && !ping_received {
            iface.poll(now(), device, socket_set);
            // 获取 ICMP socket
            let socket = socket_set.get_mut::<IcmpSocket>(icmp_handle);

            if !socket.is_open() {
                socket.bind(icmp::Endpoint::Ident(ident)).unwrap();
            }

            if !ping_sent && socket.can_send() {
                let icmp_repr = Icmpv4Repr::EchoRequest {
                    ident,
                    seq_no: attempts as u16,
                    data: b"ping test",
                };
                let icmp_payload = socket
                    .send(icmp_repr.buffer_len(), target_addr.into())
                    .unwrap();
                let mut icmp_packet = Icmpv4Packet::new_unchecked(icmp_payload);

                // 发送 ping
                icmp_repr.emit(&mut icmp_packet, &device.capabilities().checksum);
                ping_sent = true;
            }

            if ping_sent && socket.can_recv() {
                // 接收 ping 响应
                match socket.recv() {
                    Ok((data, addr)) => {
                        info!(
                            "Ping response received from {:?}: {:?}",
                            addr,
                            core::str::from_utf8(data)
                        );
                        ping_received = true;
                    }
                    Err(e) => {
                        info!("Failed to receive ping response: {e:?}");
                    }
                }
            }

            attempts += 1;
            spin_delay(Duration::from_millis(100));
        }

        ping_received
    }
    fn get_igb() -> Option<Igb> {
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt = fdt.get();

        let pcie = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .next()
            .unwrap()
            .into_pci()
            .unwrap();

        let mut pcie_regs = alloc::vec![];

        let mut bar_alloc = SimpleBarAllocator::default();

        for reg in pcie.node.reg().unwrap() {
            println!("pcie reg: {:#x}", reg.address);
            pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
        }

        let base_vaddr = pcie_regs[0];

        for range in pcie.ranges().unwrap() {
            info!("{range:?}");
            match range.space {
                PciSpace::Memory32 => bar_alloc.set_mem32(range.cpu_address as _, range.size as _),
                PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
                _ => {}
            }
        }

        let mut root = RootComplexGeneric::new(base_vaddr);

        for header in root.enumerate(None, Some(bar_alloc)) {
            println!("{}", header);
        }

        for header in root.enumerate_keep_bar(None) {
            if let pcie::Header::Endpoint(endpoint) = header.header {
                if !Igb::check_vid_did(endpoint.vendor_id, endpoint.device_id) {
                    continue;
                }

                endpoint.update_command(header.root, |cmd| {
                    cmd | CommandRegister::IO_ENABLE
                        | CommandRegister::MEMORY_ENABLE
                        | CommandRegister::BUS_MASTER_ENABLE
                });

                let bar_addr;
                let bar_size;
                match endpoint.bar {
                    pcie::BarVec::Memory32(bar_vec_t) => {
                        let bar0 = bar_vec_t[0].as_ref().unwrap();
                        bar_addr = bar0.address as usize;
                        bar_size = bar0.size as usize;
                    }
                    pcie::BarVec::Memory64(bar_vec_t) => {
                        let bar0 = bar_vec_t[0].as_ref().unwrap();
                        bar_addr = bar0.address as usize;
                        bar_size = bar0.size as usize;
                    }
                    pcie::BarVec::Io(_bar_vec_t) => todo!(),
                };

                println!("bar0: {:#x}", bar_addr);

                let addr = iomap(bar_addr.into(), bar_size);

                let igb = Igb::new(addr);
                return Some(igb);
            }
        }
        None
    }
    struct KernelImpl;
    impl_trait! {
        impl Kernel for KernelImpl {
            fn sleep(duration: Duration) {
                spin_delay(duration);
            }
        }
    }
}
