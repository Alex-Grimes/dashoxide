use sysinfo::{Disks, Networks, System};

pub struct SystemState {
    pub system: System,
    pub disks: Vec<String>,
    pub networks: Vec<String>,
    pub cpu_history: Vec<f32>,
    pub memory_history: Vec<(u64, u64)>,
    pub disk_history: Vec<(u64, u64)>,
    pub network_history: Vec<(u64, u64)>,
}

impl SystemState {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let disks: Vec<String> = Disks::new_with_refreshed_list()
            .iter()
            .map(|disk| disk.name().to_string_lossy().into_owned())
            .collect();

        let networks: Vec<String> = Networks::new().keys().map(|name| name.clone()).collect();

        Self {
            system,
            disks,
            networks,
            cpu_history: Vec::with_capacity(60),
            memory_history: Vec::with_capacity(60),
            disk_history: Vec::with_capacity(60),
            network_history: Vec::with_capacity(60),
        }
    }

    pub fn update(&mut self) {
        self.system.refresh_all();

        let cpu_usage = self.system.global_cpu_usage();
        self.cpu_history.push(cpu_usage);
        if self.cpu_history.len() > 60 {
            self.cpu_history.remove(0);
        }

        let memory_used = self.system.used_memory();
        let memory_total = self.system.total_memory();
        self.memory_history.push((memory_used, memory_total));
        if self.memory_history.len() > 60 {
            self.memory_history.remove(0);
        }

        for (pid, process) in self.system.processes() {
            let disk_usage = process.disk_usage();
            let disk_stats: (u64, u64) = (disk_usage.read_bytes, disk_usage.written_bytes);

            self.disk_history.push(disk_stats);
            if self.disk_history.len() > 60 {
                self.disk_history.remove(0);
            }
        }
        let mut rx_bytes = 0;
        let mut tx_bytes = 0;
        let network_list = Networks::new_with_refreshed_list();
        for (_interface_name, data) in &network_list {
            rx_bytes += data.received();
            tx_bytes += data.transmitted();
        }
        self.network_history.push((rx_bytes, tx_bytes));
        if self.network_history.len() > 60 {
            self.network_history.remove(0);
        }
    }
}
