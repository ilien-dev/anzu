//! Verificación manual: recorre el mismo camino protegido que la app.
//! Publicación más reciente -> descarga verificada -> compatibilidad -> desbloqueo.
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
    let kindle = kindle::detect().ok_or("Kindle no instalada")?;
    println!("Kindle {}", kindle.version);

    let release = releases::fetch_latest()?;
    println!("Publicación {} ({})", release.tag, release.published_at);

    let tool = tool::ensure_tool(&release, &data.join("tool"))?;
    println!("Herramienta verificada en {}: {:?}", tool.dir.display(), tool.exe_names);

    let exe_name = match compat::check(&kindle.version, &tool.exe_names) {
        Compat::Compatible { exe_name } => exe_name,
        other => return Err(format!("bloqueado: {other:?}").into()),
    };
    println!("Compatible: {exe_name}");

    let work = data.join("work");
    let out = work.join("out");
    let keyfile = data.join("keys").join("anzu.k4i");
    let exe = tool.dir.join(&exe_name);
    let req = UnlockRequest { exe: &exe, content_dir: &kindle.content_dir, out_dir: &out, keyfile: &keyfile, work_dir: &work };

    let unlocked = unlock::unlock(&req, &data, &mut |line| println!("  | {line}"))?;
    println!("Desbloqueados: {}", unlocked.len());
    for (asin, path) in unlocked {
        println!("  {asin} -> {}", path.display());
    }
    Ok(())
}
