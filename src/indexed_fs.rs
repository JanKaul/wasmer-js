use std::io::{Read, Seek, Write};
use std::{io::Cursor, path::Path};
use wasm_bindgen::{prelude::*, JsCast};
use wasmer_vfs::{
    DirEntry, FileOpener, FileSystem, FileType, FsError, Metadata, OpenOptions, ReadDir,
    VirtualFile,
};

static FS_NAME: &str = "indexedFS";

// #[wasm_bindgen(module = "https://esm.sh/browserfs")] // for tests
#[wasm_bindgen(module = "sync-idb-fs")]
extern "C" {
    #[derive(Debug)]
    pub type FS;

    #[wasm_bindgen(method, catch, js_name = mkdirSync)]
    fn mkdir(this: &FS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch , js_name = rmdirSync)]
    fn rmdir(this: &FS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch , js_name = readdirSync)]
    fn readdir(this: &FS, filepath: String) -> Result<js_sys::Array, JsValue>;

    #[wasm_bindgen(method, catch , js_name = writeFileSync)]
    fn writeFile(this: &FS, filepath: String, data: js_sys::Uint8Array) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch , js_name = unlinkSync)]
    fn deleteFile(this: &FS, filepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch , js_name = readFileSync)]
    fn readFile(this: &FS, filepath: String) -> Result<js_sys::Uint8Array, JsValue>;
    #[wasm_bindgen(method, catch , js_name = renameSync)]
    fn rename(this: &FS, oldFilepath: String, newFilepath: String) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch , js_name = statSync)]
    fn stat(this: &FS, filepath: String) -> Result<StatsLike, JsValue>;
    #[wasm_bindgen(method, catch , js_name = statSync)]
    fn lstat(this: &FS, filepath: String) -> Result<StatsLike, JsValue>;

    type StatsLike;

    #[wasm_bindgen(method , getter, js_name = type)]
    fn file_type(this: &StatsLike) -> String;
}

fn get_fs() -> Result<FS, FsError> {
    let global = js_sys::global();
    let fs = js_sys::Reflect::get(&global, &JsValue::from_str(&FS_NAME))
        .map_err(|_| FsError::UnknownError)?;
    if js_sys::Reflect::has(&fs, &"promises".into()).map_err(|_| FsError::UnknownError)? {
        Ok(fs.unchecked_into::<FS>())
    } else {
        Err(FsError::UnknownError)
    }
}

#[derive(Debug, Clone)]
pub struct IndexedFS;

impl IndexedFS {
    pub fn new(fs: FS) -> Result<IndexedFS, JsValue> {
        let global = js_sys::global();
        js_sys::Reflect::set(&global, &JsValue::from_str(&FS_NAME), &fs)?;
        Ok(IndexedFS)
    }
}

impl FileSystem for IndexedFS {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let array = get_fs()?.readdir(path.clone()).map_err(catchFsError)?;
        let data = array
            .iter()
            .map(|x| {
                let move_path = path.clone();
                {
                    let name: js_sys::JsString = x.dyn_into().map_err(|_| FsError::UnknownError)?;
                    let name: String = format!("{}", name).into();
                    let stats = get_fs()?
                        .stat(move_path.clone() + "/" + &name)
                        .map_err(catchFsError)?;
                    Ok(DirEntry {
                        path: name.into(),
                        metadata: get_metadata(&stats.file_type()),
                    })
                }
            })
            .collect::<Result<_, FsError>>()?;
        Ok(ReadDir::new(data))
    }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        get_fs()?.mkdir(path.to_string()).map_err(catchFsError)
    }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        get_fs()?.rmdir(path.to_string()).map_err(catchFsError)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let from = from.to_str().ok_or(FsError::UnknownError)?.to_string();
        let to = to.to_str().ok_or(FsError::UnknownError)?.to_string();
        get_fs()?
            .rename(from.to_string(), to.to_string())
            .map_err(catchFsError)
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let stats = get_fs()?.stat(path.to_string()).map_err(catchFsError)?;
        get_metadata(&stats.file_type())
    }
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let stats = get_fs()?.lstat(path.to_string()).map_err(catchFsError)?;
        get_metadata(&stats.file_type())
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        get_fs()?.deleteFile(path.to_string()).map_err(catchFsError)
    }
    fn new_open_options(&self) -> OpenOptions {
        OpenOptions::new(Box::new(IndexedFileOpener))
    }
}

fn catchFsError(err: JsValue) -> FsError {
    if let Ok(err) = err.dyn_into::<js_sys::Error>() {
        if format!("{}", err.message()).starts_with("ENOENT") {
            FsError::EntityNotFound
        } else if format!("{}", err.message()).starts_with("ENOTDIR") {
            FsError::BaseNotDirectory
        } else if format!("{}", err.message()).starts_with("EISDIR") {
            FsError::NotAFile
        } else {
            FsError::UnknownError
        }
    } else {
        FsError::UnknownError
    }
}

pub struct IndexedFileOpener;

impl FileOpener for IndexedFileOpener {
    fn open(
        &mut self,
        path: &Path,
        _conf: &wasmer_vfs::OpenOptionsConfig,
    ) -> wasmer_vfs::Result<Box<dyn VirtualFile + Send + Sync + 'static>> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let data: js_sys::Uint8Array =
            get_fs()?.readFile(path.to_string()).map_err(catchFsError)?;
        let metadata = get_metadata("file")?;
        Ok(Box::new(IndexedVirtualFile {
            path: path.to_string(),
            metadata: metadata,
            data: Cursor::new(data.to_vec()),
        }))
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
    } else if file_type == "symlink" {
        Ok(Metadata {
            ft: FileType {
                dir: false,
                file: false,
                symlink: true,
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
pub struct IndexedVirtualFile {
    path: String,
    metadata: Metadata,
    data: Cursor<Vec<u8>>,
}

impl Read for IndexedVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.data.read(buf)
    }
}

impl Write for IndexedVirtualFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.data.flush()?;
        let data = js_sys::Uint8Array::from(self.data.get_ref().as_ref());
        let temp_path = self.path.clone();
        get_fs()
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::ConnectionAborted, FsError::UnknownError)
            })?
            .writeFile(temp_path, data)
            .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::ConnectionAborted, catchFsError(err))
            })
    }
}

impl Seek for IndexedVirtualFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.data.seek(pos)
    }
}

impl VirtualFile for IndexedVirtualFile {
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
        get_fs()?
            .deleteFile(self.path.to_string())
            .map_err(catchFsError)
    }
}
