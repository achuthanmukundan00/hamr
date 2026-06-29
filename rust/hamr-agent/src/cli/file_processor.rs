//! Process @file CLI arguments into text content and image attachments.
//!
//! Port of `packages/coding-agent/src/cli/file-processor.ts`.

use std::path::Path;

use hamr_ai::types::ImageContent;

use crate::core::tools::path_utils::resolve_read_path;
use crate::utils::image_resize::{format_dimension_note, resize_image};
use crate::utils::mime::detect_supported_image_mime_type_from_file;

/// Result of processing @file arguments.
#[derive(Debug, Default)]
pub struct ProcessedFiles {
    pub text: String,
    pub images: Vec<ImageContent>,
}

/// Options for processing @file arguments.
#[derive(Debug, Clone)]
pub struct ProcessFileOptions {
    /// Whether to auto-resize images to ~2000x2000 max. Default: true.
    pub auto_resize_images: bool,
}

impl Default for ProcessFileOptions {
    fn default() -> Self {
        Self {
            auto_resize_images: true,
        }
    }
}

/// Process @file arguments (without the @ prefix) into text content and image attachments.
///
/// Each file_arg is resolved relative to the current working directory,
/// checked for existence, and read as either text or image.
pub async fn process_file_arguments(
    file_args: &[String],
    options: Option<&ProcessFileOptions>,
) -> ProcessedFiles {
    let auto_resize_images = options.map(|o| o.auto_resize_images).unwrap_or(true);
    let mut text = String::new();
    let mut images = Vec::new();
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    for file_arg in file_args {
        // Expand and resolve path (handles ~ expansion and Unicode spaces)
        let absolute_path = resolve_read_path(file_arg, &cwd);

        // Check if file exists
        if !absolute_path.exists() {
            eprintln!(
                "\x1b[31mError: File not found: {}\x1b[0m",
                absolute_path.display()
            );
            std::process::exit(1);
        }

        // Read file metadata
        let metadata = match std::fs::metadata(&absolute_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "\x1b[31mError: Could not stat file {}: {}\x1b[0m",
                    absolute_path.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        // Skip empty files
        if metadata.len() == 0 {
            continue;
        }

        // Check for image mime type
        let mime_type = match detect_supported_image_mime_type_from_file(&absolute_path) {
            Ok(Some(mime)) => Some(mime),
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "\x1b[31mError: Could not read file {}: {}\x1b[0m",
                    absolute_path.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        let path_display = absolute_path.display().to_string();

        if let Some(mime) = mime_type {
            // Handle image file
            let content = match std::fs::read(&absolute_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "\x1b[31mError: Could not read file {}: {}\x1b[0m",
                        path_display, e
                    );
                    std::process::exit(1);
                }
            };

            let mut dimension_note: Option<String> = None;
            let attachment: ImageContent;

            if auto_resize_images {
                match resize_image(content, mime.clone(), None).await {
                    Ok(Some(resized)) => {
                        dimension_note = format_dimension_note(&resized);
                        attachment = ImageContent {
                            mime_type: resized.mime_type.clone(),
                            data: resized.data.clone(),
                        };
                    }
                    Ok(None) | Err(_) => {
                        text.push_str(&format!(
                            "<file name=\"{}\">[Image omitted: could not be resized below the inline image size limit.]</file>\n",
                            path_display
                        ));
                        continue;
                    }
                }
            } else {
                attachment = ImageContent {
                    mime_type: mime.clone(),
                    data: base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &content,
                    ),
                };
            }

            images.push(attachment);

            // Add text reference to image with optional dimension note
            if let Some(note) = dimension_note {
                text.push_str(&format!(
                    "<file name=\"{}\">{}</file>\n",
                    path_display, note
                ));
            } else {
                text.push_str(&format!("<file name=\"{}\"></file>\n", path_display));
            }
        } else {
            // Handle text file
            match std::fs::read_to_string(&absolute_path) {
                Ok(content) => {
                    text.push_str(&format!(
                        "<file name=\"{}\">\n{}\n</file>\n",
                        path_display, content
                    ));
                }
                Err(e) => {
                    eprintln!(
                        "\x1b[31mError: Could not read file {}: {}\x1b[0m",
                        path_display, e
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    ProcessedFiles { text, images }
}
