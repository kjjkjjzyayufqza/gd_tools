import struct
import zlib
from pathlib import Path

BLOCK_SIZE = 65536


def load_psarc(path):
    data = Path(path).read_bytes()
    magic = data[:4]
    if magic != b"PSAR":
        raise ValueError("Invalid magic")
    major, minor = struct.unpack('>HH', data[4:8])
    toc_len, entry_size, file_count, block_size, flags = struct.unpack('>IIIII', data[12:32])
    offset = 32
    entries = []
    for _ in range(file_count):
        entry = data[offset:offset+entry_size]
        name_hash = entry[:16]
        zidx = struct.unpack('>I', entry[16:20])[0]
        uncomp = int.from_bytes(entry[20:25], 'big')
        off = int.from_bytes(entry[25:30], 'big')
        entries.append({
            'hash': name_hash,
            'zidx': zidx,
            'uncomp': uncomp,
            'offset': off,
        })
        offset += entry_size
    zsize_bytes = toc_len - (entry_size * file_count) - 32
    zsizes = [struct.unpack('>H', data[offset + i*2: offset + i*2 + 2])[0] for i in range(zsize_bytes // 2)]
    return {
        'major': major,
        'minor': minor,
        'entry_size': entry_size,
        'file_count': file_count,
        'block_size': block_size,
        'flags': flags,
        'entries': entries,
        'zsizes': zsizes,
        'data': data,
        'data_start': toc_len,
    }


def extract_entry(psarc, index):
    entry = psarc['entries'][index]
    zidx = entry['zidx']
    remaining = entry['uncomp']
    cursor = entry['offset']
    out = bytearray()
    data = psarc['data']
    while remaining > 0:
        block_uncomp = min(BLOCK_SIZE, remaining)
        zsize = psarc['zsizes'][zidx]
        zidx += 1
        if zsize == 0:
            stored = block_uncomp if block_uncomp < BLOCK_SIZE else BLOCK_SIZE
            block_data = data[cursor:cursor + stored]
            out.extend(block_data)
            cursor += stored
        else:
            block_data = data[cursor:cursor + zsize]
            out.extend(zlib.decompress(block_data))
            cursor += zsize
        remaining -= block_uncomp
    return bytes(out)

if __name__ == '__main__':
    ref = load_psarc('arc_0_ep_0_0_bak.psarc')
    test = load_psarc('test.psarc')
    for psarc, label in ((ref, 'ref'), (test, 'test')):
        print(label)
        for i, entry in enumerate(psarc['entries'][:5]):
            print(f"  entry{i}: zidx={entry['zidx']}, uncomp={entry['uncomp']}, off=0x{entry['offset']:X}, hash={entry['hash'].hex()}")
    ref_names = extract_entry(ref, 0).decode('utf-8', errors='replace').splitlines()
    test_names = extract_entry(test, 0).decode('utf-8', errors='replace').splitlines()
    print('\nref first 5 names:')
    for line in ref_names[:5]:
        print(' ', line)
    print('test first 5 names:')
    for line in test_names[:5]:
        print(' ', line)
    print('\nlen ref names', len(ref_names), 'entries', len(ref['entries'])-1)
    print('len test names', len(test_names), 'entries', len(test['entries'])-1)
