use std::fmt::Error;
use std::io::{Read, Seek, Write};
use std::sync::Arc;
use std::{io::Cursor, path::Path};
use wasm_bindgen::{prelude::*, JsCast};
use wasmer_vfs::{
    DirEntry, FileOpener, FileSystem, FsError, Metadata, OpenOptions, ReadDir, VirtualFile,
};

#[wasm_bindgen(module = "@isomorphic-git/lightning-fs")]
extern "C" {
    #[derive(Debug)]
    type FS;

    #[wasm_bindgen(constructor)]
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
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct LightningFS {
    inner: Arc<FS>,
}

impl LightningFS {
    pub fn new() -> Result<Self, JsValue> {
        Ok(LightningFS {
            inner: Arc::new(FS::new("lightning_fs".to_string())),
        })
    }
}

unsafe impl Sync for FS {}
unsafe impl Send for FS {}

impl FileSystem for LightningFS {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        let result = futures::executor::block_on(
            self.inner
                .promises()
                .readdir(path.to_str().ok_or(FsError::UnknownError)?.to_string()),
        )
        .map_err(|_| FsError::UnknownError)?;
        let array: js_sys::Array = result.dyn_into().map_err(|_| FsError::UnknownError)?;
        Ok(ReadDir::new(
            array
                .iter()
                .map(|x| {
                    let name: js_sys::JsString = x.dyn_into().map_err(|_| FsError::UnknownError)?;
                    Ok(DirEntry {
                        path: format!("{}", name).into(),
                        metadata: Ok(Metadata::default()),
                    })
                })
                .collect::<Result<_, FsError>>()?,
        ))
    }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        futures::executor::block_on(
            self.inner
                .promises()
                .mkdir(path.to_str().ok_or(FsError::UnknownError)?.to_string()),
        )
        .map_err(|_| FsError::UnknownError)
    }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        futures::executor::block_on(
            self.inner
                .promises()
                .rmdir(path.to_str().ok_or(FsError::UnknownError)?.to_string()),
        )
        .map_err(|_| FsError::UnknownError)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        futures::executor::block_on(self.inner.promises().rename(
            from.to_str().ok_or(FsError::UnknownError)?.to_string(),
            to.to_str().ok_or(FsError::UnknownError)?.to_string(),
        ))
        .map_err(|_| FsError::UnknownError)
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        Ok(Metadata::default())
    }
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        unimplemented!()
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        futures::executor::block_on(
            self.inner
                .promises()
                .deleteFile(path.to_str().ok_or(FsError::UnknownError)?.to_string()),
        )
        .map_err(|_| FsError::UnknownError)
    }
    fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(Box::new(LightningFileOpener {
            fs: Arc::clone(&self.inner),
        }))
    }
}

pub struct LightningFileOpener {
    fs: Arc<FS>,
}

impl FileOpener for LightningFileOpener {
    fn open(
        &mut self,
        path: &Path,
        conf: &wasmer_vfs::OpenOptionsConfig,
    ) -> wasmer_vfs::Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let data: js_sys::Uint8Array =
            futures::executor::block_on(self.fs.promises().readFile(path.clone()))
                .map_err(|_| FsError::UnknownError)?
                .dyn_into()
                .map_err(|_| FsError::UnknownError)?;

        Ok(Box::new(LightningVirtualFile {
            path,
            fs: Arc::clone(&self.fs),
            metadata: Metadata::default(),
            data: Cursor::new(data.to_vec()),
        }))
    }
}

#[derive(Debug)]
pub struct LightningVirtualFile {
    path: String,
    fs: Arc<FS>,
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
        futures::executor::block_on(self.fs.promises().writeFile(self.path.clone(), data)).map_err(
            |err| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, FsError::UnknownError),
        )
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

    fn set_len(&mut self, new_size: u64) -> Result<(), FsError> {
        Err(FsError::UnknownError)
    }

    fn unlink(&mut self) -> Result<(), FsError> {
        Err(FsError::UnknownError)
    }
}
