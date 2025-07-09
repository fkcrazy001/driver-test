#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate bare_test;

#[bare_test::tests]
mod tests {
    use bare_test::{
        globals::{PlatformInfoKind, global_val},
        mem::iomap,
        println,
    };
    use log::info;
    use pl011::PhytiumUart;

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
        let uart_regs = dbt
            .find_compatible(&["arm,pl011"])
            .next()
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();
        let base = uart_regs.address;

        let mut mmio = iomap((base as usize).into(), uart_regs.size.unwrap());
        let mut uart = PhytiumUart::new(unsafe { mmio.as_mut() });
        let message = ['p', 'h', 'y', 't', 'i', 'u', 'm', '!', '!', '\r', '\n'];
        for c in message {
            uart.put_byte_poll(c as u8);
        }
    }
}
