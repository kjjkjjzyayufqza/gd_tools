# PSARC File Structure Analysis

## File Layout Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PSARC File Structure                                │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ HEADER (32 bytes, offset 0x00-0x1F)                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│ 0x00: 50 53 41 52          Magic: "PSAR"                                   │
│ 0x04: 00 01                 Major Version: 1                                │
│ 0x06: 00 04                 Minor Version: 4                               │
│ 0x08: 7A 6C 69 62           Compression: "zlib"                             │
│ 0x0C: 00 01 88 6A           TOC Length: 100458 bytes                       │
│ 0x10: 00 00 00 1E           Entry Size: 30 bytes                           │
│ 0x14: 00 00 07 A0           Files Count: 1952 files                        │
│ 0x18: 00 01 00 00           Block Size: 65536 bytes (64KB)                 │
│ 0x1C: 00 00 00 01           Flags: 1 (ignorecase)                          │
└─────────────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ TOC ENTRIES (30 bytes × file_count, starts at 0x20)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│ Entry 0 (offset 0x20):                                                      │
│   ┌─────────────────────────────────────────────────────────────┐          │
│   │ 0x00-0x0F: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  │          │
│   │            name_hash (all zeros = filenames.txt manifest)  │          │
│   │ 0x10-0x13: 00 00 00 B0                                        │          │
│   │            zsize_index: 176 (points to ZSizes[176])         │          │
│   │ 0x14:      50                                                │          │
│   │ 0x15-0x18: 00 00 01 88 6A                                    │          │
│   │            uncompressed_size: 0x5000001886A bytes           │          │
│   │ 0x19:      8A                                                │          │
│   │ 0x1A-0x1D: E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23     │          │
│   │            offset: absolute offset in file                 │          │
│   └─────────────────────────────────────────────────────────────┘          │
│                                                                              │
│ Entry 1 (offset 0x3E):                                                      │
│   ┌─────────────────────────────────────────────────────────────┐          │
│   │ 0x00-0x0F: 8A E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23  │          │
│   │            name_hash: MD5 of actual filename                 │          │
│   │ 0x10-0x13: 00 00 00 01                                        │          │
│   │            zsize_index: 1                                    │          │
│   │ 0x14:      00                                                │          │
│   │ 0x15-0x18: 00 00 0C C7                                       │          │
│   │            uncompressed_size: 0xCC7 = 3271 bytes            │          │
│   │ 0x19:      00                                                │          │
│   │ 0x1A-0x1D: 00 01 AC                                          │          │
│   │            offset: 0x1AC = 428 bytes from data start        │          │
│   └─────────────────────────────────────────────────────────────┘          │
│                                                                              │
│ ... (1950 more entries) ...                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ ZSIZES TABLE (2 bytes × zsize_count, starts after last entry)              │
├─────────────────────────────────────────────────────────────────────────────┤
│ ZSizes[0]:    00 00  (uncompressed block, size = block_size)               │
│ ZSizes[1]:    00 00  (uncompressed block)                                   │
│ ZSizes[2]:    12 34  (compressed block, size = 0x1234 = 4660 bytes)        │
│ ZSizes[3]:    56 78  (compressed block, size = 0x5678 = 22136 bytes)       │
│ ...                                                                          │
│ ZSizes[176]:  XX XX  (compressed size for first file's blocks)             │
│ ...                                                                          │
│                                                                              │
│ Calculation: zsize_count = (TOC_Length - 32 - (file_count × 30)) / 2       │
│              = (100458 - 32 - (1952 × 30)) / 2                              │
│              = (100458 - 32 - 58560) / 2                                    │
│              = 41866 / 2 = 20933 entries                                    │
└─────────────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ DATA SECTION (starts at offset = TOC_Length = 0x1886A)                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ File 0 (filenames.txt) data blocks:                                         │
│   ┌─────────────────────────────────────────────────────────┐             │
│   │ Block 0: [compressed data, size from ZSizes[176]]       │             │
│   │ Block 1: [compressed data, size from ZSizes[177]]       │             │
│   │ ...                                                      │             │
│   └─────────────────────────────────────────────────────────┘             │
│                                                                              │
│ File 1 data blocks:                                                         │
│   ┌─────────────────────────────────────────────────────────┐             │
│   │ Block 0: [compressed data, size from ZSizes[1]]          │             │
│   │ ...                                                      │             │
│   └─────────────────────────────────────────────────────────┘             │
│                                                                              │
│ File 2 data blocks:                                                         │
│   ┌─────────────────────────────────────────────────────────┐             │
│   │ Block 0: [compressed data, size from ZSizes[2]]          │             │
│   │ ...                                                      │             │
│   └─────────────────────────────────────────────────────────┘             │
│                                                                              │
│ ... (more files) ...                                                        │
│                                                                              │
│ Note: Each file's blocks are stored sequentially.                           │
│       Block size = 65536 bytes (64KB) when uncompressed.                   │
│       Compressed size comes from ZSizes table.                              │
│       If ZSize = 0, block is uncompressed (size = block_size).            │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Detailed Structure Breakdown

### Header (32 bytes, offset 0x00-0x1F)

```
Offset  Value       Description
─────────────────────────────────────────────────────────────
0x00    50 53 41 52 Magic: "PSAR"
0x04    00 01       Major Version: 1
0x06    00 04       Minor Version: 4
0x08    7A 6C 69 62 Compression: "zlib"
0x0C    00 01 88 6A TOC Length: 100458 bytes
0x10    00 00 00 1E Entry Size: 30 bytes
0x14    00 00 07 A0 Files Count: 1952 files
0x18    00 01 00 00 Block Size: 65536 bytes (64KB)
0x1C    00 00 00 01 Flags: 1 (ignorecase)
```

### TOC Entry Structure (30 bytes each)

Each entry represents one file in the archive:

```
Byte Range    Size    Field              Description
─────────────────────────────────────────────────────────────
0x00-0x0F     16      name_hash          MD5 hash of filename
0x10-0x13     4       zsize_index        Index into ZSizes table
0x14          1       uncomp_size_high   High byte of 40-bit size
0x15-0x18     4       uncomp_size_low    Low 32 bits of size (BE)
0x19          1       offset_high        High byte of 40-bit offset
0x1A-0x1D     4       offset_low         Low 32 bits of offset (BE)
```

**Example Entry 0 (from your data at 0x20):**
```
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  [name_hash: all zeros = filenames.txt]
00 00 00 B0                                        [zsize_index: 176]
50 00 00 01 88 6A                                  [uncompressed_size: 0x5000001886A]
8A E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23  [offset: absolute file offset]
```

**Example Entry 1 (from your data at 0x3E):**
```
8A E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23  [name_hash: MD5 of actual filename]
00 00 00 01                                        [zsize_index: 1]
00 00 00 0C C7                                     [uncompressed_size: 0xCC7 = 3271 bytes]
00 00 01 AC                                        [offset: 0x1AC = 428 bytes from data start]
```

### ZSizes Table

Located immediately after TOC Entries:
- **Start offset**: `32 + (file_count × 30)` = `32 + (1952 × 30)` = `32 + 58560` = `58592` (0xE4E0)
- **Size calculation**: `(TOC_Length - 32 - (file_count × 30)) / 2`
  - `= (100458 - 32 - 58560) / 2`
  - `= 41866 / 2`
  - `= 20933 entries`
- Each entry is 2 bytes (u16 big-endian)
- Represents compressed size of each data block
- Value 0 means block is uncompressed (size = block_size)

### Data Section

- **Start offset**: `TOC_Length` = `0x1886A` = `100458` bytes (absolute offset in file)
- Files are stored as sequences of compressed blocks
- Each file's blocks are referenced by its entry's `zsize_index`
- Blocks are read sequentially using ZSizes table

## Data Flow Example

For extracting a file:

1. **Read entry from TOC**:
   - Get `name_hash`, `zsize_index`, `uncompressed_size`, `offset`

2. **Start reading blocks from `offset`** (absolute offset in file):
   - Current position = `offset`
   - Current zsize_index = entry's `zsize_index`

3. **For each block**:
   - Read `zsize` from `ZSizes[current_zsize_index]`
   - If `zsize == 0`: 
     - Block is uncompressed
     - Read `block_size` bytes (or remaining bytes if last block)
   - If `zsize != 0`:
     - Block is compressed
     - Read `zsize` bytes from file
     - Decompress to `block_size` bytes (or remaining bytes if last block)
   - Increment `current_zsize_index`
   - Move `offset` forward by compressed size (`zsize` or `block_size`)

4. **Continue until** `uncompressed_size` bytes are read

## Your Specific Data Analysis

From offset 0x30 (which is Entry 0 at offset 0x20 + 0x10):

```
Entry 0 (filenames.txt):
  name_hash:        00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
  zsize_index:      00 00 00 B0 = 176
  uncompressed_size: 50 00 00 01 88 6A = 0x5000001886A = 5497558139274 bytes (very large!)
  offset:           8A E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23 = absolute offset

Entry 1:
  name_hash:        8A E8 A1 F7 B0 D9 B8 07 C0 26 96 9A 3E 21 EE 23
  zsize_index:      00 00 00 01 = 1
  uncompressed_size: 00 00 00 0C C7 = 0xCC7 = 3271 bytes
  offset:           00 00 01 AC = 0x1AC = 428 bytes from data start
```

Note: The offset in Entry 0 seems unusually large. This might be because it's stored as an absolute offset rather than relative to data start.
