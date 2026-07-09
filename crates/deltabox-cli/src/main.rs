use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use deltabox_core::manifest::ReplicaPolicyMode;
use deltabox_core::{AddOptions, Vault};

#[derive(Debug, Parser)]
#[command(name = "deltabox")]
#[command(about = "AI-ready decentralized personal file core")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    vault: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    Add {
        source: PathBuf,
        #[arg(long)]
        path: Option<String>,
    },
    List {
        #[arg(long)]
        all: bool,
    },
    Info {
        file_id: String,
    },
    Search {
        query: String,
        #[arg(long)]
        all: bool,
    },
    Backend {
        #[command(subcommand)]
        command: BackendCommand,
    },
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    Storage {
        #[command(subcommand)]
        command: StorageCommand,
    },
    Restore {
        file_id: String,
        output: PathBuf,
    },
    Delete {
        file_id: String,
    },
    Trash {
        #[command(subcommand)]
        command: TrashCommand,
    },
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
    Purge {
        file_id: String,
    },
    Gc,
}

#[derive(Debug, Subcommand)]
enum TrashCommand {
    List,
    Restore { file_id: String },
}

#[derive(Debug, Subcommand)]
enum TagCommand {
    Create {
        name: String,
        #[arg(long, default_value = "generic")]
        tag_type: String,
    },
    List,
    Rename {
        old_name: String,
        new_name: String,
    },
    Delete {
        name: String,
    },
    Attach {
        file_id: String,
        name: String,
    },
    Detach {
        file_id: String,
        name: String,
    },
    File {
        file_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum IndexCommand {
    File {
        file_id: String,
    },
    Enqueue {
        file_id: String,
    },
    Rebuild,
    Jobs,
    Run {
        #[arg(long, default_value_t = 100)]
        limit: u64,
    },
    Retry {
        job_id: String,
    },
    Cancel {
        job_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum BackendCommand {
    List,
    AddLocal {
        backend_id: String,
        path: PathBuf,
    },
    AddS3 {
        backend_id: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        #[arg(long, default_value = "us-east-1")]
        region: String,
        #[arg(long)]
        access_key: String,
        #[arg(long)]
        secret_key: String,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        allow_http: bool,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        path_style: bool,
    },
}

#[derive(Debug, Subcommand)]
enum StorageCommand {
    Copy {
        file_id: String,
        target_backend: String,
    },
    Locations {
        file_id: String,
    },
    Verify {
        file_id: String,
    },
    RemoveLocation {
        file_id: String,
        backend_id: String,
        #[arg(long)]
        force: bool,
    },
    Move {
        file_id: String,
        target_backend: String,
    },
    SetPolicy {
        file_id: String,
        mode: String,
        #[arg(long, default_value_t = 1)]
        min_full_copies: u64,
        #[arg(long, value_delimiter = ',')]
        preferred_backends: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        cache_backends: Vec<String>,
        #[arg(long)]
        local_cache_ttl_days: Option<u64>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => {
            Vault::init(&cli.vault)?;
            println!("initialized deltabox vault at {}", cli.vault.display());
        }
        Command::Add { source, path } => {
            let vault = Vault::open(&cli.vault)?;
            let manifest = vault.add_file(AddOptions {
                source,
                logical_path: path,
            })?;
            println!("added {} {}", manifest.file_id, manifest.logical_path);
        }
        Command::List { all } => {
            let vault = Vault::open(&cli.vault)?;
            for file in vault.list_files(all)? {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    file.file_id,
                    file.status.as_str(),
                    file.size,
                    file.imported_at,
                    file.logical_path
                );
            }
        }
        Command::Info { file_id } => {
            let vault = Vault::open(&cli.vault)?;
            let manifest = vault.get_manifest(&file_id)?;
            println!("{}", serde_json_pretty(&manifest)?);
        }
        Command::Search { query, all } => {
            let vault = Vault::open(&cli.vault)?;
            for file in vault.search_files(&query, all)? {
                println!(
                    "{}\t{}\t{}\t{}",
                    file.file_id,
                    file.status.as_str(),
                    file.size,
                    file.logical_path
                );
            }
        }
        Command::Backend { command } => {
            let vault = Vault::open(&cli.vault)?;
            match command {
                BackendCommand::List => {
                    for backend in vault.list_backends()? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            backend.backend_id,
                            backend.backend_type,
                            backend.status,
                            redact_backend_config(&backend.config_json)
                        );
                    }
                }
                BackendCommand::AddLocal { backend_id, path } => {
                    let backend = vault.add_local_backend(&backend_id, path)?;
                    println!(
                        "{}\t{}\t{}\t{}",
                        backend.backend_id,
                        backend.backend_type,
                        backend.status,
                        redact_backend_config(&backend.config_json)
                    );
                }
                BackendCommand::AddS3 {
                    backend_id,
                    endpoint,
                    bucket,
                    region,
                    access_key,
                    secret_key,
                    prefix,
                    allow_http,
                    path_style,
                } => {
                    let backend = vault.add_s3_backend(
                        &backend_id,
                        endpoint,
                        bucket,
                        region,
                        access_key,
                        secret_key,
                        prefix,
                        allow_http,
                        path_style,
                    )?;
                    println!(
                        "{}\t{}\t{}\t{}",
                        backend.backend_id,
                        backend.backend_type,
                        backend.status,
                        redact_backend_config(&backend.config_json)
                    );
                }
            }
        }
        Command::Index { command } => {
            let vault = Vault::open(&cli.vault)?;
            match command {
                IndexCommand::File { file_id } => {
                    let job = vault.index_file(&file_id)?;
                    println!(
                        "{}\t{}\t{}\t{}/{}\t{}",
                        job.job_id,
                        job.file_id,
                        job.status,
                        job.completed_tasks,
                        job.total_tasks,
                        job.last_error.unwrap_or_default()
                    );
                }
                IndexCommand::Enqueue { file_id } => {
                    let job = vault.enqueue_index_file(&file_id)?;
                    println!(
                        "{}\t{}\t{}\t{}/{}\t{}",
                        job.job_id,
                        job.file_id,
                        job.status,
                        job.completed_tasks,
                        job.total_tasks,
                        job.last_error.unwrap_or_default()
                    );
                }
                IndexCommand::Rebuild => {
                    let jobs = vault.rebuild_text_index()?;
                    for job in jobs {
                        println!(
                            "{}\t{}\t{}\t{}/{}\t{}",
                            job.job_id,
                            job.file_id,
                            job.status,
                            job.completed_tasks,
                            job.total_tasks,
                            job.last_error.unwrap_or_default()
                        );
                    }
                }
                IndexCommand::Run { limit } => {
                    let summary = vault.run_index_worker(limit)?;
                    println!(
                        "completed={}\tfailed={}\tskipped={}",
                        summary.completed, summary.failed, summary.skipped
                    );
                }
                IndexCommand::Retry { job_id } => {
                    let job = vault.retry_index_job(&job_id)?;
                    println!(
                        "{}\t{}\t{}\t{}/{}\t{}",
                        job.job_id,
                        job.file_id,
                        job.status,
                        job.completed_tasks,
                        job.total_tasks,
                        job.last_error.unwrap_or_default()
                    );
                }
                IndexCommand::Cancel { job_id } => {
                    let job = vault.cancel_index_job(&job_id)?;
                    println!(
                        "{}\t{}\t{}\t{}/{}\t{}",
                        job.job_id,
                        job.file_id,
                        job.status,
                        job.completed_tasks,
                        job.total_tasks,
                        job.last_error.unwrap_or_default()
                    );
                }
                IndexCommand::Jobs => {
                    for job in vault.list_index_jobs()? {
                        println!(
                            "{}\t{}\t{}\t{}\t{}/{}\t{}",
                            job.job_id,
                            job.file_id,
                            job.job_type,
                            job.status,
                            job.completed_tasks,
                            job.total_tasks,
                            job.last_error.unwrap_or_default()
                        );
                    }
                }
            }
        }
        Command::Storage { command } => {
            let vault = Vault::open(&cli.vault)?;
            match command {
                StorageCommand::Copy {
                    file_id,
                    target_backend,
                } => {
                    let manifest = vault.copy_file_to_backend(&file_id, &target_backend)?;
                    println!(
                        "copied {}\t{}\tchunks={}",
                        manifest.file_id,
                        target_backend,
                        manifest.chunks.len()
                    );
                }
                StorageCommand::Locations { file_id } => {
                    for location in vault.file_locations(&file_id)? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            location.chunk_id,
                            location.backend_id,
                            location.status,
                            location.object_key
                        );
                    }
                }
                StorageCommand::Verify { file_id } => {
                    for record in vault.verify_file_locations(&file_id)? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            record.chunk_id, record.backend_id, record.ok, record.message
                        );
                    }
                }
                StorageCommand::RemoveLocation {
                    file_id,
                    backend_id,
                    force,
                } => {
                    let manifest = vault.remove_file_location(&file_id, &backend_id, force)?;
                    println!(
                        "removed-location {}\t{}\tchunks={}",
                        manifest.file_id,
                        backend_id,
                        manifest.chunks.len()
                    );
                }
                StorageCommand::Move {
                    file_id,
                    target_backend,
                } => {
                    let manifest = vault.move_file_to_backend(&file_id, &target_backend)?;
                    println!(
                        "moved {}\t{}\tchunks={}",
                        manifest.file_id,
                        target_backend,
                        manifest.chunks.len()
                    );
                }
                StorageCommand::SetPolicy {
                    file_id,
                    mode,
                    min_full_copies,
                    preferred_backends,
                    cache_backends,
                    local_cache_ttl_days,
                } => {
                    let mode = ReplicaPolicyMode::from_str(&mode)
                        .ok_or_else(|| anyhow::anyhow!("unknown replica policy mode: {mode}"))?;
                    let manifest = vault.set_replica_policy(
                        &file_id,
                        mode,
                        min_full_copies,
                        preferred_backends,
                        cache_backends,
                        local_cache_ttl_days,
                    )?;
                    println!("{}", serde_json_pretty(&manifest.replica_policy)?);
                }
            }
        }
        Command::Restore { file_id, output } => {
            let vault = Vault::open(&cli.vault)?;
            let restored = vault.restore_file(&file_id, output)?;
            println!("restored to {}", restored.display());
        }
        Command::Delete { file_id } => {
            let vault = Vault::open(&cli.vault)?;
            let manifest = vault.move_to_trash(&file_id)?;
            println!(
                "moved to trash {} {}",
                manifest.file_id, manifest.logical_path
            );
        }
        Command::Trash { command } => {
            let vault = Vault::open(&cli.vault)?;
            match command {
                TrashCommand::List => {
                    for file in vault.list_trash()? {
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            file.file_id, file.size, file.trashed_at, file.name, file.previous_path
                        );
                    }
                }
                TrashCommand::Restore { file_id } => {
                    let manifest = vault.restore_from_trash(&file_id)?;
                    println!(
                        "restored from trash {} {}",
                        manifest.file_id, manifest.logical_path
                    );
                }
            }
        }
        Command::Tag { command } => {
            let vault = Vault::open(&cli.vault)?;
            match command {
                TagCommand::Create { name, tag_type } => {
                    let tag = vault.create_tag(&name, &tag_type)?;
                    println!("tag {}\t{}\t{}", tag.tag_id, tag.tag_type, tag.name);
                }
                TagCommand::List => {
                    for tag in vault.list_tags()? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            tag.tag_id, tag.tag_type, tag.source, tag.name
                        );
                    }
                }
                TagCommand::Rename { old_name, new_name } => {
                    let tag = vault.rename_tag(&old_name, &new_name)?;
                    println!("renamed tag {}\t{}", tag.tag_id, tag.name);
                }
                TagCommand::Delete { name } => {
                    vault.delete_tag(&name)?;
                    println!("deleted tag {name}");
                }
                TagCommand::Attach { file_id, name } => {
                    let tag = vault.attach_tag(&file_id, &name)?;
                    println!("attached tag {}\t{} to {}", tag.tag_id, tag.name, file_id);
                }
                TagCommand::Detach { file_id, name } => {
                    vault.detach_tag(&file_id, &name)?;
                    println!("detached tag {name} from {file_id}");
                }
                TagCommand::File { file_id } => {
                    for tag in vault.tags_for_file(&file_id)? {
                        println!(
                            "{}\t{}\t{}\t{}",
                            tag.tag_id, tag.tag_type, tag.source, tag.name
                        );
                    }
                }
            }
        }
        Command::Purge { file_id } => {
            let vault = Vault::open(&cli.vault)?;
            vault.purge_file(&file_id)?;
            println!("purged {file_id}");
        }
        Command::Gc => {
            let vault = Vault::open(&cli.vault)?;
            let removed = vault.gc_chunks()?;
            println!("removed {removed} unreferenced chunks");
        }
    }
    Ok(())
}

fn serde_json_pretty(value: &impl serde::Serialize) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

fn redact_backend_config(config_json: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(config_json) else {
        return config_json.to_owned();
    };
    if let Some(object) = value.as_object_mut() {
        if object.contains_key("secret_key") {
            object.insert(
                "secret_key".to_owned(),
                serde_json::Value::String("***".to_owned()),
            );
        }
        if object.contains_key("access_key") {
            object.insert(
                "access_key".to_owned(),
                serde_json::Value::String("***".to_owned()),
            );
        }
    }
    value.to_string()
}
