//! Manual check: follows the same guarded path as the app.
//! Latest release -> verified download -> compatibility -> unlock.
use anzu_core::compat::{self, Compat};
use anzu_core::unlock::{self, UnlockRequest};
use anzu_core::{kindle, releases, settings, tool};

fn main() {
    if let Err(e) = run() {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let data = settings::data_dir();
    let kindle = kindle::detect().ok_or("Kindle is not installed")?;
    println!("Kindle {}", kindle.version);

    let release = releases::fetch_latest()?;
    println!("Release {} ({})", release.tag, release.published_at);

    let tool = tool::ensure_tool(&release, &data.join("tool"))?;
    println!("Tool verified at {}: {:?}", tool.dir.display(), tool.exe_names);

    let exe_name = match compat::check(&kindle.version, &tool.exe_names) {
        Compat::Compatible { exe_name } => exe_name,
        other => return Err(format!("blocked: {other:?}").into()),
    };
    println!("Compatible: {exe_name}");

    let work = data.join("work");
    let out = work.join("out");
    let keyfile = data.join("keys").join("anzu.k4i");
    let exe = tool.dir.join(&exe_name);
    let req = UnlockRequest { exe: &exe, content_dir: &kindle.content_dir, out_dir: &out, keyfile: &keyfile, work_dir: &work };

    let unlocked = unlock::unlock(&req, &data, &mut |line| println!("  | {line}"))?;
    println!("Unlocked: {}", unlocked.len());
    for (asin, path) in unlocked {
        println!("  {asin} -> {}", path.display());
    }
    Ok(())
}
