/* tslint:disable */
/* eslint-disable */

/** Options used when configuring a new WASI instance.  */
export type WasiConfig = {
    /** The command-line arguments passed to the WASI executable. */
    readonly args?: string[];
    /** Additional environment variables made available to the WASI executable. */
    readonly env?: Record<string, string>;
    /** Preopened directories. */
    readonly preopens?: Record<string, string>;
    /** The in-memory filesystem that should be used. */
    readonly fs?: MemFS | IndexedFS;
};


/**
*/
export class JSVirtualFile {
  free(): void;
/**
* @returns {bigint}
*/
  lastAccessed(): bigint;
/**
* @returns {bigint}
*/
  lastModified(): bigint;
/**
* @returns {bigint}
*/
  createdTime(): bigint;
/**
* @returns {bigint}
*/
  size(): bigint;
/**
* @param {bigint} new_size
*/
  setLength(new_size: bigint): void;
/**
* @returns {Uint8Array}
*/
  read(): Uint8Array;
/**
* @returns {string}
*/
  readString(): string;
/**
* @param {Uint8Array} buf
* @returns {number}
*/
  write(buf: Uint8Array): number;
/**
* @param {string} buf
* @returns {number}
*/
  writeString(buf: string): number;
/**
*/
  flush(): void;
/**
* @param {number} position
* @returns {number}
*/
  seek(position: number): number;
}
/**
*/
export class MemFS {
  free(): void;
/**
* @returns {Symbol}
*/
  static __wbgd_downcast_token(): Symbol;
/**
*/
  constructor();
/**
* @param {any} jso
* @returns {MemFS}
*/
  static from_js(jso: any): MemFS;
/**
* @param {string} path
* @returns {Array<any>}
*/
  readDir(path: string): Array<any>;
/**
* @param {string} path
*/
  createDir(path: string): void;
/**
* @param {string} path
*/
  removeDir(path: string): void;
/**
* @param {string} path
*/
  removeFile(path: string): void;
/**
* @param {string} path
* @param {string} to
*/
  rename(path: string, to: string): void;
/**
* @param {string} path
* @returns {object}
*/
  metadata(path: string): object;
/**
* @param {string} path
* @param {any} options
* @returns {JSVirtualFile}
*/
  open(path: string, options: any): JSVirtualFile;
}
/**
*/
export class WASI {
  free(): void;
/**
* @param {WasiConfig} config
*/
  constructor(config: WasiConfig);
/**
* @param {WebAssembly.Module} module
* @returns {object}
*/
  getImports(module: WebAssembly.Module): object;
/**
* @param {any} module_or_instance
* @param {object | undefined} imports
* @returns {WebAssembly.Instance}
*/
  instantiate(module_or_instance: any, imports?: object): WebAssembly.Instance;
/**
* Start the WASI Instance, it returns the status code when calling the start
* function
* @param {WebAssembly.Instance | undefined} instance
* @returns {number}
*/
  start(instance?: WebAssembly.Instance): number;
/**
* Get the stdout buffer
* Note: this method flushes the stdout
* @returns {Uint8Array}
*/
  getStdoutBuffer(): Uint8Array;
/**
* Get the stdout data as a string
* Note: this method flushes the stdout
* @returns {string}
*/
  getStdoutString(): string;
/**
* Get the stderr buffer
* Note: this method flushes the stderr
* @returns {Uint8Array}
*/
  getStderrBuffer(): Uint8Array;
/**
* Get the stderr data as a string
* Note: this method flushes the stderr
* @returns {string}
*/
  getStderrString(): string;
/**
* Set the stdin buffer
* @param {Uint8Array} buf
*/
  setStdinBuffer(buf: Uint8Array): void;
/**
* Set the stdin data as a string
* @param {string} input
*/
  setStdinString(input: string): void;
}
/**
* A struct representing an aborted instruction execution, with a message
* indicating the cause.
*/
export class WasmerRuntimeError {
  free(): void;
/**
* @returns {Symbol}
*/
  static __wbgd_downcast_token(): Symbol;
}
