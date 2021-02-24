mod libil2cpp;

#[macro_use]
extern crate dlopen_derive;

use dlopen::wrapper::{Container, WrapperApi};
use libil2cpp::{Il2CppAssembly, Il2CppClass, Il2CppDomain, Il2CppImage, Il2CppObject, MethodInfo};
use log::{error, info};
use std::ffi::{c_void, CStr, CString};
use std::{mem, panic};
use std::ops::Deref;

extern "C" {
    pub fn A64HookFunction(symbol: *mut c_void, replace: *mut c_void, result: *mut *mut c_void);
}

#[no_mangle]
pub extern "C" fn setup() {
    panic::set_hook(Box::new(|panic_info| {
        let (filename, line) = panic_info
            .location()
            .map(|loc| (loc.file(), loc.line()))
            .unwrap_or(("<unknown>", 0));

        let cause = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(String::deref);

        let cause = cause.unwrap_or_else(|| {
            panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| *s)
                .unwrap_or("<cause unknown>")
        });

        error!("A panic occurred at {}:{}: {}", filename, line, cause);
    }));

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("RustTest")
            .with_min_level(log::Level::Trace),
    );
}

static mut original: Option<extern "C" fn(this: &Il2CppObject)> = None;

pub extern "C" fn hook(this: &Il2CppObject) {
    info!("Hello from rust!");

    // Accessing a mutable static is unsafe
    unsafe {
        original.unwrap()(this);
    }
}

fn print_memory(region: *const u8, size: isize, reason: &str) {
    info!("Printing {} bytes at {:p}: {}", size, region, reason);
    let mut str = String::new();
    for i in 0..size {
        str += &format!("{:02x} ", unsafe { *region.offset(i) });
    }
    info!("{}", str);
}

#[derive(WrapperApi)]
struct LibIl2Cpp {
    il2cpp_class_get_method_from_name:
        extern "C" fn(class: &Il2CppClass, name: *const u8, argsCount: u32) -> &'static MethodInfo,
    il2cpp_domain_get: extern "C" fn() -> &'static Il2CppDomain,
    il2cpp_domain_get_assemblies:
        extern "C" fn(domain: &Il2CppDomain, size: &mut usize) -> &'static [&'static Il2CppAssembly],
    il2cpp_assembly_get_image: extern "C" fn(assembly: &Il2CppAssembly) -> Option<&'static Il2CppImage>,
    il2cpp_class_from_name:
        extern "C" fn(image: &Il2CppImage, namespace: *const u8, name: *const u8) -> Option<&'static Il2CppClass>,
}

#[no_mangle]
pub extern "C" fn load() {
    info!("Installing RustTest hooks!");

    // Information about the method to hook
    let namespace = CString::new("").unwrap();
    let classname = CString::new("MainSettingsModelSO").unwrap();
    let method_name = CString::new("OnEnable").unwrap();
    let method_args_count = 0;

    let mut libil2cpp: Container<LibIl2Cpp> = unsafe { Container::load("libil2cpp.so") }.unwrap();

    let domain = libil2cpp.il2cpp_domain_get();

    let mut assemblies_count = 0;
    let assemblies = libil2cpp.il2cpp_domain_get_assemblies(domain, &mut assemblies_count);

    for i in 0..assemblies_count {
        let assembly = assemblies[i];

        // For some reason, an assembly might not have an image
        let image = libil2cpp.il2cpp_assembly_get_image(assembly);
        if image.is_none() {
            continue;
        }

        let class = libil2cpp.il2cpp_class_from_name(image.unwrap(), namespace.as_ptr(), classname.as_ptr());
        if let Some(class) = class {
            let method = libil2cpp.il2cpp_class_get_method_from_name(class, method_name.as_ptr(), method_args_count);

            info!("Found method, hooking now...");
            unsafe {
                A64HookFunction(
                    mem::transmute::<unsafe extern "C" fn(), *mut c_void>(method.methodPointer.unwrap()),
                    mem::transmute::<extern "C" fn(&Il2CppObject), *mut c_void>(hook),
                    mem::transmute::<&Option<extern "C" fn(&Il2CppObject)>, *mut *mut c_void>(
                        &original,
                    ),
                );
            }

            break;
        } else {
            info!(
                "Could not find class MainSettingsModelSO in {}",
                unsafe { CStr::from_ptr(image.unwrap().name) }
                    .to_str()
                    .unwrap()
            );
        }
    }

    info!("Installed RustTest hooks!");
}
