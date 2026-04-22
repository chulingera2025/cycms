//! Native 动态插件加载器。
//!
//! 该 crate 位于 workspace 外部，专门承接 `libloading` 所需的 `unsafe` 边界：
//! 上层运行时只使用这里暴露的安全 API，不直接接触符号查找和原始指针还原。

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use cycms_plugin_api::{NATIVE_PLUGIN_CREATE_SYMBOL, NATIVE_PLUGIN_CREATE_SYMBOL_NAME, Plugin};

/// 已打开的 Native 动态插件库句柄。
pub struct DynamicPluginLibrary {
    library: libloading::Library,
    path: PathBuf,
}

/// Native 动态插件加载失败。
#[derive(Debug)]
pub enum DynamicPluginLoadError {
    /// 打开动态库文件失败。
    Open {
        path: PathBuf,
        source: libloading::Error,
    },
    /// 动态库未导出宿主约定的工厂函数符号。
    MissingSymbol {
        path: PathBuf,
        source: libloading::Error,
    },
    /// 工厂函数返回了空指针。
    NullFactoryReturn { path: PathBuf },
}

impl DynamicPluginLibrary {
    /// 打开动态库文件，但不立即实例化插件对象。
    pub fn open(path: &Path) -> Result<Self, DynamicPluginLoadError> {
        let path = path.to_path_buf();
        // Safety: 动态库句柄会被 `DynamicPluginLibrary` 持有直到调用方显式 drop，
        // 不会把原始库句柄泄漏到安全 API 之外。
        let library = unsafe { libloading::Library::new(&path) }.map_err(|source| {
            DynamicPluginLoadError::Open {
                path: path.clone(),
                source,
            }
        })?;
        Ok(Self { library, path })
    }

    /// 调用宿主约定的导出工厂函数，构造一个插件实例。
    pub fn instantiate(&self) -> Result<Box<dyn Plugin>, DynamicPluginLoadError> {
        type PluginCreate = unsafe extern "C" fn() -> *mut std::ffi::c_void;

        // Safety: 符号名与 ABI 由 `cycms-plugin-api::export_plugin!` 统一约定。
        let constructor = unsafe {
            self.library
                .get::<PluginCreate>(NATIVE_PLUGIN_CREATE_SYMBOL)
        }
        .map_err(|source| DynamicPluginLoadError::MissingSymbol {
            path: self.path.clone(),
            source,
        })?;

        // Safety: 上面查到的函数指针遵循相同 ABI，返回宿主接管所有权的插件指针。
        let raw = unsafe { constructor() };
        if raw.is_null() {
            return Err(DynamicPluginLoadError::NullFactoryReturn {
                path: self.path.clone(),
            });
        }

        // Safety: 导出宏返回的是 `Box<Box<dyn Plugin>>` 的裸指针；这里按相同布局
        // 还原，并立刻拆出内层 `Box<dyn Plugin>` 交给调用方持有。
        let plugin = unsafe { Box::from_raw(raw.cast::<Box<dyn Plugin>>()) };
        Ok(*plugin)
    }
}

impl Display for DynamicPluginLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { path, source } => {
                write!(
                    f,
                    "failed to open native plugin {}: {source}",
                    path.display()
                )
            }
            Self::MissingSymbol { path, source } => write!(
                f,
                "native plugin {} does not export {}: {source}",
                path.display(),
                NATIVE_PLUGIN_CREATE_SYMBOL_NAME
            ),
            Self::NullFactoryReturn { path } => write!(
                f,
                "native plugin {} returned a null plugin factory pointer",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DynamicPluginLoadError {}
