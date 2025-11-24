use byteorder::{BigEndian, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use md5::{Digest, Md5};
use memmap2::Mmap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;
use walkdir::WalkDir;

const BLOCK_SIZE: usize = 65536; // 64KB

#[derive(Debug)]
pub struct PackingStatus {
    pub current_file: String,
    pub progress: f32,
    pub is_packing: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct ZSize {
    size: u16, // Compressed size (0 means uncompressed/same size as block)
}

struct Entry {
    name_hash: [u8; 16],
    zsize_index: u32,
    uncompressed_size: u64,
    offset: u64,
}

#[allow(dead_code)]
struct CompressedBlock {
    // We might need these if we were sorting, but for now we process sequentially per file
    // file_index: usize,
    // block_index: usize,
    // data: Vec<u8>,
    // original_size: usize,
}

struct ProcessedFile {
    file_idx: usize,
    compressed_data: Vec<u8>,
    zsizes: Vec<ZSize>,
    entry: Entry,
}

pub fn pack_directory<F>(
    root_path: &Path,
    output_path: &Path,
    progress_callback: F,
) -> io::Result<()>
where
    F: Fn(PackingStatus) + Send + Sync + 'static,
{
    let root_path = root_path.to_path_buf();
    let output_path = output_path.to_path_buf();
    let start_time = Instant::now();

    // Run in a separate thread to avoid blocking UI
    thread::spawn(move || {
        let result = pack_directory_internal(&root_path, &output_path, &progress_callback);
        let elapsed_ms = start_time.elapsed().as_millis();

        match result {
            Err(e) => {
                eprintln!("[PSARC] Packing failed after {} ms: {}", elapsed_ms, e);
                progress_callback(PackingStatus {
                    current_file: "Error".to_string(),
                    progress: 0.0,
                    is_packing: false,
                    error: Some(e.to_string()),
                });
            }
            Ok(()) => {
                eprintln!("[PSARC] Packing completed successfully in {} ms", elapsed_ms);
                progress_callback(PackingStatus {
                    current_file: "Done".to_string(),
                    progress: 1.0,
                    is_packing: false,
                    error: None,
                });
            }
        }
    });

    Ok(())
}

fn pack_directory_internal<F>(
    root_path: &Path,
    output_path: &Path,
    progress_callback: &F,
) -> io::Result<()>
where
    F: Fn(PackingStatus),
{
    // Phase 1: Scan Directory
    progress_callback(PackingStatus {
        current_file: "Scanning directory...".to_string(),
        progress: 0.0,
        is_packing: true,
        error: None,
    });

    let mut discovered_files = Vec::new();
    let mut manifest_bytes_on_disk: Option<Vec<u8>> = None;

    for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(root_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                .to_string_lossy()
                .replace('\\', "/"); // PSARC uses forward slashes

            // Skip if output file is inside the source directory to avoid recursion loop
            if path == output_path {
                continue;
            }

            // Add leading slash if not present (PSARC convention usually starts with /)
            // UnPSARC logic: relative path, forward slashes, NO leading slash.
            let psarc_path = relative_path.clone();

            let name_lower = psarc_path.to_ascii_lowercase();
            let is_filenames = name_lower == "filenames.txt";
            let is_filelist = name_lower == "filelist.xml" || name_lower == "filelist.txt";
            if is_filenames || is_filelist {
                if is_filelist || manifest_bytes_on_disk.is_none() {
                    manifest_bytes_on_disk = Some(std::fs::read(path)?);
                }
                continue;
            }

            discovered_files.push((path.to_path_buf(), psarc_path));
        }
    }

    let (files, filenames_bytes) = resolve_file_order(discovered_files, manifest_bytes_on_disk)?;

    // Pre-calculate MD5 hashes for all files in parallel to avoid duplicate calculations
    let file_hashes: Vec<[u8; 16]> = files
        .par_iter()
        .map(|(_, psarc_path)| calculate_md5(psarc_path))
        .collect();

    // Create temp file for compressed data
    let mut temp_data_file = tempfile::tempfile()?;
    let total_files = files.len() + 1; // +1 for Filenames.txt

    // Phase 2: Parallel Compression
    // We treat Filenames.txt as file index 0
    // We won't use the separate writer thread complexity for now,
    // instead we'll do the "Sequential Files, Parallel Blocks" approach in the main thread.

    let mut zsizes: Vec<ZSize> = Vec::new();
    let mut entries: Vec<Entry> = Vec::with_capacity(total_files);
    let mut current_offset = 0u64;

    // Use larger buffer for better I/O performance (1MB instead of default 8KB)
    let mut writer = BufWriter::with_capacity(1024 * 1024, &mut temp_data_file);

    // 1. Process Filenames.txt
    {
        let uncompressed_size = filenames_bytes.len() as u64;
        let zsize_start_index = zsizes.len() as u32;

        // Chunkify
        let chunks: Vec<&[u8]> = filenames_bytes.chunks(BLOCK_SIZE).collect();

        // Parallel Compress
        let compressed_chunks: Vec<Vec<u8>> = chunks
            .par_iter()
            .map(|chunk| compress_block(chunk))
            .collect();

        for (i, compressed) in compressed_chunks.iter().enumerate() {
            let size = compressed.len();
            let is_compressed = size < chunks[i].len(); // Only use compressed if smaller

            let final_data = if is_compressed {
                compressed.as_slice()
            } else {
                chunks[i]
            };

            let zsize = if is_compressed { size as u16 } else { 0 }; // 0 means raw
            zsizes.push(ZSize { size: zsize });

            writer.write_all(final_data)?;
            current_offset += final_data.len() as u64;
        }

        // Manifest has no name hash (special entry)
        entries.push(Entry {
            name_hash: [0; 16],
            zsize_index: zsize_start_index,
            uncompressed_size,
            offset: 0, // Will fix up later relative to start of data
        });
    }

    // 2. Process Real Files in Parallel
    let total_files_count = files.len();
    
    // Process files in parallel, but collect results to maintain order
    // Note: We can't call progress_callback in parallel context, so we'll update after collection
    let processed_files: Result<Vec<ProcessedFile>, io::Error> = files
        .par_iter()
        .enumerate()
        .map(|(file_idx, (sys_path, _psarc_path))| {

            let file = File::open(sys_path)?;
            let len = file.metadata()?.len();

            if len == 0 {
                return Ok(ProcessedFile {
                    file_idx,
                    compressed_data: Vec::new(),
                    zsizes: Vec::new(),
                    entry: Entry {
                        name_hash: file_hashes[file_idx],
                        zsize_index: 0, // Will be set later
                        uncompressed_size: 0,
                        offset: 0, // Will be set later
                    },
                });
            }

            // Mmap for efficiency on large files
            // SAFETY: We assume the file is not modified while we read it.
            #[allow(unsafe_code)]
            let mmap = unsafe { Mmap::map(&file)? };
            let chunks: Vec<&[u8]> = mmap.chunks(BLOCK_SIZE).collect();

            // Parallel Compress blocks
            let compressed_chunks: Vec<Vec<u8>> = chunks
                .par_iter()
                .map(|chunk| compress_block(chunk))
                .collect();

            let mut file_zsizes = Vec::new();
            let mut file_data = Vec::new();

            for (i, compressed) in compressed_chunks.iter().enumerate() {
                let size = compressed.len();
                let original_len = chunks[i].len();
                let is_worth_compressing = size < original_len;

                let final_data = if is_worth_compressing {
                    compressed.as_slice()
                } else {
                    chunks[i]
                };

                let stored_size = final_data.len();

                // Determine ZSize value
                let zsize_val = if !is_worth_compressing {
                    if original_len == BLOCK_SIZE {
                        0 // Special case for full raw block
                    } else {
                        original_len as u16 // Partial raw block
                    }
                } else {
                    stored_size as u16
                };

                file_zsizes.push(ZSize { size: zsize_val });
                file_data.extend_from_slice(final_data);
            }

            Ok(ProcessedFile {
                file_idx,
                compressed_data: file_data,
                zsizes: file_zsizes,
                entry: Entry {
                    name_hash: file_hashes[file_idx],
                    zsize_index: 0, // Will be set later
                    uncompressed_size: len,
                    offset: 0, // Will be set later
                },
            })
        })
        .collect();

    let mut processed_files = processed_files?;

    // Sort by file_idx to maintain order
    processed_files.sort_by_key(|f| f.file_idx);

    // Write processed files in order and build entries/zsizes
    let progress_update_interval = (total_files_count / 100).max(1).min(10);
    for (idx, processed) in processed_files.into_iter().enumerate() {
        // Update progress during sequential write phase
        if idx % progress_update_interval == 0 || idx == total_files_count - 1 {
            if let Some((_, psarc_path)) = files.get(processed.file_idx) {
                progress_callback(PackingStatus {
                    current_file: psarc_path.clone(),
                    progress: (idx as f32) / (total_files_count as f32),
                    is_packing: true,
                    error: None,
                });
            }
        }
        let zsize_start_index = zsizes.len() as u32;
        let start_offset = current_offset;

        // Add zsizes for this file
        zsizes.extend(processed.zsizes);

        // Write compressed data
        writer.write_all(&processed.compressed_data)?;
        current_offset += processed.compressed_data.len() as u64;

        // Create entry with correct offsets
        entries.push(Entry {
            name_hash: processed.entry.name_hash,
            zsize_index: zsize_start_index,
            uncompressed_size: processed.entry.uncompressed_size,
            offset: start_offset,
        });
    }

    // Final progress update
    progress_callback(PackingStatus {
        current_file: "Writing...".to_string(),
        progress: 1.0,
        is_packing: true,
        error: None,
    });

    writer.flush()?;
    drop(writer); // Release borrow on temp_data_file

    // Phase 3: Write Final Output
    // Use larger buffer for better I/O performance
    let mut output = BufWriter::with_capacity(1024 * 1024, File::create(output_path)?);

    // --- Header ---
    output.write_all(b"PSAR")?;
    output.write_u16::<BigEndian>(1)?; // Major
    output.write_u16::<BigEndian>(4)?; // Minor
    output.write_all(b"zlib")?;

    // TOC Length calculation
    // Header (32) + Entries (30 * count) + ZSizes (2 * count)
    // But wait, spec says: "Includes 32 byte header length + block length table following ToC"
    // So TOC_Length = 32 + (Entries.len * 30) + (ZSizes.len * 2)
    let toc_entries_size = entries.len() * 30;
    let zsizes_size = zsizes.len() * 2;
    let toc_length = 32 + toc_entries_size + zsizes_size;

    output.write_u32::<BigEndian>(toc_length as u32)?;
    output.write_u32::<BigEndian>(30)?; // Entry Size
    output.write_u32::<BigEndian>(entries.len() as u32)?; // Files Count
    output.write_u32::<BigEndian>(BLOCK_SIZE as u32)?;
    output.write_u32::<BigEndian>(1)?; // Flags: 1 = ignorecase

    // --- TOC Entries ---
    for entry in &entries {
        output.write_all(&entry.name_hash)?;
        output.write_u32::<BigEndian>(entry.zsize_index)?;

        // 40-bit Uncompressed Size
        output.write_u8((entry.uncompressed_size >> 32) as u8)?;
        output.write_u32::<BigEndian>(entry.uncompressed_size as u32)?;

        // 40-bit Offset
        // The offset in Entry is relative to the start of the file? Or start of Data?
        // Spec: "Byte offset in psarc for this entry."
        // So it's Absolute Offset.
        // Our `entry.offset` is relative to start of Data.
        // Data starts after TOC and ZSizes.
        let absolute_offset = entry.offset + toc_length as u64;

        output.write_u8((absolute_offset >> 32) as u8)?;
        output.write_u32::<BigEndian>(absolute_offset as u32)?;
    }

    // --- ZSizes Table ---
    for zsize in &zsizes {
        output.write_u16::<BigEndian>(zsize.size)?;
    }

    // --- Data ---
    temp_data_file.seek(SeekFrom::Start(0))?;
    io::copy(&mut temp_data_file, &mut output)?;

    output.flush()?;

    Ok(())
}

fn resolve_file_order(
    discovered_files: Vec<(PathBuf, String)>,
    manifest_bytes_on_disk: Option<Vec<u8>>,
) -> io::Result<(Vec<(PathBuf, String)>, Vec<u8>)> {
    if let Some(bytes) = manifest_bytes_on_disk {
        if let Ok(text) = String::from_utf8(bytes) {
            let manifest_paths = normalize_manifest_lines(&text);

            if !manifest_paths.is_empty() {
                let mut path_map: HashMap<String, PathBuf> = discovered_files
                    .iter()
                    .map(|(path_buf, psarc_path)| (psarc_path.clone(), path_buf.clone()))
                    .collect();

                let mut ordered = Vec::with_capacity(manifest_paths.len());
                let mut missing = Vec::new();

                for path in &manifest_paths {
                    if let Some(real_path) = path_map.remove(path) {
                        ordered.push((real_path, path.clone()));
                    } else {
                        missing.push(path.clone());
                    }
                }

                if missing.is_empty() && path_map.is_empty() {
                    let normalized_bytes = manifest_bytes_from_paths(&manifest_paths);
                    return Ok((ordered, normalized_bytes));
                }

                if !missing.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("File list references missing files: {}", missing.join(", ")),
                    ));
                }
                // If there are extra files on disk beyond the manifest, fall back to regenerating it.
            }
        }
    }

    let mut files = discovered_files;
    files.sort_by(|a, b| {
        let md5_a = calculate_md5(&a.1);
        let md5_b = calculate_md5(&b.1);
        md5_a.cmp(&md5_b)
    });

    let mut manifest_content = String::new();
    for (_, psarc_path) in &files {
        manifest_content.push_str(psarc_path);
        manifest_content.push('\n');
    }

    Ok((files, manifest_content.into_bytes()))
}

fn normalize_manifest_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_start_matches('\u{feff}');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.replace('\\', "/"))
            }
        })
        .collect()
}

fn manifest_bytes_from_paths(paths: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for path in paths {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(b'\n');
    }
    bytes
}

fn compress_block(data: &[u8]) -> Vec<u8> {
    // Use default compression level for better speed/ratio balance
    // best() is too slow, default() provides good compression with better speed
    let mut encoder = ZlibEncoder::new(Vec::with_capacity(data.len()), Compression::best());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn calculate_md5(path: &str) -> [u8; 16] {
    // PSARC hashes uppercase paths, otherwise the entry order and hash values won't
    // match the original manifest and the archive becomes unreadable by the game.
    // Optimize: check if already uppercase to avoid allocation
    let mut hasher = Md5::new();
    if path.chars().all(|c| !c.is_ascii_lowercase()) {
        // Already uppercase or no lowercase chars, use directly
        hasher.update(path.as_bytes());
    } else {
        // Need to uppercase
        hasher.update(path.to_ascii_uppercase().as_bytes());
    }
    hasher.finalize().into()
}
