//! Extension-based file classification — metadata only, never reads content.

use lso_core::FileCategory;
use std::path::Path;

/// Classify a file by its extension and name (metadata only).
pub fn classify_by_extension(path: &Path) -> FileCategory {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => return classify_by_name(path),
    };

    match ext.as_str() {
        // Documents
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "tex" | "pages" | "epub"
        | "mobi" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp" => FileCategory::Documents,

        // Media — images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "svg" | "webp" | "ico"
        | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw" | "psd" | "ai" | "xcf" => {
            FileCategory::Media
        }

        // Media — video
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
        | "3gp" | "ogv" => FileCategory::Media,

        // Media — audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" | "alac" => {
            FileCategory::Media
        }

        // Code
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "java"
        | "go" | "rb" | "php" | "swift" | "kt" | "scala" | "cs" | "lua" | "sh" | "bash"
        | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "r" | "jl" | "hs" | "ml" | "ex"
        | "exs" | "erl" | "clj" | "vim" | "el" | "zig" | "nim" | "v" | "dart" | "sql" => {
            FileCategory::Code
        }

        // Code — config/build files
        "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "cmake" | "makefile"
        | "dockerfile" | "lock" => FileCategory::Code,

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz4" | "lzma"
        | "cab" | "iso" | "dmg" | "pkg" | "deb" | "rpm" | "apk" | "appimage" | "snap"
        | "flatpak" | "msi" | "exe" => FileCategory::Archives,

        // Data
        "csv" | "json" | "jsonl" | "xml" | "parquet" | "avro" | "sqlite" | "db" | "sqlite3"
        | "hdf5" | "h5" | "feather" | "arrow" | "npy" | "npz" | "pickle" | "pkl" | "tsv" => {
            FileCategory::Data
        }

        // Temporary / cache
        "tmp" | "temp" | "bak" | "swp" | "swo" | "pyc" | "pyo" | "o" | "obj" | "class"
        | "cache" | "log" => FileCategory::Temporary,

        _ => FileCategory::Unknown,
    }
}

fn classify_by_name(path: &Path) -> FileCategory {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_ascii_lowercase(),
        None => return FileCategory::Unknown,
    };

    match name.as_str() {
        "makefile" | "dockerfile" | "rakefile" | "gemfile" | "procfile" | "vagrantfile"
        | "justfile" | "cmakelists.txt" | ".gitignore" | ".gitattributes" | ".editorconfig"
        | ".env" | ".envrc" => FileCategory::Code,

        "readme" | "license" | "licence" | "changelog" | "authors" | "contributing"
        | "copying" | "notice" => FileCategory::Documents,

        "thumbs.db" | ".ds_store" | "desktop.ini" => FileCategory::Temporary,

        _ if name.starts_with('.') && name.ends_with("rc") => FileCategory::Code,
        _ if name.starts_with(".bash") || name.starts_with(".zsh") => FileCategory::Code,
        _ => FileCategory::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_documents() {
        assert_eq!(classify_by_extension(&PathBuf::from("report.pdf")), FileCategory::Documents);
        assert_eq!(classify_by_extension(&PathBuf::from("notes.md")), FileCategory::Documents);
    }

    #[test]
    fn classifies_media() {
        assert_eq!(classify_by_extension(&PathBuf::from("photo.jpg")), FileCategory::Media);
        assert_eq!(classify_by_extension(&PathBuf::from("song.mp3")), FileCategory::Media);
        assert_eq!(classify_by_extension(&PathBuf::from("clip.mp4")), FileCategory::Media);
    }

    #[test]
    fn classifies_code() {
        assert_eq!(classify_by_extension(&PathBuf::from("main.rs")), FileCategory::Code);
        assert_eq!(classify_by_extension(&PathBuf::from("config.toml")), FileCategory::Code);
    }

    #[test]
    fn classifies_archives() {
        assert_eq!(classify_by_extension(&PathBuf::from("backup.tar.gz")), FileCategory::Archives);
        assert_eq!(classify_by_extension(&PathBuf::from("package.zip")), FileCategory::Archives);
    }

    #[test]
    fn classifies_data() {
        assert_eq!(classify_by_extension(&PathBuf::from("data.csv")), FileCategory::Data);
        assert_eq!(classify_by_extension(&PathBuf::from("store.sqlite")), FileCategory::Data);
    }

    #[test]
    fn classifies_temporary() {
        assert_eq!(classify_by_extension(&PathBuf::from("file.tmp")), FileCategory::Temporary);
        assert_eq!(classify_by_extension(&PathBuf::from("app.log")), FileCategory::Temporary);
    }

    #[test]
    fn classifies_by_name_when_no_extension() {
        assert_eq!(classify_by_extension(&PathBuf::from("Makefile")), FileCategory::Code);
        assert_eq!(classify_by_extension(&PathBuf::from("README")), FileCategory::Documents);
        assert_eq!(classify_by_extension(&PathBuf::from(".DS_Store")), FileCategory::Temporary);
    }

    #[test]
    fn unknown_for_unrecognized() {
        assert_eq!(classify_by_extension(&PathBuf::from("mystery.xyz123")), FileCategory::Unknown);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(classify_by_extension(&PathBuf::from("IMAGE.JPG")), FileCategory::Media);
        assert_eq!(classify_by_extension(&PathBuf::from("CODE.RS")), FileCategory::Code);
    }
}
