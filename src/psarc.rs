use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::ZlibDecoder;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use md5::{Digest, Md5};
use memmap2::Mmap;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;
use walkdir::WalkDir;

const BLOCK_SIZE: usize = 65536; // 64KB

/// Packing mode for PSARC creation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingMode {
    Full,        // Repack everything from scratch
    Incremental, // Only recompress modified files, reuse cached data for unchanged files
}

#[derive(Debug, Clone)]
pub struct PackingStatus {
    pub current_file: String,
    pub progress: f32,
    pub is_packing: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractionStatus {
    pub current_file: String,
    pub progress: f32,
    pub is_extracting: bool,
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
    compression: Compression,
    packing_mode: PackingMode,
    modified_files: HashSet<PathBuf>,
    existing_psarc: Option<PathBuf>,
    progress_callback: F,
) -> io::Result<()>
where
    F: Fn(PackingStatus) + Send + Sync + 'static,
{
    let root_path = root_path.to_path_buf();
    let output_path = output_path.to_path_buf();
    let start_time = Instant::now();
    let mode_str = match packing_mode {
        PackingMode::Full => "Full",
        PackingMode::Incremental => "Incremental",
    };

    // Run in a separate thread to avoid blocking UI
    thread::spawn(move || {
        let result = pack_directory_internal(
            &root_path,
            &output_path,
            compression,
            packing_mode,
            &modified_files,
            existing_psarc.as_deref(),
            &progress_callback,
        );
        let elapsed_ms = start_time.elapsed().as_millis();

        match result {
            Err(e) => {
                eprintln!("[PSARC] Packing failed (mode: {}) after {} ms: {}", mode_str, elapsed_ms, e);
                progress_callback(PackingStatus {
                    current_file: "Error".to_string(),
                    progress: 0.0,
                    is_packing: false,
                    error: Some(e.to_string()),
                });
            }
            Ok((recompressed, reused)) => {
                eprintln!(
                    "[PSARC] Packing completed (mode: {}) in {} ms - {} files recompressed, {} files reused from cache",
                    mode_str, elapsed_ms, recompressed, reused
                );
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

/// Cached file data from an existing PSARC for incremental packing
struct CachedFileData {
    compressed_data: Vec<u8>,
    zsizes: Vec<ZSize>,
    uncompressed_size: u64,
}

/// Read cached compressed data for a specific entry from an existing PSARC
fn read_cached_file_data(
    mmap: &Mmap,
    entry: &Entry,
    zsizes: &[u16],
    block_size: usize,
) -> io::Result<CachedFileData> {
    let mut result_data = Vec::new();
    let mut result_zsizes = Vec::new();
    let mut current_zsize_index = entry.zsize_index as usize;
    let mut current_offset = entry.offset as usize;
    let mut remaining = entry.uncompressed_size as usize;

    while remaining > 0 {
        let zsize = zsizes.get(current_zsize_index)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid zsize index {} (max: {})", current_zsize_index, zsizes.len()),
            ))?;

        let compressed_size = if *zsize == 0 {
            block_size
        } else {
            *zsize as usize
        };

        if current_offset + compressed_size > mmap.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Block read would exceed file bounds: offset {}, size {}, file size {}",
                    current_offset, compressed_size, mmap.len()
                ),
            ));
        }

        // Copy raw compressed data (don't decompress!)
        result_data.extend_from_slice(&mmap[current_offset..current_offset + compressed_size]);
        result_zsizes.push(ZSize { size: *zsize });

        current_offset += compressed_size;
        remaining = remaining.saturating_sub(block_size);
        current_zsize_index += 1;
    }

    Ok(CachedFileData {
        compressed_data: result_data,
        zsizes: result_zsizes,
        uncompressed_size: entry.uncompressed_size,
    })
}

/// Load cache from existing PSARC file for incremental packing
fn load_psarc_cache(psarc_path: &Path) -> io::Result<HashMap<[u8; 16], CachedFileData>> {
    let file = File::open(psarc_path)?;
    #[allow(unsafe_code)]
    let mmap = unsafe { Mmap::map(&file)? };
    let mut reader = io::Cursor::new(&mmap[..]);

    // Read header
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"PSAR" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid PSARC magic number",
        ));
    }

    let _major = reader.read_u16::<BigEndian>()?;
    let _minor = reader.read_u16::<BigEndian>()?;
    let mut compression = [0u8; 4];
    reader.read_exact(&mut compression)?;
    
    let toc_length = reader.read_u32::<BigEndian>()?;
    let _entry_size = reader.read_u32::<BigEndian>()?;
    let file_count = reader.read_u32::<BigEndian>()?;
    let block_size = reader.read_u32::<BigEndian>()?;
    let _flags = reader.read_u32::<BigEndian>()?;

    // Read TOC entries
    let mut entries: Vec<Entry> = Vec::with_capacity(file_count as usize);
    for _ in 0..file_count {
        let mut name_hash = [0u8; 16];
        reader.read_exact(&mut name_hash)?;
        let zsize_index = reader.read_u32::<BigEndian>()?;

        let uncompressed_size_high = reader.read_u8()?;
        let uncompressed_size_low = reader.read_u32::<BigEndian>()?;
        let uncompressed_size = ((uncompressed_size_high as u64) << 32) | (uncompressed_size_low as u64);

        let offset_high = reader.read_u8()?;
        let offset_low = reader.read_u32::<BigEndian>()?;
        let offset = ((offset_high as u64) << 32) | (offset_low as u64);

        entries.push(Entry {
            name_hash,
            zsize_index,
            uncompressed_size,
            offset,
        });
    }

    // Read ZSizes table
    let zsizes_start = reader.position() as usize;
    let zsizes_count = (toc_length as usize - 32 - (file_count as usize * 30)) / 2;
    let zsizes: Vec<u16> = (0..zsizes_count)
        .map(|i| {
            let pos = zsizes_start + (i * 2);
            u16::from_be_bytes([mmap[pos], mmap[pos + 1]])
        })
        .collect();

    // Build cache map
    let mut cache = HashMap::new();
    for entry in &entries {
        // Skip manifest entry (all zeros hash)
        if entry.name_hash == [0; 16] {
            continue;
        }
        
        if let Ok(cached_data) = read_cached_file_data(&mmap, entry, &zsizes, block_size as usize) {
            cache.insert(entry.name_hash, cached_data);
        }
    }

    eprintln!("[PSARC] Loaded cache with {} entries from existing archive", cache.len());
    Ok(cache)
}

/// Returns (recompressed_count, reused_count)
fn pack_directory_internal<F>(
    root_path: &Path,
    output_path: &Path,
    compression: Compression,
    packing_mode: PackingMode,
    modified_files: &HashSet<PathBuf>,
    existing_psarc: Option<&Path>,
    progress_callback: &F,
) -> io::Result<(usize, usize)>
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
            let is_filelist = name_lower == "filelist.xml";
            if is_filelist {
                if manifest_bytes_on_disk.is_none() {
                    manifest_bytes_on_disk = Some(std::fs::read(path)?);
                }
                continue;
            }

            discovered_files.push((path.to_path_buf(), psarc_path));
        }
    }

    let (files, filelist_bytes) = resolve_file_order(discovered_files, manifest_bytes_on_disk)?;

    // Pre-calculate MD5 hashes for all files in parallel to avoid duplicate calculations
    let file_hashes: Vec<[u8; 16]> = files
        .par_iter()
        .map(|(_, psarc_path)| calculate_md5(psarc_path))
        .collect();

    // Load cache from existing PSARC if in incremental mode
    let cache: HashMap<[u8; 16], CachedFileData> = if packing_mode == PackingMode::Incremental {
        if let Some(psarc_path) = existing_psarc {
            if psarc_path.exists() {
                progress_callback(PackingStatus {
                    current_file: "Loading cache from existing PSARC...".to_string(),
                    progress: 0.0,
                    is_packing: true,
                    error: None,
                });
                load_psarc_cache(psarc_path).unwrap_or_else(|e| {
                    eprintln!("[PSARC] Warning: Failed to load cache: {}", e);
                    HashMap::new()
                })
            } else {
                eprintln!("[PSARC] No existing PSARC found, will compress all files");
                HashMap::new()
            }
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // Create temp file for compressed data
    let mut temp_data_file = tempfile::tempfile()?;
    let total_files = files.len() + 1; // +1 for FileList.xml

    // Phase 2: Parallel Compression
    // We treat FileList.xml as file index 0
    // We won't use the separate writer thread complexity for now,
    // instead we'll do the "Sequential Files, Parallel Blocks" approach in the main thread.

    let mut zsizes: Vec<ZSize> = Vec::new();
    let mut entries: Vec<Entry> = Vec::with_capacity(total_files);
    let mut current_offset = 0u64;

    // Use larger buffer for better I/O performance (1MB instead of default 8KB)
    let mut writer = BufWriter::with_capacity(1024 * 1024, &mut temp_data_file);

    // 1. Process FileList.xml (always recompress)
    {
        let uncompressed_size = filelist_bytes.len() as u64;
        let zsize_start_index = zsizes.len() as u32;

        // Chunkify
        let chunks: Vec<&[u8]> = filelist_bytes.chunks(BLOCK_SIZE).collect();

        // Parallel Compress
        let compressed_chunks: Vec<Vec<u8>> = chunks
            .par_iter()
            .map(|chunk| compress_block(chunk, compression))
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

    // 2. Process Real Files
    let total_files_count = files.len();
    
    // Convert modified_files to a set of normalized paths for comparison
    let modified_set: HashSet<String> = modified_files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();

    // Track statistics
    let mut recompressed_count = 0usize;
    let mut reused_count = 0usize;
    
    // Process files - determine which need recompression vs cache reuse
    let processed_files: Result<Vec<ProcessedFile>, io::Error> = files
        .par_iter()
        .enumerate()
        .map(|(file_idx, (sys_path, psarc_path))| {
            let name_hash = file_hashes[file_idx];
            
            // Check if this file should use cached data
            let should_recompress = packing_mode == PackingMode::Full 
                || modified_set.contains(psarc_path)
                || !cache.contains_key(&name_hash);

            if !should_recompress {
                // Use cached data
                if let Some(cached) = cache.get(&name_hash) {
                    return Ok(ProcessedFile {
                        file_idx,
                        compressed_data: cached.compressed_data.clone(),
                        zsizes: cached.zsizes.clone(),
                        entry: Entry {
                            name_hash,
                            zsize_index: 0, // Will be set later
                            uncompressed_size: cached.uncompressed_size,
                            offset: 0, // Will be set later
                        },
                    });
                }
            }

            // Need to recompress this file
            let file = File::open(sys_path)?;
            let len = file.metadata()?.len();

            if len == 0 {
                return Ok(ProcessedFile {
                    file_idx,
                    compressed_data: Vec::new(),
                    zsizes: Vec::new(),
                    entry: Entry {
                        name_hash,
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
                .map(|chunk| compress_block(chunk, compression))
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
                    name_hash,
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
        let psarc_path = &files[processed.file_idx].1;
        let name_hash = processed.entry.name_hash;
        
        // Track if this file was reused from cache
        let was_reused = packing_mode == PackingMode::Incremental 
            && !modified_set.contains(psarc_path)
            && cache.contains_key(&name_hash);
        
        if was_reused {
            reused_count += 1;
        } else {
            recompressed_count += 1;
        }

        // Update progress during sequential write phase
        if idx % progress_update_interval == 0 || idx == total_files_count - 1 {
            let status = if was_reused { "cached" } else { "compressing" };
            progress_callback(PackingStatus {
                current_file: format!("[{}] {}", status, psarc_path),
                progress: (idx as f32) / (total_files_count as f32),
                is_packing: true,
                error: None,
            });
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

    Ok((recompressed_count, reused_count))
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
    for (i, (_, psarc_path)) in files.iter().enumerate() {
        manifest_content.push_str(psarc_path);
        // Don't add newline after the last file (PSARC format doesn't have trailing newline)
        if i < files.len() - 1 {
            manifest_content.push('\n');
        }
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
    for (i, path) in paths.iter().enumerate() {
        bytes.extend_from_slice(path.as_bytes());
        // Don't add newline after the last file (PSARC format doesn't have trailing newline)
        if i < paths.len() - 1 {
            bytes.push(b'\n');
        }
    }
    bytes
}

fn compress_block(data: &[u8], compression: Compression) -> Vec<u8> {
    // Use default compression level for better speed/ratio balance
    // best() is too slow, default() provides good compression with better speed
    let mut encoder = ZlibEncoder::new(Vec::with_capacity(data.len()), compression);
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

fn hash_to_string(hash: &[u8; 16]) -> String {
    // Format hash as "AA-BB-CC-DD-..." (BitConverter.ToString format used by UnPSARC)
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join("-")
}

pub fn extract_psarc<F>(
    psarc_path: &Path,
    output_dir: &Path,
    progress_callback: F,
) -> io::Result<()>
where
    F: Fn(ExtractionStatus) + Send + Sync + 'static,
{
    let psarc_path = psarc_path.to_path_buf();
    let output_dir = output_dir.to_path_buf();
    let start_time = Instant::now();

    thread::spawn(move || {
        let result = extract_psarc_internal(&psarc_path, &output_dir, &progress_callback);
        let elapsed_ms = start_time.elapsed().as_millis();

        match result {
            Err(e) => {
                eprintln!("[PSARC] Extraction failed after {} ms: {}", elapsed_ms, e);
                progress_callback(ExtractionStatus {
                    current_file: "Error".to_string(),
                    progress: 0.0,
                    is_extracting: false,
                    error: Some(e.to_string()),
                });
            }
            Ok(()) => {
                eprintln!("[PSARC] Extraction completed successfully in {} ms", elapsed_ms);
                progress_callback(ExtractionStatus {
                    current_file: "Done".to_string(),
                    progress: 1.0,
                    is_extracting: false,
                    error: None,
                });
            }
        }
    });

    Ok(())
}

fn extract_psarc_internal<F>(
    psarc_path: &Path,
    output_dir: &Path,
    progress_callback: &F,
) -> io::Result<()>
where
    F: Fn(ExtractionStatus),
{
    progress_callback(ExtractionStatus {
        current_file: "Reading PSARC file...".to_string(),
        progress: 0.0,
        is_extracting: true,
        error: None,
    });

    let file = File::open(psarc_path)?;
    #[allow(unsafe_code)]
    let mmap = unsafe { Mmap::map(&file)? };
    let mut reader = io::Cursor::new(&mmap[..]);

    // Read header
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"PSAR" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid PSARC magic number",
        ));
    }

    let _major = reader.read_u16::<BigEndian>()?;
    let _minor = reader.read_u16::<BigEndian>()?;
    let mut compression = [0u8; 4];
    reader.read_exact(&mut compression)?;
    if &compression != b"zlib" {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Unsupported compression: {:?}", compression),
        ));
    }

    let toc_length = reader.read_u32::<BigEndian>()?;
    let _entry_size = reader.read_u32::<BigEndian>()?;
    let file_count = reader.read_u32::<BigEndian>()?;
    let block_size = reader.read_u32::<BigEndian>()?;
    let _flags = reader.read_u32::<BigEndian>()?;

    // Read TOC entries
    let mut entries: Vec<Entry> = Vec::with_capacity(file_count as usize);
    for _ in 0..file_count {
        let mut name_hash = [0u8; 16];
        reader.read_exact(&mut name_hash)?;
        let zsize_index = reader.read_u32::<BigEndian>()?;

        let uncompressed_size_high = reader.read_u8()?;
        let uncompressed_size_low = reader.read_u32::<BigEndian>()?;
        let uncompressed_size = ((uncompressed_size_high as u64) << 32) | (uncompressed_size_low as u64);

        let offset_high = reader.read_u8()?;
        let offset_low = reader.read_u32::<BigEndian>()?;
        let offset = ((offset_high as u64) << 32) | (offset_low as u64);

        entries.push(Entry {
            name_hash,
            zsize_index,
            uncompressed_size,
            offset,
        });
    }

    // Read ZSizes table
    let zsizes_start = reader.position() as usize;
    let zsizes_count = (toc_length as usize - 32 - (file_count as usize * 30)) / 2;
    let zsizes: Vec<u16> = (0..zsizes_count)
        .map(|i| {
            let pos = zsizes_start + (i * 2);
            u16::from_be_bytes([mmap[pos], mmap[pos + 1]])
        })
        .collect();

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Step 1: Read and parse FileList.xml from the first entry (name_hash == [0; 16])
    progress_callback(ExtractionStatus {
        current_file: "Reading FileList.xml...".to_string(),
        progress: 0.0,
        is_extracting: true,
        error: None,
    });

    let mut filename_map: HashMap<[u8; 16], String> = HashMap::new();
    
    if let Some(first_entry) = entries.first() {
        if first_entry.name_hash == [0; 16] && first_entry.offset != 0 {
            match read_file_data(&mmap, first_entry, &zsizes, block_size as usize) {
                Ok(filelist_data) => {
                    // Save FileList.xml to output directory
                    let filelist_xml_path = output_dir.join("FileList.xml");
                    if let Err(e) = std::fs::write(&filelist_xml_path, &filelist_data) {
                        eprintln!("[PSARC] Warning: Failed to save FileList.xml: {}", e);
                    } else {
                        eprintln!("[PSARC] Saved FileList.xml to output directory");
                    }

                    // Parse FileList.xml content
                    // UnPSARC splits by both '\n' and '\0'
                    let filenames_text = String::from_utf8_lossy(&filelist_data);
                    let lines: Vec<&str> = filenames_text
                        .split(|c| c == '\n' || c == '\0')
                        .filter(|line| !line.trim().is_empty())
                        .collect();

                    // Build hash map: for each filename, calculate MD5 hash and map it
                    // UnPSARC adds three versions: original, uppercase, and lowercase
                    // Important: All hash variants should map to the ORIGINAL filename (not the transformed one)
                    for line in lines {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Store original filename for mapping
                        let original_filename = trimmed.to_string();

                        // Add original case hash -> original filename
                        let hash_original = calculate_md5(trimmed);
                        filename_map.insert(hash_original, original_filename.clone());

                        // Add uppercase version hash -> original filename
                        let upper = trimmed.to_ascii_uppercase();
                        let hash_upper = calculate_md5(&upper);
                        filename_map.insert(hash_upper, original_filename.clone());

                        // Add lowercase version hash -> original filename
                        let lower = trimmed.to_ascii_lowercase();
                        let hash_lower = calculate_md5(&lower);
                        filename_map.insert(hash_lower, original_filename);
                    }

                    eprintln!("[PSARC] Loaded {} filenames from FileList.xml", filename_map.len());
                }
                Err(e) => {
                    eprintln!("[PSARC] Warning: Failed to read FileList.xml: {}", e);
                    eprintln!("[PSARC] Will use hash-based filenames instead");
                }
            }
        }
    }

    let total_entries = entries.len();
    let mut extracted_count = 0;
    let mut skipped_count = 0;
    
    for (idx, entry) in entries.iter().enumerate() {
        // Skip entries with zero name_hash (FileList.xml manifest)
        if entry.name_hash == [0; 16] {
            skipped_count += 1;
            continue;
        }

        // Skip entries with zero offset
        if entry.offset == 0 {
            skipped_count += 1;
            continue;
        }
        
        // Look up filename from hash map
        let path = if let Some(filename) = filename_map.get(&entry.name_hash) {
            // Found in filename map - use the original filename
            // Replace forward slashes with OS-specific separator
            let mut file_path = filename.replace('/', &std::path::MAIN_SEPARATOR.to_string());
            // Remove leading separator if present (UnPSARC does this)
            if file_path.starts_with(std::path::MAIN_SEPARATOR) {
                file_path = file_path[1..].to_string();
            }
            file_path
        } else {
            // Not found in filename map - use hash-based filename
            // Format hash without dashes for filename (UnPSARC uses Replace("-", ""))
            let hash_hex = format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                entry.name_hash[0], entry.name_hash[1], entry.name_hash[2], entry.name_hash[3],
                entry.name_hash[4], entry.name_hash[5], entry.name_hash[6], entry.name_hash[7],
                entry.name_hash[8], entry.name_hash[9], entry.name_hash[10], entry.name_hash[11],
                entry.name_hash[12], entry.name_hash[13], entry.name_hash[14], entry.name_hash[15]);
            
            // Put unknown files in _Unknowns directory (like UnPSARC does)
            eprintln!("[PSARC] Archive contains a hash which is not in FileList.xml table: {}", hash_to_string(&entry.name_hash));
            format!("_Unknowns{}{}.bin", std::path::MAIN_SEPARATOR, hash_hex)
        };
        
        extracted_count += 1;
        progress_callback(ExtractionStatus {
            current_file: path.clone(),
            progress: (idx as f32) / (total_entries as f32),
            is_extracting: true,
            error: None,
        });

        let file_data = match read_file_data(&mmap, entry, &zsizes, block_size as usize) {
            Ok(data) => {
                if data.len() != entry.uncompressed_size as usize {
                    eprintln!("[PSARC] Warning: File {} size mismatch: expected {}, got {}", 
                             path, entry.uncompressed_size, data.len());
                }
                data
            },
            Err(e) => {
                eprintln!("[PSARC] Failed to read file {} (offset: 0x{:X}, size: {}): {}", 
                         path, entry.offset, entry.uncompressed_size, e);
                return Err(e);
            }
        };

        let output_path = output_dir.join(&path);
        if let Some(parent) = output_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[PSARC] Failed to create directory for {}: {}", path, e);
                return Err(e);
            }
        }

        if let Err(e) = std::fs::write(&output_path, file_data) {
            eprintln!("[PSARC] Failed to write file {}: {}", path, e);
            return Err(e);
        }
    }
    
    eprintln!("[PSARC] Extraction summary: {} files extracted, {} entries skipped, {} total entries", 
              extracted_count, skipped_count, total_entries);

    progress_callback(ExtractionStatus {
        current_file: "Done".to_string(),
        progress: 1.0,
        is_extracting: false,
        error: None,
    });

    Ok(())
}

fn read_file_data(
    mmap: &Mmap,
    entry: &Entry,
    zsizes: &[u16],
    block_size: usize,
) -> io::Result<Vec<u8>> {
    let mut result = Vec::with_capacity(entry.uncompressed_size as usize);
    let mut current_zsize_index = entry.zsize_index as usize;
    let mut current_offset = entry.offset as usize;
    let mut remaining = entry.uncompressed_size as usize;

    // Verify offset is within bounds
    if current_offset >= mmap.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Entry offset {} is beyond file size {}", current_offset, mmap.len()),
        ));
    }

    // Follow UnPSARC logic: loop until we've written all uncompressed data
    while (result.len() as u64) < entry.uncompressed_size {
        let zsize = zsizes.get(current_zsize_index)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid zsize index {} (max: {})", current_zsize_index, zsizes.len()),
            ))?;

        // UnPSARC logic: if zsize == 0, compressed_size = block_size
        let compressed_size = if *zsize == 0 {
            block_size
        } else {
            *zsize as usize
        };

        // Verify we can read the compressed block
        if current_offset + compressed_size > mmap.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Block read would exceed file bounds: offset {}, size {}, file size {}",
                    current_offset, compressed_size, mmap.len()
                ),
            ));
        }

        let compressed_data = &mmap[current_offset..current_offset + compressed_size];
        
        // UnPSARC logic for determining how much to read/decompress
        let decompressed = if compressed_size == entry.uncompressed_size as usize {
            // Special case: entire file is uncompressed in one block
            compressed_data.to_vec()
        } else if *zsize == 0 {
            // Uncompressed block
            if remaining < block_size {
                // Last block - only read remaining bytes
                compressed_data[..remaining.min(compressed_data.len())].to_vec()
            } else {
                // Full block
                compressed_data.to_vec()
            }
        } else {
            // Compressed block - determine target size
            let target_size = if remaining < block_size || compressed_size == block_size {
                remaining
            } else {
                block_size
            };
            
            // Check for zlib magic (0x78DA, 0x789C, etc.)
            let is_zlib = compressed_data.len() >= 2 && 
                          compressed_data[0] == 0x78 && 
                          (compressed_data[1] == 0x9C || compressed_data[1] == 0xDA || 
                           compressed_data[1] == 0x01 || compressed_data[1] == 0x5E);
            
            if is_zlib {
                let mut decoder = ZlibDecoder::new(compressed_data);
                let mut decompressed_block = Vec::with_capacity(target_size);
                decoder.read_to_end(&mut decompressed_block)
                    .map_err(|e| io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Failed to decompress block at offset {} (zsize: {}, compressed_size: {}, target_size: {}): {}",
                            current_offset, zsize, compressed_size, target_size, e
                        ),
                    ))?;
                
                // Truncate to target size if needed (UnPSARC reads exactly target_size)
                if decompressed_block.len() > target_size {
                    decompressed_block.truncate(target_size);
                }
                decompressed_block
            } else {
                // Not compressed or unknown format - return as-is (up to target_size)
                compressed_data[..target_size.min(compressed_data.len())].to_vec()
            }
        };

        // Copy the decompressed data
        result.extend_from_slice(&decompressed);
        
        // UnPSARC logic: 
        // - BlockOffset += CompressedSize (compressed size in file)
        // - RemainingSize -= BlockSize (always subtract block_size, not actual read size)
        current_offset += compressed_size;
        remaining = remaining.saturating_sub(block_size);
        current_zsize_index += 1;
    }

    Ok(result)
}
