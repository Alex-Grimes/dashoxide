mod dashboard;
mod util;

use std::sync::Mutex;
use std::time::Duration;
use std::{sync::Arc, thread};
use sysinfo::{Components, Cpu, Disks, Networks, System};

use dashboard::Dashboard;
use tokio::io;
use util::SystemState;

fn main() -> Result<(), io::Error> {
    let system_state = Arc::new(Mutex::new(SystemState::new()));
    let state_clone = Arc::clone(&system_state);
    thread::spawn(move || {
        loop {
            {
                let mut state = state_clone.lock().unwrap();
                state.update();
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    let mut dashboard = Dashboard::new(Arc::clone(&system_state));
    //let mut sys = System::new_all();
    //let disk_list = Disks::new_with_refreshed_list();
    //let network_list = Networks::new_with_refreshed_list();

    //    loop {
    //        sys.refresh_all();
    //        for (index, cpu) in sys.cpus().iter().enumerate() {
    //            println!("CPU {} Usage: {}%", index, cpu.cpu_usage());
    //        }

    //        println!(
    //            "Memory: {:.2} GB used / s{:.2} GB total",
    //            sys.used_memory() as f64 / 1_000_000_000.0,
    //            sys.total_memory() as f64 / 1_000_000_000.0
    //        );

    //        for disk in disk_list.list() {
    //            println!("Disk: {disk:?}");
    //        }

    //        for network in network_list.list() {
    //            println!("Network: {network:?}");
    //        }

    //        thread::sleep(Duration::from_secs(1));

    //        print!("{}[2J", 27 as char);
    //    }
    dashboard.run()?;
    Ok(())
}
