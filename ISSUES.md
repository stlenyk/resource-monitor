# Issues

## Frequency measurement doesn't work on windows

`sysinfo` uses winapi's [CallNtPowerInformation](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-callntpowerinformation) function to check frequency. The `PROCESSOR_POWER_INFORMATION.CurrentMhz` ([reference](https://learn.microsoft.com/en-us/windows/win32/power/processor-power-information-str)) is treated as the result. Somewhere in ~2021 however, the function's behaviour was changed to instead return maximum frequency (at the time of writing the winapi's docs are not updated to reflect that). See [this thread](https://github.com/microsoft/Windows-Dev-Performance/issues/100) for more info.

`sysinfo`'s [code](https://github.com/GuillaumeGomez/sysinfo/blob/a1a87de366df2cdc9f5448b926ac22292ed4a826/src/windows/cpu.rs#L473-L496)

```rs
use core::{mem, ptr::null_mut};
use ntapi::ntpoapi::PROCESSOR_POWER_INFORMATION;
use winapi::um::{powerbase::CallNtPowerInformation, winnt::ProcessorInformation};

// --snip--

pub fn get_frequencies(nb_cpus: usize) -> Vec<u64> {
    let size = nb_cpus * mem::size_of::<PROCESSOR_POWER_INFORMATION>();
    let mut infos: Vec<PROCESSOR_POWER_INFORMATION> = Vec::with_capacity(nb_cpus);

    unsafe {
        if CallNtPowerInformation(
            ProcessorInformation,
            null_mut(),
            0,
            infos.as_mut_ptr() as _,
            size as _,
        ) == 0
        {
            infos.set_len(nb_cpus);
            // infos.Number
            return infos
                .into_iter()
                .map(|i| i.CurrentMhz as u64)
                .collect::<Vec<_>>();
        }
    }
    vec![0; nb_cpus]
}

```

### Possible solution

Inspired by:

* <https://github.com/oshi/oshi/issues/966#issuecomment-713035564>
* <https://github.com/heim-rs/heim/issues/232#issuecomment-748381783>

Explore [Win32_PerfFormattedData_Counters_ProcessorInformation](https://wutils.com/wmi/root/cimv2/win32_perfformatteddata_counters_processorinformation/#percentofmaximumfrequency_properties). Especially [PercentPerformanceLimit](https://wutils.com/wmi/root/cimv2/win32_perfformatteddata_counters_processorinformation/#percentofmaximumfrequency_properties) and [PercentPerformanceLimit](https://wutils.com/wmi/root/cimv2/win32_perfformatteddata_counters_processorinformation/#percentperformancelimit_properties).

During quick testing, despite the naming, the second one seemed to be producing values closer resembling Windows Task Manager's.

Sample code

```rs
use std::{collections::HashMap, thread::sleep, time::Duration};
use wmi::{COMLibrary, Variant, WMIConnection};

fn main() {
    // my cpu's max frequency
    // (or the value I took from PROCESSOR_POWER_INFORMATION.CurrentMhz to be precise :/)
    let max_freq = 1792;

    let com_con = COMLibrary::new().unwrap();
    let wmi_con = WMIConnection::new(com_con).unwrap();

    let query_item = "PercentProcessorPerformance";
    let query = format!(
        "SELECT {} FROM Win32_PerfFormattedData_Counters_ProcessorInformation WHERE NAME='_TOTAL'",
        query_item
    );

    loop {
        let results: Vec<HashMap<String, Variant>> = wmi_con.raw_query(&query).unwrap();
        print!("\x1B[2J\x1B[1;1H");
        for procs in results {
            let freq = procs.get(query_item).unwrap();
            if let Variant::UI8(freq) = freq {
                println!("{:.2} GHz", (freq * max_freq) as f32 / 100.0 / 1000.0);
            }
        }
        sleep(Duration::from_millis(1000));
    }
}

```
