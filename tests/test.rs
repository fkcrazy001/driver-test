#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate bare_test;

#[bare_test::tests]
mod tests {
    use bare_test::{
        GetIrqConfig,
        globals::{PlatformInfoKind, global_val},
        irq::{IrqInfo, IrqParam},
        mem::iomap,
        println,
    };
    use log::info;
    use my_driver::{mutex::Mutex, uart::pl011::PhytiumUart};

    static PL011: Mutex<Option<PhytiumUart>> = Mutex::new(None);

    #[test]
    fn it_works() {
        info!("This is a test log message.");
        let a = 2;
        let b = 2;
        assert_eq!(a + b, 4);
        println!("test passed!");
    }
    #[test]
    fn test_uart_send() {
        // uart1 send actual write value on screen
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let dbt = fdt.get();
        let node = dbt.find_compatible(&["arm,pl011"]).next().unwrap();
        let uart_regs = node.reg().unwrap().next().unwrap();
        let irq_info = node.irq_info().unwrap();
        let cfg = irq_info.cfgs[0].clone();
        println!("irq info {irq_info:?}");
        let base = uart_regs.address;

        let mut mmio = iomap((base as usize).into(), uart_regs.size.unwrap());

        let uart = PhytiumUart::new(unsafe { mmio.as_mut() });
        {
            let mut ug = PL011.lock();
            *ug = Some(uart);
        }

        spin_on::spin_on(async {
            let mut ug = PL011.lock();
            let uart = ug.as_mut().unwrap();
            uart.init_irq(100_000_000, 115200);
            // register rx中断号
            IrqParam {
                intc: irq_info.irq_parent,
                cfg,
            }
            .register_builder(|_| {
                unsafe {
                    PL011.force_use().as_mut().unwrap().handle_interrupt();
                }
                bare_test::irq::IrqHandleResult::Handled
            })
            .register();
            let message = b"hello,phytium\r\n";
            uart.write_bytes(message).await;
            println!("uart = {:?}", uart);
        });
    }
}
