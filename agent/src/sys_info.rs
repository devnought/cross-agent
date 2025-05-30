use sysinfo::{Disk, Disks, System};

pub fn display() {
    let mut sys = System::new_all();

    // First we update all information of our `System` struct.
    sys.refresh_all();

    // println!("=> system:");
    // RAM and swap information:
    // println!("total memory: {} bytes", sys.total_memory());
    // println!("used memory : {} bytes", sys.used_memory());
    // println!("total swap  : {} bytes", sys.total_swap());
    // println!("used swap   : {} bytes", sys.used_swap());

    // Display system information:
    // println!("System name:             {:?}", System::name());
    // println!("System kernel version:   {:?}", System::kernel_version());
    // println!("System OS version:       {:?}", System::os_version());
    // println!("System host name:        {:?}", System::host_name());

    // Number of CPUs:
    // println!("CPUs: {}", sys.cpus().len());

    // Display processes ID, name na disk usage:
    // for (pid, process) in sys.processes() {
    //     println!("[{pid}] {:?} {:?}", process.name(), process.disk_usage());
    // }

    // We display all disks' information:
    println!("=> disks:");
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        println!("{disk:?}");
    }

    let total_space = disks
        .iter()
        .filter(|disk| !disk.is_removable())
        .map(Disk::total_space)
        .sum::<u64>();

    println!("Total space: `{total_space}`");

    // Network interfaces name, total data received and total data transmitted:
    // let networks = Networks::new_with_refreshed_list();
    // println!("=> networks:");
    // for (interface_name, data) in &networks {
    //     println!(
    //         "{interface_name}: {} B (down) / {} B (up)",
    //         data.total_received(),
    //         data.total_transmitted(),
    //     );
    //     // If you want the amount of data received/transmitted since last call
    //     // to `Networks::refresh`, use `received`/`transmitted`.
    // }

    // Components temperature:
    // let components = Components::new_with_refreshed_list();
    // println!("=> components:");
    // for component in &components {
    //     println!("{component:?}");
    // }
}
