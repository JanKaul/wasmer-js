use std::io::{Read, Seek, Write};
use std::{io::Cursor, path::Path};
use wasm_bindgen::{prelude::*, JsCast};
use wasmer_vfs::{
    DirEntry, FileOpener, FileSystem, FileType, FsError, Metadata, OpenOptions, ReadDir,
    VirtualFile,
};

static FS_NAME: &str = "browserFS";

#[wasm_bindgen(module = "https://esm.sh/browserfs")] // for tests
                                                     // #[wasm_bindgen(module = "browserfs")]
extern "C" {
    #[derive(Debug)]
    #[wasm_bindgen(js_name = LocalStorage, js_namespace = ["default", "FileSystem"])]
    type FS;
    #[wasm_bindgen(constructor, js_namespace = ["default", "FileSystem"], js_class = LocalStorage)]
    fn new() -> FS;

    #[wasm_bindgen(js_namespace = default)]
    fn initialize(fs: FS) -> FS;

    #[wasm_bindgen(method, js_name = mkdirSync)]
    fn mkdir(this: &FS, filepath: String);
    #[wasm_bindgen(method, js_name = rmdirSync)]
    fn rmdir(this: &FS, filepath: String);
    #[wasm_bindgen(method, js_name = readdirSync)]
    fn readdir(this: &FS, filepath: String) -> js_sys::Array;

    #[wasm_bindgen(method, js_name = writeFileSync)]
    fn writeFile(this: &FS, filepath: String, data: js_sys::Uint8Array);
    #[wasm_bindgen(method, js_name = unlinkSync)]
    fn deleteFile(this: &FS, filepath: String);
    #[wasm_bindgen(method, js_name = readFileSync)]
    fn readFile(this: &FS, filepath: String) -> js_sys::Uint8Array;
    #[wasm_bindgen(method, js_name = renameSync)]
    fn rename(this: &FS, oldFilepath: String, newFilepath: String);
    #[wasm_bindgen(method, js_name = statSync)]
    fn stat(this: &FS, filepath: String) -> Stats;

    type Stats;

    #[wasm_bindgen(method)]
    fn isFile(this: &Stats) -> bool;
    #[wasm_bindgen(method)]
    fn isDirectory(this: &Stats) -> bool;
}

fn get_fs() -> Result<FS, FsError> {
    let global = js_sys::global();
    let fs = js_sys::Reflect::get(&global, &JsValue::from_str(&FS_NAME))
        .map_err(|_| FsError::UnknownError)?;
    fs.dyn_into().map_err(|_| FsError::UnknownError)
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct BrowserFS;

impl BrowserFS {
    pub fn new() -> Result<BrowserFS, JsValue> {
        let global = js_sys::global();
        let fs = FS::new();
        let initialized = initialize(fs);
        js_sys::Reflect::set(&global, &JsValue::from_str(&FS_NAME), &initialized)?;
        Ok(BrowserFS)
    }
}

impl FileSystem for BrowserFS {
    fn read_dir(&self, path: &Path) -> Result<ReadDir, FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let array = get_fs()?.readdir(path.clone());
        let data = array
            .iter()
            .map(|x| {
                let move_path = path.clone();
                {
                    let name: js_sys::JsString = x.dyn_into().map_err(|_| FsError::UnknownError)?;
                    let name: String = format!("{}", name).into();
                    let stats = get_fs()?.stat(move_path.clone() + "/" + &name);
                    Ok(DirEntry {
                        path: name.into(),
                        metadata: get_metadata(stats.isFile(), stats.isDirectory()),
                    })
                }
            })
            .collect::<Result<_, FsError>>()?;
        Ok(ReadDir::new(data))
    }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        Ok(get_fs()?.mkdir(path.to_string()))
    }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        Ok(get_fs()?.rmdir(path.to_string()))
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let from = from.to_str().ok_or(FsError::UnknownError)?.to_string();
        let to = to.to_str().ok_or(FsError::UnknownError)?.to_string();
        Ok(get_fs()?.rename(from.to_string(), to.to_string()))
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let stats = get_fs()?.stat(path.to_string());
        get_metadata(stats.isFile(), stats.isDirectory())
    }
    fn symlink_metadata(&self, _path: &Path) -> Result<Metadata, FsError> {
        unimplemented!()
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        Ok(get_fs()?.deleteFile(path.to_string()))
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
        let path = path.to_str().ok_or(FsError::UnknownError)?.to_string();
        let data: js_sys::Uint8Array = get_fs()?.readFile(path.to_string());
        let stats = get_fs()?.stat(path.to_string());
        let metadata = get_metadata(stats.isFile(), stats.isDirectory())?;
        Ok(Box::new(LightningVirtualFile {
            path: path.to_string(),
            metadata: metadata,
            data: Cursor::new(data.to_vec()),
        }))
    }
}

fn get_metadata(is_file: bool, is_dir: bool) -> Result<Metadata, FsError> {
    if is_file {
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
    } else if is_dir {
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
        Ok(get_fs()
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::ConnectionAborted, FsError::UnknownError)
            })?
            .writeFile(temp_path, data))
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
        Ok(get_fs()?.deleteFile(self.path.to_string()))
    }
}
