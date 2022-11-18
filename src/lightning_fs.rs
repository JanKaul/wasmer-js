use futures_lite::stream::StreamExt;
use std::io::{Read, Seek, Write};
use std::sync::Arc;
use std::{io::Cursor, path::Path};
use wasm_bindgen::{prelude::*, JsCast};
use wasmer_vfs::{
    DirEntry, FileOpener, FileSystem, FileType, FsError, Metadata, OpenOptions, ReadDir,
    VirtualFile,
};

static FS_NAME: &str = "lightningFS";

// #[wasm_bindgen(module = "https://esm.sh/@isomorphic-git/lightning-fs")] // for tests
#[wasm_bindgen(module = "@isomorphic-git/lightning-fs")]
extern "C" {
    #[derive(Debug)]
    #[wasm_bindgen( js_name = default)]
    type FS;

    #[wasm_bindgen(constructor, js_class = default)]
    fn new(name: String) -> FS;
    #[wasm_bindgen(method, getter)]
    fn promises(this: &FS) -> PromisifiedFS;

    type PromisifiedFS;

    #[wasm_bindgen(method, catch)]
    async fn mkdir(this: &PromisifiedFS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    async fn rmdir(this: &PromisifiedFS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    async fn readdir(this: &PromisifiedFS, filepath: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch)]
    async fn writeFile(
        this: &PromisifiedFS,
        filepath: String,
        data: js_sys::Uint8Array,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch, js_name = unlink)]
    async fn deleteFile(this: &PromisifiedFS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    async fn readFile(this: &PromisifiedFS, filepath: String) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(method, catch)]
    async fn rename(
        this: &PromisifiedFS,
        oldFilepath: String,
        newFilepath: String,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    async fn stat(this: &PromisifiedFS, filepath: String) -> Result<JsValue, JsValue>;
}

fn get_fs() -> Result<FS, FsError> {
    let global = js_sys::global();
    let fs = js_sys::Reflect::get(&global, &JsValue::from_str(FS_NAME))
        .map_err(|_| FsError::UnknownError)?;
    let fs: FS = fs.dyn_into().map_err(|_| FsError::UnknownError)?;
    Ok(fs)
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct LightningFS;

impl LightningFS {
    pub fn new() -> Result<LightningFS, JsValue> {
        let global = js_sys::global();
        let fs = FS::new(FS_NAME.to_string());
        js_sys::Reflect::set(&global, &JsValue::from_str(FS_NAME), &fs)?;
        Ok(LightningFS)
    }
}

impl FileSystem for LightningFS {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            let result = get_fs()?
                .promises()
                .readdir(path.clone().as_ref().to_string())
                .await
                .map_err(|_| FsError::UnknownError)?;
            let array: js_sys::Array = result.dyn_into().map_err(|_| FsError::UnknownError)?;
            let data = futures_lite::stream::iter(array.iter())
                .then(|x| {
                    let move_path = Arc::clone(&path);
                    async move {
                        let name: js_sys::JsString =
                            x.dyn_into().map_err(|_| FsError::UnknownError)?;
                        let name: String = format!("{}", name).into();
                        let stats = get_fs()?
                            .promises()
                            .stat(move_path.clone().as_ref().to_string() + "/" + &name)
                            .await
                            .map_err(|_| FsError::UnknownError)?;
                        let file_type =
                            js_sys::Reflect::get(&stats, &JsValue::from_str("file_type"))
                                .map_err(|_| FsError::UnknownError)?;
                        let file_type: js_sys::JsString =
                            file_type.dyn_into().map_err(|_| FsError::UnknownError)?;
                        let file_type: String = format!("{}", file_type).into();
                        Ok::<_, FsError>((name, file_type))
                    }
                })
                .fold(Ok::<Vec<DirEntry>, FsError>(Vec::new()), |acc, x| {
                    let mut acc = acc?;
                    let (name, file_type) = x?;
                    acc.push(DirEntry {
                        path: name.into(),
                        metadata: get_metadata(&file_type),
                    });
                    Ok(acc)
                })
                .await?;
            Ok(ReadDir::new(data))
        })
        .map_err(|_: FsError| FsError::UnknownError)
    }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            get_fs().unwrap().promises().mkdir(path.to_string()).await
        })
        .map_err(|_| FsError::UnknownError)
    }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            get_fs().unwrap().promises().rmdir(path.to_string()).await
        })
        .map_err(|_| FsError::UnknownError)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let from = Arc::new(from.to_str().ok_or(FsError::UnknownError)?.to_string());
        let to = Arc::new(to.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            get_fs()
                .unwrap()
                .promises()
                .rename(from.to_string(), to.to_string())
                .await
        })
        .map_err(|_| FsError::UnknownError)
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            let stats = get_fs()?
                .promises()
                .stat(path.to_string())
                .await
                .map_err(|_| FsError::UnknownError)?;
            let file_type = js_sys::Reflect::get(&stats, &JsValue::from_str("file_type"))
                .map_err(|_| FsError::UnknownError)?;
            let file_type: js_sys::JsString =
                file_type.dyn_into().map_err(|_| FsError::UnknownError)?;
            let file_type: String = format!("{}", file_type).into();
            get_metadata(&file_type)
        })
        .map_err(|_| FsError::UnknownError)
    }
    fn symlink_metadata(&self, _path: &Path) -> Result<Metadata, FsError> {
        unimplemented!()
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        futures_lite::future::block_on(async move {
            get_fs()
                .unwrap()
                .promises()
                .deleteFile(path.to_string())
                .await
        })
        .map_err(|_| FsError::UnknownError)
    }
    fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(Box::new(LightningFileOpener))
    }
}

pub struct LightningFileOpener;

impl FileOpener for LightningFileOpener {
    fn open(
        &mut self,
        path: &Path,
        _conf: &wasmer_vfs::OpenOptionsConfig,
    ) -> wasmer_vfs::Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        let path = Arc::new(path.to_str().ok_or(FsError::UnknownError)?.to_string());
        let result = futures_lite::future::block_on(async move {
            let data: js_sys::Uint8Array = get_fs()?
                .promises()
                .readFile(path.to_string())
                .await
                .map_err(|_| FsError::UnknownError)?
                .dyn_into()
                .map_err(|_| FsError::UnknownError)?;
            let stats = get_fs()?
                .promises()
                .stat(path.to_string())
                .await
                .map_err(|_| FsError::UnknownError)?;
            let file_type = js_sys::Reflect::get(&stats, &JsValue::from_str("file_type"))
                .map_err(|_| FsError::UnknownError)?;
            let file_type: js_sys::JsString =
                file_type.dyn_into().map_err(|_| FsError::UnknownError)?;
            let file_type: String = format!("{}", file_type).into();
            let metadata = get_metadata(&file_type)?;

            Ok::<_, FsError>(LightningVirtualFile {
                path: path.to_string(),
                metadata: metadata,
                data: Cursor::new(data.to_vec()),
            })
        });
        Ok(Box::new(result.map_err(|_| FsError::UnknownError)?))
    }
}

fn get_metadata(file_type: &str) -> Result<Metadata, FsError> {
    if file_type == "file" {
        Ok(Metadata {
            ft: FileType {
                dir: false,
                file: true,
                symlink: false,
                char_device: false,
                block_device: false,
                socket: false,
                fifo: false,
            },
            accessed: 0,
            created: 0,
            modified: 0,
            len: 0,
        })
    } else if file_type == "dir" {
        Ok(Metadata {
            ft: FileType {
                dir: true,
                file: false,
                symlink: false,
                char_device: false,
                block_device: false,
                socket: false,
                fifo: false,
            },
            accessed: 0,
            created: 0,
            modified: 0,
            len: 0,
        })
    } else {
        Err(FsError::UnknownError)
    }
}

#[derive(Debug)]
pub struct LightningVirtualFile {
    path: String,
    metadata: Metadata,
    data: Cursor<Vec<u8>>,
}

impl Read for LightningVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.data.read(buf)
    }
}

impl Write for LightningVirtualFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.data.flush()?;
        let data = js_sys::Uint8Array::from(self.data.get_ref().as_ref());
        let temp_path = self.path.clone();
        futures_lite::future::block_on(async move {
            get_fs()
                .unwrap()
                .promises()
                .writeFile(temp_path, data)
                .await
        })
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::ConnectionAborted, FsError::UnknownError)
        })
    }
}

impl Seek for LightningVirtualFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.data.seek(pos)
    }
}

impl VirtualFile for LightningVirtualFile {
    fn last_accessed(&self) -> u64 {
        self.metadata.accessed
    }

    fn last_modified(&self) -> u64 {
        self.metadata.modified
    }

    fn created_time(&self) -> u64 {
        self.metadata.created
    }

    fn size(&self) -> u64 {
        self.metadata.len
    }

    fn set_len(&mut self, _new_size: u64) -> Result<(), FsError> {
        Err(FsError::UnknownError)
    }

    fn unlink(&mut self) -> Result<(), FsError> {
        Err(FsError::UnknownError)
    }
}
