use std::thread;
use std::time::Duration;
use sysinfo::{Components, Cpu, Disks, Networks, System};

fn main() {
    let mut sys = System::new_all();
    let mut disk_list = Disks::new_with_refreshed_list();
    loop {
        sys.refresh_all();
        for cpu in sys.cpus() {
            println!("CPU Usage: {}%", cpu.cpu_usage());
        }

        println!(
            "Memory: {:.2} GB used / {:.2} GB total",
            sys.used_memory() as f64 / 1_000_000_000.0,
            sys.total_memory() as f64 / 1_000_000_000.0
        );

        for disk in disk_list.list() {
            println!("Disk: {disk:?}");
        }

        thread::sleep(Duration::from_secs(1));

        print!("{}[2J", 27 as char);
    }
}
