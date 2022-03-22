use std::fs::{File};
use std::io::{Read, Write};
use std::path::Path;
use apk_editor::apk_zip;
use apk_editor::manifest::manifest_editor::{AndroidManifest, Provider};

const APK_PATH: &str = "app-release-unsigned.apk";

fn main() {
    let mut f = File::open(APK_PATH).unwrap();
    let mut out = File::create("./out/g.apk").unwrap();
    let mut data = Vec::new();
    f.read_to_end(&mut data).unwrap();
    println!("zip len: {}", data.len());
    let mut zip_file = apk_zip::ApkFile::from(&data).unwrap();
    let manifest = zip_file.get_manifest();
    println!("manifest len: {}", manifest.len());
    let mut fest = AndroidManifest::from(&manifest).unwrap();
    fest.add_content_provider(Provider{
        class_name: "io.github.yearsyan.hookme.Prov".to_string(),
        authorities: "io.github.yearsyan.hookme.Provider".to_string()
    });
    let new_manifest = fest.get_data();
    let ext_file = Vec::from("hello test");

    zip_file.set_manifest(&new_manifest);
    zip_file.add_assets("ext.txt", &ext_file);
    zip_file.save(&mut out).unwrap();
    println!("edit done");
}
