use std::ffi::OsStr;
use std::io::{self};
use std::fs::OpenOptions;
use std::path::{Path};
use async_std::task;
use glob::glob;
use botw_conv;
use futures::executor::block_on;
use clap::{Arg, App};

fn main() {
	if let Err(e) = block_on(async_main()) {
		println!("Error: {}", e);
	}
}

async fn async_main() -> io::Result<()> {
	let matches = App::new("botw_saveconv - Rust edition")
		.version("1.0")
		.about("Convert BOTW saves from WiiU <-> Switch")
		.arg(Arg::with_name("INPUTDIR")
			.help("Sets the directory (containing option.sav) to convert")
			.required(true))
		.arg(Arg::with_name("no-confirm")
			.help("Disables the backup prompt")
			.takes_value(false)
			.long("no-confirm"))
		.get_matches();

	let save_dir = Path::new(matches.value_of("INPUTDIR").unwrap());
	let option_sav_path = save_dir.join("option.sav");

	if !option_sav_path.exists() {
		println!("That directory doesn't contain an option.sav file\n");
		return Ok(());
	}

	let mut option_sav = OpenOptions::new().read(true).open(&option_sav_path)?;
	let from_platform = botw_conv::get_save_platform(&mut option_sav)?;
	let to_platform = match from_platform  {
		botw_conv::SavePlatform::Switch => botw_conv::SavePlatform::WiiU,
		_ => botw_conv::SavePlatform::Switch,
	};

	let mut answer = String::new();

	if !matches.is_present("no-confirm") {
		println!("This will convert your BOTW save from {} -> {}.", from_platform, to_platform);
		println!("Make sure you made a backup first.");
		println!("Press Y to continue, or any other key to abort.");
		io::stdin().read_line(&mut answer).unwrap_or(0);

		if answer.trim_end().to_lowercase().ne("y") {
			println!("Aborted.");
			return Ok(());
		}
		println!();
	}

	println!("Starting {} -> {} conversion...\n", from_platform, to_platform);

	let mut my_futures = Vec::new();

	for entry in glob("**/*.sav").unwrap() {
		if let Ok(path) = entry {
			my_futures.push(async move {
				process_save(&path).await
			});
		}
	}

	let handles = my_futures.into_iter().map(task::spawn).collect::<Vec<_>>();
	let results = futures::future::join_all(handles).await;

	for result in results {
		if let Ok(file_path_short) = result {
			println!("Processed {}", file_path_short);
		}
	}

	println!("\nConverted successfully!");

	Ok(())
}

async fn process_save(path: &Path) -> io::Result<String> {
	let mut sav = OpenOptions::new().read(true).write(true).open(path)?;
	botw_conv::convert_save(&mut sav, path)?;

	let file_name = path.file_name()
		.and_then(OsStr::to_str)
		.and_then(|s| Some(s.to_string()))
		.ok_or(io::Error::new(io::ErrorKind::Other, "Couldn't determine file name"))?;

	let file_path_short = match path.parent().and_then(Path::file_name).and_then(OsStr::to_str) {
		Some(directory_name) => format!("{}/{}", directory_name, file_name),
		None => file_name,
	};

	Ok(file_path_short)
}