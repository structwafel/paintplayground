use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// HAHAHAHAHAAHHAAH this is great i love it

fn main() {
    println!("cargo:rerun-if-changed=public/js/");

    bundle_js_files("js", "js/bundled.js").unwrap();
}

fn bundle_js_files(js_dir: &str, output_file: &str) -> std::io::Result<()> {
    let mut combined_js = String::new();
    let dir_path = Path::new(js_dir);

    combined_js.push_str("/* Bundled JS file generated on ");
    // combined_js.push_str(&chrono::Local::now().to_string());
    combined_js.push_str(" */\n\n");

    let mut js_files = collect_js_files(dir_path, &mut Vec::new())?;
    js_files = order_js_files_by_dependencies(js_files)?;

    for js_file in js_files {
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

fn order_js_files_by_dependencies(mut files: Vec<PathBuf>) -> std::io::Result<Vec<PathBuf>> {
    // Define explicit ordering - dependencies first
    let ordering = [
        "utils.js",
        "cell.js",
        "color.js",
        "manager.js", // ChunkManager must come before files that use it
        "canvas.js",
        "grid.js",
        "ws.js",
        "stuff.js", // This uses ChunkManager, so it comes after
    ];

    // Sort files according to the defined order
    files.sort_by(|a, b| {
        let name_a = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name_b = b.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let pos_a = ordering
            .iter()
            .position(|&x| x == name_a)
            .unwrap_or(usize::MAX);
        let pos_b = ordering
            .iter()
            .position(|&x| x == name_b)
            .unwrap_or(usize::MAX);

        pos_a.cmp(&pos_b)
    });

    Ok(files)
}
