use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3Client, S3};
extern crate image;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::ImageFormat;
use log::{error, info};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use tokio::{fs::File, io};

#[derive(Debug, Deserialize)]
pub struct InputEvent {
    bucket_name: String,
    prefix: String,
    tgt_size: u32,
    tgt_ext: Option<String>,
}

struct Resizer {
    client: S3Client,
    bucket_name: String,
    prefix: String,
    tgt_size: u32,
    tgt_ext: Option<String>,
}

impl Resizer {
    fn new(event: InputEvent) -> Self {
        let client = S3Client::new(Region::ApNortheast1);

        let bucket_name = event.bucket_name;
        let prefix = event.prefix;
        let tgt_size = event.tgt_size;
        let tgt_ext = event.tgt_ext;
        Self {
            client,
            bucket_name,
            prefix,
            tgt_size,
            tgt_ext,
        }
    }

    fn get_file_name(key: String) -> String {
        let prefix_list: Vec<&str> = key.split('/').collect();
        let prefix_list_len = &prefix_list.len();
        return prefix_list[prefix_list_len - 1].to_string();
    }

    fn ext_cond(&self, key: String) -> bool {
        let file_name: String = Resizer::get_file_name(key);
        match Path::new(&file_name).extension() {
            // ファイル拡張子を取得
            Some(ext) => {
                let lc_ext = ext.to_string_lossy().to_lowercase();
                match &self.tgt_ext {
                    // リサイズ対象の拡張子が指定されている場合
                    Some(tgt_ext) => return lc_ext.eq(tgt_ext),
                    None => return lc_ext.eq("jpg") || lc_ext.eq("jpeg") || lc_ext.eq("png"),
                }
            }
            None => false,
        }
    }

    async fn list_objects(&self) -> Vec<String> {
        let bucket = &self.bucket_name;
        let prefix = &self.prefix;
        let mut request = ListObjectsV2Request {
            bucket: bucket.to_owned(),
            prefix: Some(prefix.to_string()),
            ..Default::default()
        };
        let mut resize_target_results = Vec::new();
        loop {
            let result = self.client.list_objects_v2(request.clone()).await.unwrap();
            match result.contents {
                Some(contents) => {
                    // サムネイルされていないオブジェクトキー配列
                    let not_thumbnailed_keys: Vec<String> = contents
                        .iter()
                        .filter(|&content| {
                            let key = &content.key;
                            match key {
                                Some(k) => {
                                    // ファイル拡張子の条件を取得
                                    let ext_cond: bool = self.ext_cond(k.to_string());
                                    return ext_cond && !k.clone().contains("thumb_");
                                }
                                None => false,
                            }
                        })
                        .map(|content| content.key.clone().unwrap())
                        .collect();

                    // リサイズ対象のサムネイルを持つオブジェクトキー配列
                    let target_thumb_keys: Vec<String> = contents
                        .iter()
                        .filter(|&content| {
                            let key = &content.key;
                            match key {
                                Some(k) => {
                                    // ファイル拡張子の条件を取得
                                    let ext_cond: bool = self.ext_cond(k.to_string());
                                    return ext_cond
                                        && k.clone().contains(&format!(
                                            "thumb_{size}x{size}",
                                            size = self.tgt_size
                                        ));
                                }
                                None => false,
                            }
                        })
                        .map(|content| content.key.clone().unwrap())
                        .collect();

                    for key in not_thumbnailed_keys {
                        let prefix_list: Vec<&str> = key.split('/').collect();
                        let prefix_list_len = &prefix_list.len();
                        let file_name = prefix_list[prefix_list_len - 1];
                        let id = prefix_list[prefix_list_len - 2];
                        let target_key = format!(
                            "{}/{}/thumb_{size}x{size}_{}",
                            prefix,
                            id,
                            file_name,
                            size = self.tgt_size,
                        );
                        // 既に対象のサムネイルが作成されている場合はリサイズしない
                        let already_exists =
                            Resizer::exists_object(target_thumb_keys.clone(), target_key);
                        if !already_exists {
                            resize_target_results.push(key);
                        }
                    }
                }
                None => break,
            }
            request.continuation_token = result.next_continuation_token;
            if request.continuation_token.is_none() {
                break;
            }
        }
        resize_target_results
    }

    fn exists_object(keys: std::vec::Vec<String>, target_key: String) -> bool {
        return keys.iter().any(|key| {
            return key == &target_key;
        });
    }

    async fn get_object(&self, key: &str) -> rusoto_core::ByteStream {
        let get_request = GetObjectRequest {
            bucket: self.bucket_name.to_string(),
            key: key.to_owned(),
            ..Default::default()
        };
        let mut get_response = self.client.get_object(get_request).await.unwrap();
        get_response.body.take().expect("error..")
    }

    async fn put_object(&self, key: &str) -> Result<(), &'static str> {
        let streaming_body = self.get_object(key).await;
        let mut reader = streaming_body.into_async_read();
        let temp_file_path = format!("/tmp/{}", key.to_string());
        let path = std::path::Path::new(&temp_file_path);
        let prefix = path.parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();
        let mut file = File::create(&temp_file_path).await.unwrap();
        io::copy(&mut reader, &mut file).await.unwrap();
        let thumbnail_path_name = &format!("/tmp/thumb-{}", key.to_string());
        let reader = ImageReader::open(&temp_file_path.as_str()).unwrap();
        let reader = reader.with_guessed_format().unwrap();
        let image = reader.decode().unwrap();
        let resized_image_file: image::DynamicImage = image.resize(
            self.tgt_size as u32,
            self.tgt_size as u32,
            FilterType::Lanczos3,
        );
        let thumbnail_path = std::path::Path::new(&thumbnail_path_name);
        let thumbnail_parent_path = thumbnail_path.parent().unwrap();
        std::fs::create_dir_all(thumbnail_parent_path).unwrap();
        let mut output = std::fs::File::create(thumbnail_path).unwrap();

        let file_name: String = Resizer::get_file_name(key.to_string());
        match Path::new(&file_name).extension() {
            // ファイル拡張子を取得
            Some(ext) => {
                let lc_ext = ext.to_string_lossy().to_lowercase();
                match lc_ext.as_str() {
                    "png" => resized_image_file
                        .write_to(&mut output, ImageFormat::Png)
                        .unwrap(),
                    "jpg" | "jpeg" => resized_image_file
                        .write_to(&mut output, ImageFormat::Jpeg)
                        .unwrap(),
                    _ => panic!("invalid image supplied, only jpg, jpeg, png are supported"),
                }
            }
            None => panic!("extension does not exist"),
        }

        let mut put_request = PutObjectRequest {
            bucket: self.bucket_name.to_string(),
            ..Default::default()
        };
        let object_path = Path::new(key);
        let file_name = object_path.file_stem().unwrap().to_str().unwrap();
        let extension = object_path
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\"", "");
        match object_path.parent().unwrap().to_str() {
            Some(parent) => {
                put_request.key = format!(
                    "{}/thumb_{size}x{size}_{}.{}",
                    parent.replace("\"", ""),
                    file_name.to_string().replace("\"", ""),
                    extension,
                    size = self.tgt_size,
                )
            }
            None => {
                put_request.key = format!(
                    "thumb_{size}x{size}_{}.{}",
                    file_name.to_string().replace("\"", ""),
                    extension,
                    size = self.tgt_size,
                )
            }
        }
        let mut thumbnail_file = std::fs::File::open(thumbnail_path).unwrap();
        let mut contents: Vec<u8> = Vec::new();
        thumbnail_file.read_to_end(&mut contents).unwrap();
        put_request.body = Some(contents.into());
        let _ = self
            .client
            .put_object(put_request)
            .await
            .or_else(|e| Err(e));
        std::fs::remove_file(thumbnail_path_name).unwrap();
        std::fs::remove_file(temp_file_path).unwrap();

        Ok(())
    }
}

pub fn run(event: InputEvent) {
    let resizer = Resizer::new(event);
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let keys: Vec<String> = rt.block_on(resizer.list_objects());
    info!("Object list length: {}", keys.len());
    for key in keys {
        if !key.is_empty() {
            match rt.block_on(resizer.put_object(&key)) {
                Ok(_) => info!("Resize completed: {}/{}", resizer.bucket_name, key),
                Err(err) => error!("Resize failed: {:?}", err),
            };
        }
    }
}
