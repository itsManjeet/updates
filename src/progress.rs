use humansize::{format_size, DECIMAL};
use humantime::format_duration;
use ostree::AsyncProgress;
use std::time::{Duration, Instant};

pub fn update_callback(p: &AsyncProgress) {
    let mut message = String::new();
    let outstanding_fetches = p
        .variant("outstanding-fetches")
        .unwrap()
        .get::<u32>()
        .unwrap();
    let status = p.variant("status").unwrap().get::<String>().unwrap();
    let caught_error = p.variant("caught-error").unwrap().get::<bool>().unwrap();
    let outstanding_writes = p
        .variant("outstanding-writes")
        .unwrap()
        .get::<u32>()
        .unwrap();
    let scanned_metadata = p.variant("scanned-metadata").unwrap().get::<u32>().unwrap();
    if !status.is_empty() {
        message.push_str(&status);
    }

    if caught_error {
        message.push_str("\ncaught error, waiting for outstanding tasks");
    }

    if outstanding_fetches > 0 {
        let bytes_sec: u64;
        let mut formatted_bytes_sec = String::new();
        let bytes_transferred = p
            .variant("bytes-transferred")
            .unwrap()
            .get::<u64>()
            .unwrap();
        let formatted_bytes_transferred = format_size(bytes_transferred, DECIMAL);
        let fetched_delta_parts = p
            .variant("fetched-delta-parts")
            .unwrap()
            .get::<u32>()
            .unwrap();
        let total_delta_parts = p
            .variant("total-delta-parts")
            .unwrap()
            .get::<u32>()
            .unwrap();
        let scanning = p.variant("scanning").unwrap().get::<u32>().unwrap();
        let outstanding_metadata_fetches = p
            .variant("outstanding-metadata-fetches")
            .unwrap()
            .get::<u32>()
            .unwrap();
        let metadata_fetched = p.variant("metadata-fetched").unwrap().get::<u32>().unwrap();

        let start_time = p.variant("start-time").unwrap().get::<u64>().unwrap() as u128;
        let current_time = Instant::now().elapsed().as_micros() as u128;

        if (current_time - start_time < 1_000_000) || bytes_transferred == 0 {
            bytes_sec = 0;
            formatted_bytes_sec.push('-');
        } else {
            bytes_sec = bytes_transferred / ((current_time - start_time) as u64 / 1_000_000);
            formatted_bytes_sec = format_size(bytes_sec as u64, DECIMAL);
        }

        if total_delta_parts > 0 {
            let mut fetched_delta_part_size = p
                .variant("fetched-delta-part-size")
                .unwrap()
                .get::<u64>()
                .unwrap();
            let mut total_delta_part_size = p
                .variant("total-delta-part-size")
                .unwrap()
                .get::<u64>()
                .unwrap();
            let fetched_delta_part_fallback = match p.variant("fetched-delta-fallbacks") {
                Some(value) => value.get::<u32>().unwrap(),
                None => 0,
            };
            let total_delta_part_fallback = match p.variant("total-delta-fallbacks") {
                Some(value) => value.get::<u32>().unwrap(),
                None => 0,
            };

            fetched_delta_part_size += fetched_delta_part_fallback as u64;
            total_delta_part_size += total_delta_part_fallback as u64;

            let formatted_fetched = format_size(fetched_delta_part_size, DECIMAL);
            let formatted_total = format_size(total_delta_part_size, DECIMAL);

            if bytes_sec > 0 {
                let mut est_time_remaining = 0;
                if total_delta_part_size > fetched_delta_part_size {
                    est_time_remaining =
                        (total_delta_part_size - fetched_delta_part_size) / bytes_sec;
                }
                let est_time_duration = Duration::new(est_time_remaining, 0);
                let est_time_str = format_duration(est_time_duration).to_string();

                message.push_str(&format!(
                    "\nReceiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total} {formatted_bytes_sec}/s {est_time_str} remaining"
                ));
            } else {
                message.push_str(&format!(
                    "\nReceiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total}"
                ));
            }
        }

        if scanning > 0 || outstanding_metadata_fetches > 0 {
            message.push_str(&format!("\nReceiving metadata objects: {metadata_fetched}/(estimating) {formatted_bytes_sec}/s {formatted_bytes_transferred}"))
        }
    }

    if outstanding_writes > 0 {
        message.push_str(&format!("\nwriting objects: {}", outstanding_writes));
    }
    if scanned_metadata > 0 {
        message.push_str(&format!("\nScanning metadata: {}", scanned_metadata));
    }
    println!("{}", message);
}
