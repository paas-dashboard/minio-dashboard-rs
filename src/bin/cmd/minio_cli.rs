// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use clap::{Args, Parser, Subcommand};

use minio_dashboard::minio::s3_client;
use minio_dashboard::util;

#[derive(Parser)]
#[command()]
struct Cli {
    #[arg(long, default_value = "localhost")]
    host: String,
    #[arg(long, default_value = "9000")]
    port: u16,
    #[arg(long, default_value = "minioadmin")]
    access_key: String,
    #[arg(long, default_value = "minioadmin")]
    secret_key: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(name = "backup")]
    Backup(Backup),
}

#[derive(Args)]
struct Backup {
    #[arg(long)]
    action: String,
    /// Path to the import location
    #[arg(short, long)]
    path: String,
    /// backup bucket name
    #[arg(short, long)]
    bucket_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    util::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Backup(backup) => {
            let s3_client = s3_client::new(cli.host, cli.port, cli.access_key, cli.secret_key);
            match backup.action.as_str() {
                "backup" => {
                    s3_client.backup(backup.path, backup.bucket_name).await?;
                }
                "restore" => {
                    s3_client.restore(backup.path).await?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}
