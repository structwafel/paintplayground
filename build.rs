use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// HAHAHAHAHAAHHAAH this is great i love it

fn main() {
    println!("cargo:rerun-if-changed=public/js/");

    bundle_js_files("js", "js/bundled.js").unwrap();

    update_html_script_tag().unwrap();
}

fn bundle_js_files(js_dir: &str, output_file: &str) -> std::io::Result<()> {
    let mut combined_js = String::new();
    let dir_path = Path::new(js_dir);

    combined_js.push_str("/* Bundled JS file generated on ");
    // combined_js.push_str(&chrono::Local::now().to_string());
    combined_js.push_str(" */\n\n");

    collect_js_files(dir_path, &mut Vec::new())?.sort();

    for js_file in collect_js_files(dir_path, &mut Vec::new())? {
        if js_file.to_string_lossy() == output_file {
            continue;
        }

        let mut content = String::new();
        File::open(&js_file)?.read_to_string(&mut content)?;

        // some headers
        combined_js.push_str("/* File: ");
        combined_js.push_str(&js_file.to_string_lossy());
        combined_js.push_str(" */\n");

        // filter out imports HAHAHAHAHAHAHHHAHAHHA
        let filtered_content: String = content
            .lines()
            .into_iter()
            .filter(|line| !line.starts_with("import { "))
            .collect::<Vec<&str>>()
            .join("\n");

        combined_js.push_str(&filtered_content);

        combined_js.push_str("\n\n");
    }

    // Write bundled file
    let mut output = File::create(output_file)?;
    output.write_all(combined_js.as_bytes())?;

    println!(
        "cargo:warning=JavaScript files bundled into {}",
        output_file
    );
    Ok(())
}

fn collect_js_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<Vec<PathBuf>> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                collect_js_files(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "js") {
                files.push(path);
            }
        }
    }

    Ok(files.to_vec())
}

fn update_html_script_tag() -> std::io::Result<()> {
    let html_path = "public/index.html";
    let html_content = fs::read_to_string(html_path)?;

    // Only update if the bundle script tag doesn't already exist
    if !html_content.contains("src=\"./js/bundled.js\"") {
        let updated_content = html_content.replace(
            "<script type=\"module\" src=\"./js/stuff.js\"></script>",
            "<script type=\"module\" src=\"./js/bundled.js\"></script>",
        );

        fs::write(html_path, updated_content)?;
        println!("cargo:warning=Updated HTML to use bundled JavaScript");
    }

    Ok(())
}
