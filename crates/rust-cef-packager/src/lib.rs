use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum PackageFormat {
    App,
    Dmg,
    Wix,
    Nsis,
    Deb,
    AppImage,
    Pacman,
}

impl PackageFormat {
    fn as_cli_value(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Dmg => "dmg",
            Self::Wix => "wix",
            Self::Nsis => "nsis",
            Self::Deb => "deb",
            Self::AppImage => "appimage",
            Self::Pacman => "pacman",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CargoPackagerConfig {
    pub workspace_dir: PathBuf,
    pub formats: Vec<PackageFormat>,
    pub release: bool,
    pub target: Option<String>,
    pub out_dir: Option<PathBuf>,
    pub binaries_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct MacOsBundleConfig {
    pub workspace_dir: PathBuf,
    pub target_dir: PathBuf,
    pub app_name: String,
    pub main_exe_name: String,
    pub bundle_identifier: String,
    pub url_name: String,
    pub url_schemes: Vec<String>,
    pub document_type_name: String,
    pub document_type_identifier: String,
    pub document_extension: String,
    pub helper_bundle_id_base: String,
    pub framework_name: String,
}

#[derive(Debug, Clone)]
pub struct MacOsPackageConfig {
    pub bundle: MacOsBundleConfig,
    pub main_entitlements: PathBuf,
    pub helper_entitlements: PathBuf,
    pub signing_identity: String,
    pub dmg_name: String,
    pub create_dmg: bool,
}

pub fn package_with_cargo_packager(config: &CargoPackagerConfig) -> Result<()> {
    if config.formats.is_empty() {
        bail!("at least one package format must be specified");
    }

    let formats = config
        .formats
        .iter()
        .map(|format| format.as_cli_value())
        .collect::<Vec<_>>()
        .join(",");

    let mut command = Command::new("cargo");
    command.arg("packager");
    if config.release {
        command.arg("--release");
    }
    command.arg("--formats").arg(formats);
    if let Some(target) = &config.target {
        command.arg("--target").arg(target);
    }
    if let Some(out_dir) = &config.out_dir {
        command.arg("--out-dir").arg(out_dir);
    }
    if let Some(binaries_dir) = &config.binaries_dir {
        command.arg("--binaries-dir").arg(binaries_dir);
    }
    command.current_dir(&config.workspace_dir);

    run_command(&mut command, "cargo packager")
}

pub fn package_windows_msi(workspace_dir: &Path) -> Result<()> {
    package_with_cargo_packager(&CargoPackagerConfig {
        workspace_dir: workspace_dir.to_path_buf(),
        formats: vec![PackageFormat::Wix],
        release: true,
        target: None,
        out_dir: None,
        binaries_dir: None,
    })
}

pub fn package_windows_nsis(workspace_dir: &Path) -> Result<()> {
    package_with_cargo_packager(&CargoPackagerConfig {
        workspace_dir: workspace_dir.to_path_buf(),
        formats: vec![PackageFormat::Nsis],
        release: true,
        target: None,
        out_dir: None,
        binaries_dir: None,
    })
}

pub fn package_linux_packages(workspace_dir: &Path, formats: &[PackageFormat]) -> Result<()> {
    package_with_cargo_packager(&CargoPackagerConfig {
        workspace_dir: workspace_dir.to_path_buf(),
        formats: formats.to_vec(),
        release: true,
        target: None,
        out_dir: None,
        binaries_dir: None,
    })
}

pub fn bundle_dev_app(config: &MacOsBundleConfig) -> Result<PathBuf> {
    let framework_src = find_framework(
        &config.target_dir.join("build"),
        &config.framework_name,
    )?;
    let bundle_dir = config
        .target_dir
        .join(format!("{}.app", config.main_exe_name));
    let contents_dir = bundle_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let frameworks_dir = contents_dir.join("Frameworks");

    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir).with_context(|| format!("remove {:?}", bundle_dir))?;
    }

    fs::create_dir_all(&macos_dir)?;
    fs::create_dir_all(&frameworks_dir)?;

    fs::copy(
        config.target_dir.join(&config.main_exe_name),
        macos_dir.join(&config.main_exe_name),
    )
    .with_context(|| "copy main executable into dev app bundle")?;
    copy_dir_recursive(&framework_src, &frameworks_dir.join(&config.framework_name))?;

    let target_root = config
        .target_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.target_dir.clone());
    fs::create_dir_all(target_root.join("Frameworks"))?;
    copy_dir_recursive(
        &framework_src,
        &target_root.join("Frameworks").join(&config.framework_name),
    )?;

    let resources_dir = framework_src.join("Resources");
    if resources_dir.exists() {
        copy_dir_contents(&resources_dir, &config.target_dir)?;
    }

    let libraries_dir = framework_src.join("Libraries");
    if libraries_dir.exists() {
        copy_matching_files(&libraries_dir, &config.target_dir, ".dylib")?;
    }

    fs::write(
        contents_dir.join("Info.plist"),
        render_main_info_plist(
            &config.main_exe_name,
            &config.bundle_identifier,
            &config.app_name,
            &config.url_name,
            &config.url_schemes,
            &config.document_type_name,
            &config.document_type_identifier,
            &config.document_extension,
        ),
    )?;

    Ok(bundle_dir)
}

pub fn package_release_app(config: &MacOsPackageConfig) -> Result<PathBuf> {
    let framework_src = find_framework(
        &config.bundle.target_dir.join("build"),
        &config.bundle.framework_name,
    )?;
    let staged_frameworks_dir = config.bundle.target_dir.join("Frameworks");
    fs::create_dir_all(&staged_frameworks_dir)?;
    let staged_framework = staged_frameworks_dir.join(&config.bundle.framework_name);
    if staged_framework.exists() {
        fs::remove_dir_all(&staged_framework)?;
    }
    copy_dir_recursive(&framework_src, &staged_framework)?;

    package_with_cargo_packager(&CargoPackagerConfig {
        workspace_dir: config.bundle.workspace_dir.clone(),
        formats: vec![PackageFormat::App],
        release: true,
        target: None,
        out_dir: None,
        binaries_dir: None,
    })?;

    let bundle_dir = config
        .bundle
        .target_dir
        .join(format!("{}.app", config.bundle.app_name));
    let contents_dir = bundle_dir.join("Contents");
    let frameworks_dir = contents_dir.join("Frameworks");
    let macos_dir = contents_dir.join("MacOS");

    if !bundle_dir.exists() {
        bail!("packaged app bundle not found at {:?}", bundle_dir);
    }

    let plist_path = contents_dir.join("Info.plist");
    fs::write(
        &plist_path,
        render_main_info_plist(
            &config.bundle.main_exe_name,
            &config.bundle.bundle_identifier,
            &config.bundle.app_name,
            &config.bundle.url_name,
            &config.bundle.url_schemes,
            &config.bundle.document_type_name,
            &config.bundle.document_type_identifier,
            &config.bundle.document_extension,
        ),
    )?;

    let framework_dst = frameworks_dir.join(&config.bundle.framework_name);
    if framework_dst.exists() {
        fs::remove_dir_all(&framework_dst)?;
    }
    copy_dir_recursive(&framework_src, &framework_dst)?;

    for suffix in ["", " (GPU)", " (Plugin)", " (Renderer)"] {
        create_helper_app(&config.bundle, &bundle_dir, suffix)?;
    }

    sign_macos_bundle(config, &bundle_dir, &frameworks_dir)?;
    if config.create_dmg {
        create_dmg(
            &bundle_dir,
            &config.bundle.target_dir.join(&config.dmg_name),
            &config.bundle.app_name,
        )?;
    }

    let _ = macos_dir;
    Ok(bundle_dir)
}

fn sign_macos_bundle(config: &MacOsPackageConfig, bundle_dir: &Path, frameworks_dir: &Path) -> Result<()> {
    run_command(
        Command::new("codesign")
            .arg("--force")
            .arg("--sign")
            .arg(&config.signing_identity)
            .arg(frameworks_dir.join(&config.bundle.framework_name)),
        "codesign framework",
    )?;

    for helper in fs::read_dir(frameworks_dir)? {
        let helper = helper?;
        let path = helper.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("app")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.contains("Helper"))
                .unwrap_or(false)
        {
            run_command(
                Command::new("codesign")
                    .arg("--force")
                    .arg("--sign")
                    .arg(&config.signing_identity)
                    .arg("--entitlements")
                    .arg(&config.helper_entitlements)
                    .arg(&path),
                &format!("codesign helper {:?}", path.file_name()),
            )?;
        }
    }

    run_command(
        Command::new("codesign")
            .arg("--force")
            .arg("--sign")
            .arg(&config.signing_identity)
            .arg("--entitlements")
            .arg(&config.main_entitlements)
            .arg(bundle_dir),
        "codesign app bundle",
    )?;

    Ok(())
}

fn create_dmg(bundle_dir: &Path, dmg_path: &Path, app_name: &str) -> Result<()> {
    if dmg_path.exists() {
        fs::remove_file(dmg_path)?;
    }

    let staging_root = dmg_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".dmg-staging-{}", app_name.replace(' ', "-")));
    if staging_root.exists() {
        fs::remove_dir_all(&staging_root)?;
    }
    fs::create_dir_all(&staging_root)?;

    let staged_app = staging_root.join(
        bundle_dir
            .file_name()
            .context("bundle path is missing file name")?,
    );
    copy_dir_recursive(bundle_dir, &staged_app)?;

    run_command(
        Command::new("hdiutil")
            .arg("makehybrid")
            .arg("-hfs")
            .arg("-hfs-volume-name")
            .arg(app_name)
            .arg("-ov")
            .arg("-o")
            .arg(dmg_path)
            .arg(&staging_root),
        "hdiutil makehybrid",
    )?;

    fs::remove_dir_all(&staging_root)?;
    Ok(())
}

fn create_helper_app(config: &MacOsBundleConfig, bundle_dir: &Path, suffix: &str) -> Result<()> {
    let helper_name = format!("{} Helper{}", config.app_name, suffix);
    let helper_dir = bundle_dir
        .join("Contents/Frameworks")
        .join(format!("{}.app", helper_name));
    let helper_contents = helper_dir.join("Contents");
    let helper_macos = helper_contents.join("MacOS");

    fs::create_dir_all(&helper_macos)?;
    fs::copy(
        bundle_dir
            .join("Contents/MacOS")
            .join(&config.main_exe_name),
        helper_macos.join(&helper_name),
    )?;

    let clean_suffix = suffix
        .replace(['(', ')'], "")
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "");
    let bundle_id = if clean_suffix.is_empty() {
        config.helper_bundle_id_base.clone()
    } else {
        format!("{}.{}", config.helper_bundle_id_base, clean_suffix)
    };

    fs::write(
        helper_contents.join("Info.plist"),
        render_helper_info_plist(&helper_name, &bundle_id),
    )?;
    Ok(())
}

fn find_framework(build_dir: &Path, framework_name: &str) -> Result<PathBuf> {
    let mut stack = vec![build_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).with_context(|| format!("read {:?}", dir))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == framework_name)
                    .unwrap_or(false)
                {
                    return Ok(path);
                }
                stack.push(path);
            }
        }
    }
    bail!("could not find framework {framework_name} under {:?}", build_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("copy {:?} -> {:?}", src_path, dst_path))?;
        }
    }
    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_matching_files(src: &Path, dst: &Path, suffix: &str) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        if src_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(suffix))
            .unwrap_or(false)
        {
            fs::copy(&src_path, dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn run_command(command: &mut Command, description: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run {description}"))?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn render_main_info_plist(
    executable: &str,
    bundle_identifier: &str,
    app_name: &str,
    url_name: &str,
    url_schemes: &[String],
    document_type_name: &str,
    document_type_identifier: &str,
    document_extension: &str,
) -> String {
    let url_schemes_xml = url_schemes
        .iter()
        .map(|scheme| format!("                <string>{scheme}</string>"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{executable}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleName</key>
    <string>{app_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>{url_name}</string>
            <key>CFBundleURLSchemes</key>
            <array>
{url_schemes_xml}
            </array>
        </dict>
    </array>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>{document_type_name}</string>
            <key>LSHandlerRank</key>
            <string>Owner</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>{document_type_identifier}</string>
            </array>
        </dict>
    </array>
    <key>UTExportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key>
            <string>{document_type_identifier}</string>
            <key>UTTypeDescription</key>
            <string>{document_type_name}</string>
            <key>UTTypeConformsTo</key>
            <array>
                <string>public.data</string>
            </array>
            <key>UTTypeTagSpecification</key>
            <dict>
                <key>public.filename-extension</key>
                <array>
                    <string>{document_extension}</string>
                </array>
            </dict>
        </dict>
    </array>
    <key>PrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#
    )
}

fn render_helper_info_plist(helper_name: &str, bundle_identifier: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{helper_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleName</key>
    <string>{helper_name}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
"#
    )
}
