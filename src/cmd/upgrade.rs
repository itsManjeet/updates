use std::time::{Duration, Instant};

use clap::{Arg, ArgAction, ArgMatches, Command};
use humansize::{format_size, DECIMAL};
use humantime::format_duration;

use ostree::{
    gio::Cancellable, AsyncProgress, RepoPullFlags, Sysroot, SysrootUpgrader,
    SysrootUpgraderPullFlags,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("failed to aquire lock")]
    FailedToAquireLock,
}

pub fn cmd() -> Command {
    Command::new("upgrade")
        .about("Upgrade System")
        .long_about("Check and apply system updates")
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .help("Check for updates only")
                .action(ArgAction::SetTrue),
        )
}

pub async fn run(args: &ArgMatches) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let sysroot = Sysroot::new_default();

    sysroot.load(cancellable)?;

    if !sysroot.try_lock()? {
        return Err(Error::FailedToAquireLock);
    }

    let upgrader = SysrootUpgrader::new(&sysroot, cancellable)?;
    if let Some(origin) = upgrader.dup_origin() {
        ostree::Deployment::origin_remove_transient_state(&origin);
        upgrader.set_origin(Some(&origin), cancellable)?;
    }

    let progress = AsyncProgress::new();

    progress.connect_changed(|p| {
        let mut message = String::new();
        let outstanding_fetches = p.variant("outstanding-fetches").unwrap().get::<u32>().unwrap();
        let status = p.variant("status").unwrap().get::<String>().unwrap();
        let caught_error = p.variant("caught-error").unwrap().get::<bool>().unwrap();
        let outstanding_writes = p.variant("outstanding-writes").unwrap().get::<u32>().unwrap();
        let scanned_metadata = p.variant("scanned-metadata").unwrap().get::<u32>().unwrap();
        if !status.is_empty() {
            message.push_str(&status);
        } else if caught_error {
            if caught_error {
                message.push_str("caught error, waiting for outstanding tasks");
            }
        } else if outstanding_fetches > 0  {
            // let outstanding_fetches = outstanding_fetches.get::<u32>().unwrap();
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
            let scanning = p
                .variant("scanning")
                .unwrap()
                .get::<u32>()
                .unwrap();
            let outstanding_metadata_fetches = p
                .variant("outstanding-metadata-fetches")
                .unwrap()
                .get::<u32>()
                .unwrap();
            let metadata_fetched = p
                .variant("metadata-fetched")
                .unwrap()
                .get::<u32>()
                .unwrap();
            
            let start_time = p.variant("start-time").unwrap().get::<u64>().unwrap();
            let current_time = Instant::now().elapsed().as_micros() as u64;

            if (current_time - start_time < 1_000_000) || bytes_transferred == 0 {
                bytes_sec = 0;
                formatted_bytes_sec.push('-');
            } else {
                bytes_sec = bytes_transferred / ((current_time - start_time) / 1_000_000);
                formatted_bytes_sec = format_size(bytes_sec, DECIMAL);
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
                let fetched_delta_part_fallback = p
                    .variant("fetched-delta-fallbacks")
                    .unwrap()
                    .get::<u64>()
                    .unwrap();
                let total_delta_part_fallback = p
                    .variant("total-delta-fallbacks")
                    .unwrap()
                    .get::<u64>()
                    .unwrap();

                fetched_delta_part_size += fetched_delta_part_fallback;
                total_delta_part_size += total_delta_part_fallback;

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
                        "Receiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total} {formatted_bytes_sec}/s {est_time_str} remaining"
                    ));
                } else {
                    message.push_str(&format!(
                        "Receiving delta parts: {fetched_delta_parts}/{total_delta_parts} {formatted_fetched}/{formatted_total}"
                    ));
                }
            } else if scanning > 0 || outstanding_metadata_fetches > 0 {
                message.push_str(&format!("Receiving metadata objects: {metadata_fetched}/(estimating) {formatted_bytes_sec}/s {formatted_bytes_transferred}"))
            }
        } else if outstanding_writes > 0  {
            message.push_str(&format!(
                "writing objects: {}",
                outstanding_writes
            ));
        } else if scanned_metadata > 0 {
            message.push_str(&format!(
                "Scanning metadata: {}",
                scanned_metadata
            ));
        }
        print!("\r{}", message);
    });

    if !upgrader.pull(
        RepoPullFlags::COMMIT_ONLY,
        SysrootUpgraderPullFlags::NONE,
        Some(&progress),
        cancellable,
    )? {
        progress.finish();

        println!("\nno updates available");
        return Ok(());
    }

    progress.finish();
    println!();

    let repo = sysroot.repo();
    let origin = upgrader.origin().unwrap();

    let origin_ref_spec = origin.string("origin", "refspec")?;

    let rev = repo.resolve_rev(&origin_ref_spec.as_str(), false)?.unwrap();

    if args.get_flag("check") {
        let commit_info = repo.load_variant(ostree::ObjectType::Commit, rev.as_str())?;
        let subject = commit_info.child_get::<String>(3);
        let body = commit_info.child_get::<String>(4);
        let timestamp = commit_info.child_get::<u64>(5);

        println!("{timestamp}:{subject}\n{body}");
    } else {
        for deployment in sysroot.deployments() {
            if deployment.csum() == rev {
                println!("Latest revision already deployed; pending reboot");
                return Ok(());
            }
        }

        upgrader.pull(
            RepoPullFlags::NONE,
            SysrootUpgraderPullFlags::ALLOW_OLDER,
            Some(&progress),
            cancellable,
        )?;

        sysroot.cleanup(cancellable)?;
        progress.finish();
        println!();

        upgrader.deploy(cancellable)?;

        println!("Upgrade successfull!");
    }

    Ok(())
}
