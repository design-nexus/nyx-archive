//! Archive's tools on the control interface (`nyx_core::control`), for Nyx Assistant: look
//! inside an archive, open it, extract it, or pack files into a new one. Every path stays
//! inside the home folder; extracting never replaces files unless asked.

use crate::sevenzip::{
    self,
    job::{CreateOpts, Format, Op, Outcome, Overwrite},
    ListError,
};
use crate::{cmd, window};
use nyx_core::control::{self, text, Reply};
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn tools() -> Value {
    json!([
        {"name": "archive.list", "description": "List what's inside an archive file (zip, 7z, tar…) in the home folder.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
        {"name": "archive.open", "description": "Show an archive's contents in the Archive window.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
        {"name": "archive.extract", "description": "Extract an archive into a folder in the home folder (by default a new folder beside it, named after it).", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "to": {"type": "string"}, "overwrite": {"type": "boolean"}}, "required": ["path"]}},
        {"name": "archive.create", "description": "Pack files or folders (in the home folder) into a new archive; the format follows the name (.7z, .zip, .tar.gz, .tar.xz).", "inputSchema": {"type": "object", "properties": {"files": {"type": "array", "items": {"type": "string"}}, "path": {"type": "string"}, "overwrite": {"type": "boolean"}}, "required": ["files", "path"]}},
    ])
}

fn archive_arg(args: &Value) -> Result<PathBuf, String> {
    let path = control::home_path(&text(args, "path")?)?;
    if !path.is_file() {
        return Err("no such file".into());
    }
    if !sevenzip::looks_like_archive(&path) {
        return Err("that doesn't look like an archive".into());
    }
    Ok(path)
}

/// Jobs run with the window's progress card: it must be up, 7-Zip there, nothing running.
fn ready() -> Result<(), String> {
    if window::window().is_none() {
        return Err("Archive isn't ready".into());
    }
    if sevenzip::binary().is_none() {
        return Err("7-Zip isn't installed (the 7zip package)".into());
    }
    if window::busy() {
        return Err("another job is still running".into());
    }
    Ok(())
}

fn finished(outcome: Outcome, done: Value) -> Result<Value, String> {
    match outcome {
        Outcome::Ok => Ok(done),
        Outcome::Cancelled => Err("cancelled".into()),
        Outcome::WrongPassword => Err("the archive needs a password".into()),
        Outcome::Failed(why) => Err(why),
    }
}

/// `name.tar.gz` → `name`, `name.zip` → `name`.
fn stem(path: &std::path::Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    for ext in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if let Some(s) = name.strip_suffix(ext) {
            return s.to_string();
        }
    }
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(name)
}

fn format_for(path: &std::path::Path) -> Option<Format> {
    let name = path.to_string_lossy().to_lowercase();
    Format::ALL.into_iter().find(|f| name.ends_with(&format!(".{}", f.id())))
}

pub fn handle(name: &str, args: Value, reply: Reply) {
    match name {
        "archive.list" => {
            let path = match archive_arg(&args) {
                Ok(p) => p,
                Err(e) => return reply(Err(e)),
            };
            let Some(binary) = sevenzip::binary() else { return reply(Err("7-Zip isn't installed (the 7zip package)".into())) };
            cmd::background(
                move || sevenzip::list(binary, &path, None),
                move |listed| {
                    reply(match listed {
                        Ok(l) => {
                            let files: Vec<Value> = l.entries.iter().take(200).map(|e| json!({"path": e.path, "folder": e.is_dir, "size": e.size})).collect();
                            Ok(json!({"entries": l.entries.len(), "files": files, "untrusted_content": true}))
                        }
                        Err(ListError::NeedsPassword) => Err("the archive needs a password".into()),
                        Err(ListError::Failed(why)) => Err(why),
                    })
                },
            );
        }
        "archive.open" => reply(archive_arg(&args).map(|p| {
            window::open_archive(p.clone());
            json!({"opened": p})
        })),
        "archive.extract" => {
            let run = || -> Result<(PathBuf, PathBuf), String> {
                let path = archive_arg(&args)?;
                let to = match args["to"].as_str() {
                    Some(t) => control::home_path(t)?,
                    None => path.with_file_name(stem(&path)),
                };
                if to.exists() && !to.is_dir() {
                    return Err("the destination is a file".into());
                }
                ready()?;
                Ok((path, to))
            };
            let (path, dest) = match run() {
                Ok(v) => v,
                Err(e) => return reply(Err(e)),
            };
            let overwrite = if args["overwrite"] == true { Overwrite::Replace } else { Overwrite::Skip };
            let op = Op::Extract { archive: path, dest: dest.clone(), paths: vec![], password: None, overwrite };
            window::run_job(op, window::JobInfo { total: 0, folder: Some(dest.clone()) }, move |outcome| {
                reply(finished(outcome, json!({"extracted_to": dest, "existing_files": if overwrite == Overwrite::Replace { "replaced" } else { "kept" }})))
            });
        }
        "archive.create" => {
            let run = || -> Result<CreateOpts, String> {
                let output = control::save_target(&args)?;
                let format = format_for(&output).ok_or("name it .7z, .zip, .tar.gz or .tar.xz")?;
                let files = args["files"].as_array().ok_or("files is required")?;
                let mut inputs = Vec::new();
                for f in files.iter().filter_map(Value::as_str).take(500) {
                    let p = control::home_path(f)?;
                    if !p.exists() {
                        return Err(format!("{f} doesn't exist"));
                    }
                    inputs.push(p);
                }
                if inputs.is_empty() {
                    return Err("give at least one file".into());
                }
                ready()?;
                Ok(CreateOpts { format, level: 5, output, inputs, password: None, encrypt_names: false, solid: true, split_mb: None, threads: None })
            };
            let opts = match run() {
                Ok(o) => o,
                Err(e) => return reply(Err(e)),
            };
            let (output, folder) = (opts.output.clone(), opts.output.parent().map(PathBuf::from));
            let count = opts.inputs.len();
            window::run_job(Op::Create(opts), window::JobInfo { total: 0, folder }, move |outcome| {
                reply(finished(outcome, json!({"created": output, "items": count})))
            });
        }
        _ => reply(Err(format!("unknown tool {name}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_formats() {
        assert_eq!(stem(std::path::Path::new("/h/photos.tar.gz")), "photos");
        assert_eq!(stem(std::path::Path::new("/h/notes.zip")), "notes");
        assert_eq!(format_for(std::path::Path::new("/h/a.tar.xz")), Some(Format::TarXz));
        assert_eq!(format_for(std::path::Path::new("/h/a.zip")), Some(Format::Zip));
        assert_eq!(format_for(std::path::Path::new("/h/a.rar")), None);
    }
}
