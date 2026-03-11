use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use rust_cef_packager::{
    bundle_dev_app, package_linux_packages, package_release_app, package_windows_msi,
    package_windows_nsis, MacOsBundleConfig, MacOsPackageConfig, PackageFormat,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    BundleDevMacos,
    PackageMacos {
        #[arg(long = "format", value_enum)]
        formats: Vec<MacPackageFormat>,
    },
    PackageWindowsMsi,
    PackageWindowsNsis,
    PackageLinux {
        #[arg(long = "format", value_enum)]
        formats: Vec<LinuxPackageFormat>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum MacPackageFormat {
    App,
    Dmg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum LinuxPackageFormat {
    Deb,
    #[value(name = "appimage")]
    AppImage,
    Pacman,
}

impl From<LinuxPackageFormat> for PackageFormat {
    fn from(value: LinuxPackageFormat) -> Self {
        match value {
            LinuxPackageFormat::Deb => Self::Deb,
            LinuxPackageFormat::AppImage => Self::AppImage,
            LinuxPackageFormat::Pacman => Self::Pacman,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace_dir = std::env::current_dir()?;

    let bundle = MacOsBundleConfig {
        workspace_dir: workspace_dir.clone(),
        target_dir: workspace_dir.join("target/debug"),
        app_name: "Rust CEF".to_string(),
        main_exe_name: "rust-cef".to_string(),
        bundle_identifier: "com.rustcef.app".to_string(),
        url_name: "com.rustcef.app.deeplink".to_string(),
        url_schemes: vec!["rustcef".to_string(), "rust-cef".to_string()],
        document_type_name: "Rust CEF Document".to_string(),
        document_type_identifier: "com.rustcef.document".to_string(),
        document_extension: "rustcef".to_string(),
        helper_bundle_id_base: "com.rustcef.helper".to_string(),
        framework_name: "Chromium Embedded Framework.framework".to_string(),
    };

    match cli.command {
        Command::BundleDevMacos => {
            bundle_dev_app(&bundle)?;
        }
        Command::PackageMacos { formats } => {
            let create_dmg = if formats.is_empty() {
                true
            } else {
                formats.contains(&MacPackageFormat::Dmg)
            };
            let config = MacOsPackageConfig {
                bundle: MacOsBundleConfig {
                    target_dir: workspace_dir.join("target/release"),
                    ..bundle
                },
                main_entitlements: std::env::var("RUST_CEF_MAIN_ENTITLEMENTS")
                    .map(Into::into)
                    .unwrap_or_else(|_| workspace_dir.join("Entitlements.plist")),
                helper_entitlements: std::env::var("RUST_CEF_HELPER_ENTITLEMENTS")
                    .map(Into::into)
                    .unwrap_or_else(|_| workspace_dir.join("Helper.entitlements")),
                signing_identity: std::env::var("RUST_CEF_SIGNING_IDENTITY")
                    .unwrap_or_else(|_| "-".to_string()),
                dmg_name: std::env::var("RUST_CEF_DMG_NAME")
                    .unwrap_or_else(|_| "Rust CEF.dmg".to_string()),
                create_dmg,
            };
            package_release_app(&config)?;
        }
        Command::PackageWindowsMsi => {
            package_windows_msi(&workspace_dir)?;
        }
        Command::PackageWindowsNsis => {
            package_windows_nsis(&workspace_dir)?;
        }
        Command::PackageLinux { formats } => {
            let formats = if formats.is_empty() {
                vec![PackageFormat::Deb, PackageFormat::AppImage, PackageFormat::Pacman]
            } else {
                formats.into_iter().map(Into::into).collect()
            };
            package_linux_packages(
                &workspace_dir,
                &formats,
            )?;
        }
    }

    Ok(())
}
