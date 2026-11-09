//! Compiles the Slint UI, and on Windows builds the exe's icon and version details: the app
//! icon is drawn from `ui/icons/ebbrise.svg` into an ICO, and the Windows SDK's `rc.exe` turns
//! both into a resource that the linker adds to the exe.

use std::path::{Path, PathBuf};

const ICON_SIZES: [u32; 7] = [16, 20, 24, 32, 48, 64, 256];

fn main() {
    slint_build::compile("ui/main.slint").expect("Slint UI");
    println!("cargo:rerun-if-changed=ui/icons/ebbrise.svg");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let icon = out.join("ebbrise.ico");
    std::fs::write(
        &icon,
        ico(&std::fs::read("ui/icons/ebbrise.svg").expect("app icon")),
    )
    .expect("ICO");
    let script = out.join("ebbrise.rc");
    std::fs::write(&script, resource_script(&icon)).expect("resource script");
    let Some(rc) = find_rc() else {
        return println!("cargo:warning=rc.exe not found: the exe has no icon or version details");
    };
    let res = out.join("ebbrise.res");
    let status = std::process::Command::new(rc)
        .args(["/nologo", "/fo"])
        .arg(&res)
        .arg(&script)
        .status()
        .expect("rc.exe runs");
    assert!(status.success(), "rc.exe failed on {}", script.display());
    println!("cargo:rustc-link-arg-bins={}", res.display());
}

/// An ICO file with one PNG image per size in [`ICON_SIZES`].
fn ico(svg: &[u8]) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()).expect("SVG");
    let images: Vec<(u32, Vec<u8>)> = ICON_SIZES
        .iter()
        .map(|&size| {
            let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("icon size");
            let scale = size as f32 / tree.size().width();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(scale, scale),
                &mut pixmap.as_mut(),
            );
            (size, pixmap.encode_png().expect("PNG"))
        })
        .collect();
    let mut file = Vec::new();
    file.extend([0, 0, 1, 0]); // reserved, type 1 = icon
    file.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in &images {
        let side = if *size >= 256 { 0 } else { *size as u8 }; // 0 means 256
        file.extend([side, side, 0, 0]); // width, height, no palette, reserved
        file.extend(1u16.to_le_bytes()); // colour planes
        file.extend(32u16.to_le_bytes()); // bits per pixel
        file.extend((png.len() as u32).to_le_bytes());
        file.extend(offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        file.extend(png);
    }
    file
}

/// The icon and the version details that Explorer shows under Properties > Details.
fn resource_script(icon: &Path) -> String {
    let version = std::env::var("CARGO_PKG_VERSION").expect("version");
    let mut parts: Vec<u16> = version
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    parts.resize(4, 0);
    let numbers = format!("{},{},{},{}", parts[0], parts[1], parts[2], parts[3]);
    let icon = icon.display().to_string().replace('\\', "\\\\");
    format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "EbbRise"
      VALUE "ProductName", "EbbRise"
      VALUE "FileVersion", "{version}"
      VALUE "ProductVersion", "{version}"
      VALUE "OriginalFilename", "ebbrise.exe"
      VALUE "InternalName", "ebbrise"
      VALUE "LegalCopyright", "Copyright (C) 2026 zantoichi. 0BSD; see THIRD-PARTY-NOTICES.txt"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}

/// The newest x64 `rc.exe` in the Windows 10/11 SDK.
fn find_rc() -> Option<PathBuf> {
    let root = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)?
        .join(r"Windows Kits\10\bin");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|entry| Some(entry.ok()?.path().join(r"x64\rc.exe")))
        .filter(|rc| rc.is_file())
        .collect();
    versions.sort();
    versions.pop()
}
