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

use std::path;

use async_recursion::async_recursion;
use aws_sdk_s3::{Credentials, Region};
use aws_sdk_s3::types::ByteStream;
use serde::Deserialize;
use serde::Serialize;

use futures::future::join_all;

use crate::constant;

pub struct S3Client {
    client: aws_sdk_s3::Client,
}

pub fn from_env() -> S3Client {
    new(constant::MINIO_HOST.to_string(), *constant::MINIO_PORT, constant::MINIO_ACCESS_KEY.to_string(), constant::MINIO_SECRET_KEY.to_string())
}

pub fn new(host: String, port: u16, access_key: String, secret_key: String) -> S3Client {
    let client = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::config::Config::builder()
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "faked",
            ))
            .region(Region::new("us-east-1"))
            .endpoint_resolver(aws_sdk_s3::Endpoint::immutable(format!("http://{}:{}", host, port).parse().unwrap()))
            .build(),
    );
    S3Client {
        client,
    }
}

#[derive(Deserialize)]
pub struct CreateBucketReq {
    pub bucket_name: String,
}

#[derive(Serialize, Debug)]
pub struct ListBucketResp {
    pub bucket_name: String,
}

#[derive(Serialize, Debug)]
pub struct ListObjectResp {
    pub object_name: String,
}

impl S3Client {
    pub async fn create_bucket(&self, req: CreateBucketReq) -> Result<(), Box<dyn std::error::Error>> {
        self.client.create_bucket().bucket(req.bucket_name.as_str()).send().await?;
        Ok(())
    }

    pub async fn delete_bucket(&self, bucket_name: String) -> Result<(), Box<dyn std::error::Error>> {
        self.client.delete_bucket().bucket(bucket_name).send().await?;
        Ok(())
    }

    pub async fn list_buckets(&self) -> Result<Vec<ListBucketResp>, Box<dyn std::error::Error>> {
        let resp = self.client.list_buckets().send().await?;
        let buckets = resp.buckets.unwrap();
        let mut bucket_names = Vec::new();
        for bucket in buckets {
            bucket_names.push(ListBucketResp {
                bucket_name: bucket.name.unwrap(),
            });
        }
        Ok(bucket_names)
    }

    pub async fn list_objects(&self, bucket_name: String) -> Result<Vec<ListObjectResp>, Box<dyn std::error::Error>> {
        let resp = self.client.list_objects().bucket(bucket_name).send().await?;
        match resp.contents() {
            Some(contents) => {
                let mut object_names = Vec::new();
                for content in contents {
                    object_names.push(ListObjectResp {
                        object_name: content.key().unwrap().to_string(),
                    });
                }
                Ok(object_names)
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn put_object(&self, bucket_name: String, object_name: String, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.client.put_object().bucket(bucket_name).key(object_name).body(ByteStream::from(data)).send().await?;
        Ok(())
    }

    pub async fn get_object_hex(&self, bucket_name: String, object_name: String) -> Result<String, Box<dyn std::error::Error>> {
        match self.get_object(bucket_name, object_name).await {
            Ok(data) => {
                let hex = hex::encode(data);
                Ok(hex)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_object(&self, bucket_name: String, object_name: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let resp = self.client.get_object().bucket(bucket_name).key(object_name).send().await?;
        let result = resp.body.collect().await;
        match result {
            Ok(data) => Ok(data.into_bytes().to_vec()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn delete_object(&self, bucket_name: String, object_name: String) -> Result<(), Box<dyn std::error::Error>> {
        self.client.delete_object().bucket(bucket_name).key(object_name).send().await?;
        Ok(())
    }

    pub async fn upload_file(&self, bucket_name: String, object_name: String, file_path: String) -> Result<(), Box<dyn std::error::Error>> {
        self.client.put_object().bucket(bucket_name).key(object_name).body(ByteStream::from_path(file_path).await.unwrap()).send().await?;
        Ok(())
    }

    pub async fn backup(&self, file_path: String, bucket_name: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut buckets = Vec::new();
        if bucket_name == "null" || bucket_name.is_empty() {
            buckets = self.list_buckets().await?;
        } else {
            buckets.push(ListBucketResp{bucket_name});
        }
        for bucket in buckets {
            let path = format!("{}{}{}", file_path, path::MAIN_SEPARATOR, bucket.bucket_name);
            log::info!("Exporting bucket {} to {}", bucket.bucket_name, path);
            let objects = self.list_objects(bucket.bucket_name.to_string()).await?;
            let mut v = Vec::new();
            for object in objects {
                v.push(self.backup_object(bucket.bucket_name.to_string(), object.object_name.to_string(), path.to_string()));
            }
            join_all(v).await;
        }
        Ok(())
    }

    async fn backup_object(&self, bucket_path: String, bucket_name: String, object_name: String) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("{}{}{}", bucket_path, path::MAIN_SEPARATOR, object_name.clone());
        std::fs::create_dir_all(path::Path::new(&path).parent().unwrap())?;
        log::info!("Exporting object {} to {}", object_name.clone(), path.clone());
        let data = self.get_object(bucket_name, object_name.clone()).await?;
        match std::fs::write(path.clone(), data) {
            Ok(_) => log::info!("Exported object {} to {}", object_name.clone(), path),
            Err(e) => log::error!("Error exporting object {} to {}: {}", object_name.clone(), path.clone(), e),
        }
        Ok(())
    }

    pub async fn restore(&self, file_path: String) -> Result<(), Box<dyn std::error::Error>> {
        for child_dir in std::fs::read_dir(file_path)? {
            let child_dir = child_dir?;
            let bucket_name = child_dir.file_name().to_str().unwrap().to_string();
            let bucket_path = child_dir.path().to_str().unwrap().to_string();
            log::info!("Restoring bucket {} from {}", bucket_name, bucket_path);
            match self.create_bucket(CreateBucketReq { bucket_name: bucket_name.clone() }).await {
                Ok(_) => {
                    log::info!("Bucket {} created", bucket_name);
                }
                Err(e) => {
                    log::error!("Failed to create bucket {}: {}", bucket_name, e);
                }
            }
            let prefix = "";
            match self.upload_dir(bucket_name, prefix.to_string(), &bucket_path).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to upload dir {}: prefix: {} {}", bucket_path, prefix, e);
                }
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn upload_dir(&self, bucket_name: String, prefix: String, bucket_path: &String) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Uploading dir {} to bucket {} with prefix {}", bucket_path, bucket_name, prefix);
        for child_file in std::fs::read_dir(&bucket_path)? {
            let child_file = child_file?;
            let object_name = child_file.file_name().to_str().unwrap().to_string();
            let file_path = child_file.path().to_str().unwrap().to_string();
            // if file_path is dir, upload recursively
            if child_file.path().is_dir() {
                let new_prefix = if prefix.is_empty() {
                    object_name.clone()
                } else {
                    format!("{}/{}", prefix, object_name)
                };
                match self.upload_dir(bucket_name.clone(), new_prefix.to_string(), &file_path).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Failed to upload dir {}: prefix: {} {}", file_path, new_prefix, e);
                    }
                }
            } else {
                let object_name = format!("{}/{}", prefix, object_name);
                log::info!("Uploading file {} to bucket {} with object name {}", file_path, bucket_name, object_name);
                match self.upload_file(bucket_name.clone(), object_name.clone(), file_path.clone()).await {
                    Ok(_) => {
                        log::info!("Uploaded {} to {}/{}", file_path, bucket_name, object_name);
                    }
                    Err(e) => {
                        log::error!("Failed to upload {} to {}/{}: {}", file_path, bucket_name, object_name, e);
                    }
                }
            }
        }
        Ok(())
    }
}
