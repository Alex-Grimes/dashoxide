use sysinfo::{Disks, System};

pub struct SystemState {
    pub system: System,
    pub disk: Disks,
    pub cpu_history: Vec<f32>,
    pub memory_history: Vec<(u64, u64)>,
}

impl SystemState {
    pub fn new() -> Self {
        let mut system = System::new_all();
        let disk = Disks::new_with_refreshed_list();
        system.refresh_all();

        Self {
            system,
            disk,
            cpu_history: Vec::with_capacity(60),
            memory_history: Vec::with_capacity(60),
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
    }
}
