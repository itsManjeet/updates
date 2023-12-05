use std::io::Write;
use humansize::{format_size, DECIMAL};
use humantime::format_duration;
use ostree::AsyncProgress;
use std::time::{Duration, Instant};

pub fn update_callback(p: &AsyncProgress) {
    let mut message = String::new();
    let outstanding_fetches = p.variant("outstanding-fetches").unwrap().get::<u32>().unwrap();
    let outstanding_metadata_fetches = p.variant("outstanding-metadata-fetches").unwrap().get::<u32>().unwrap();
    let outstanding_writes = p.variant("outstanding-writes").unwrap().get::<u32>().unwrap();
    let caught_error = p.variant("caught-error").unwrap().get::<bool>().unwrap();
    let scanning = p.variant("scanning").unwrap().get::<u32>().unwrap();
    let scanned_metadata = p.variant("scanned-metadata").unwrap().get::<u32>().unwrap();
    let fetched_delta_parts = p.variant("fetched-delta-parts").unwrap().get::<u32>().unwrap();
    let total_delta_parts = p.variant("total-delta-parts").unwrap().get::<u32>().unwrap();
    let fetched_delta_part_fallbacks = p.variant("fetched-delta-fallbacks").unwrap().get::<u32>().unwrap();
    let total_delta_parts_fallback = p.variant("total-delta-fallbacks").unwrap().get::<u32>().unwrap();
    let status = p.variant("status").unwrap().get::<String>().unwrap();

    if !status.is_empty() {
        message.push_str(&status);
    } else if caught_error {
        message.push_str("caught error, waiting for outstanding tasks");
    } else if outstanding_fetches > 0 {
        let bytes_transferred = p.variant("bytes-transferred").unwrap().get::<u64>().unwrap();
        let fetched = p.variant("fetched").unwrap().get::<u32>().unwrap();
        let metadata_fetched = p.variant("metadata-fetched").unwrap().get::<u32>().unwrap();
        let requested = p.variant("requested").unwrap().get::<u32>().unwrap();
        let start_time = p.variant("start-time").unwrap().get::<u64>().unwrap() as i128;
        let total_delta_part_size = p.variant("total-delta-part-size").unwrap().get::<u64>().unwrap();
        let formatted_bytes_transferred = format_size(bytes_transferred, DECIMAL);

        let bytes_sec: u64;
        let formatted_bytes_sec: String;

        let current_time = Instant::now().elapsed().as_micros() as i128;
        if (start_time - current_time) < 10_00_000 || bytes_transferred == 0 {
            bytes_sec = 0;
            formatted_bytes_sec = String::from("-");
        } else {
            bytes_sec = (bytes_transferred as f64 / ((start_time - current_time) as f64 / 10_00_000f64)) as u64;
            formatted_bytes_sec = format_size(bytes_sec, DECIMAL);
        }

        if total_delta_parts > 0 {
            let fetched_delta_part_size = p.variant("fetched-delta-part-size").unwrap().get::<u64>().unwrap();
            let formatted_fetched: String;
            let formatted_total: String;

            let fetched_delta_parts = fetched_delta_parts + fetched_delta_part_fallbacks;
            let total_delta_parts = total_delta_parts + total_delta_parts_fallback;

            formatted_fetched = format_size(fetched_delta_part_size, DECIMAL);
            formatted_total = format_size(total_delta_part_size, DECIMAL);

            if bytes_sec > 0 {
                let est_time_remaining;
                if total_delta_part_size > fetched_delta_part_size {
                    est_time_remaining = (total_delta_part_size - fetched_delta_part_size) / bytes_sec;
                } else {
                    est_time_remaining = 0;
                }

                let formatted_est_time_remaining = format_duration(Duration::from_secs(est_time_remaining)).to_string();

                message.push_str(&format!("Receiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total}, {formatted_bytes_sec}/s {formatted_est_time_remaining}remaining"));
            } else {
                message.push_str(&format!("Receiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total}"));
            }
        } else if scanning > 0 || outstanding_metadata_fetches > 0 {
            message.push_str(&format!("Receiving metadata objects: {metadata_fetched}/(estimating) {formatted_bytes_sec}/s {formatted_bytes_transferred}"))
        } else {
            message.push_str(&format!("Receiving objects: {}% ({fetched}/{requested} {formatted_bytes_sec}/s {formatted_bytes_transferred}", ((fetched as f32 / requested as f32) * 100.0) as u32));
        }
    } else if outstanding_writes > 0 {
        message.push_str(&format!("Writing objects: {outstanding_writes}"));
    } else {
        message.push_str(&format!("Scanning metadata: {scanned_metadata}"));
    }
    print!("\r{message}              ");
    std::io::stdout().flush().unwrap();
}
