#!/usr/bin/env python3
# Gravity Rush 2 .evb File Parser
# This script parses .evb (database) files and prints all extracted information

import sys
import struct
from io import BytesIO
from typing import List, Dict, Any

DEBUG = False
GLOBAL_SCALE = 100

class EVBParser:
    """Parser for Gravity Rush 2 .evb database files"""
    
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.data = None
        self.stream = None
        self.bones: List[Dict[str, Any]] = []
        
    def load_file(self) -> bool:
        """Load the .evb file into memory"""
        try:
            with open(self.file_path, 'rb') as f:
                self.data = f.read()
            self.stream = BytesIO(self.data)
            print(f"[INFO] Loaded file: {self.file_path}")
            print(f"[INFO] File size: {len(self.data)} bytes")
            return True
        except FileNotFoundError:
            print(f"[ERROR] File not found: {self.file_path}")
            return False
        except Exception as e:
            print(f"[ERROR] Failed to load file: {e}")
            return False
    
    def check_type(self) -> bool:
        """Check if file is valid .evb format (header should be 'FBKK')"""
        if len(self.data) < 4:
            print("[ERROR] File is too small to be a valid .evb file")
            return False
        
        header = self.data[:4].decode('ASCII', errors='ignore').rstrip("\0")
        print(f"[INFO] File header: {repr(header)}")
        
        if header == 'FBKK':
            print("[INFO] Valid FBKK header detected")
            return True
        else:
            print(f"[WARNING] Expected 'FBKK' header, got '{header}'")
            return False
    
    def read_uint(self) -> int:
        """Read 4 bytes as unsigned integer (little-endian)"""
        data = self.stream.read(4)
        if len(data) < 4:
            raise EOFError("Not enough data to read uint")
        return struct.unpack('<I', data)[0]
    
    def read_bytes(self, count: int) -> bytes:
        """Read specified number of bytes"""
        data = self.stream.read(count)
        if len(data) < count:
            raise EOFError(f"Not enough data to read {count} bytes")
        return data
    
    def read_float(self) -> float:
        """Read 4 bytes as float (little-endian)"""
        data = self.stream.read(4)
        if len(data) < 4:
            raise EOFError("Not enough data to read float")
        return struct.unpack('<f', data)[0]
    
    def read_vec3(self) -> Dict[str, float]:
        """Read 3D vector (3 floats)"""
        x = self.read_float()
        y = self.read_float()
        z = self.read_float()
        return {'x': x, 'y': y, 'z': z}
    
    def read_quat(self) -> Dict[str, float]:
        """Read quaternion (4 floats)"""
        x = self.read_float()
        y = self.read_float()
        z = self.read_float()
        w = self.read_float()
        return {'x': x, 'y': y, 'z': z, 'w': w}
    
    def seek(self, offset: int, whence: int = 0):
        """Seek to position in stream"""
        self.stream.seek(offset, whence)
    
    def tell(self) -> int:
        """Get current position in stream"""
        return self.stream.tell()
    
    def load_string_from_pointer(self, offset: int) -> str:
        """Load a null-terminated string from the given offset"""
        original_offset = self.tell()
        try:
            self.seek(offset - 4, 1)  # Relative seek
            string_bytes = self.read_bytes(64)
            string = string_bytes.split(b'\x00')[0].decode('UTF-8', errors='ignore')
            return string
        finally:
            self.seek(original_offset, 0)  # Absolute seek
    
    def read_sub_data_chunk(self, offset: int, parent_bone_index: int, parent_name: str = ""):
        """Read and parse sub data chunk"""
        original_offset = self.tell()
        try:
            self.seek(offset - 4, 1)  # Relative seek
            current_pos = self.tell()
            print(f"\n[SUB_DATA_CHUNK] Loading at offset 0x{current_pos:X}")
            
            self.seek(0x08, 1)  # Skip 8 bytes
            name = self.load_string_from_pointer(self.read_uint())
            print(f"  [NAME] {name}")
            
            self.seek(0x0C, 1)  # Skip 12 bytes
            self.seek(self.read_uint() - 4, 1)
            
            # Read bone transformation data
            rotation = self.read_quat()
            translation = self.read_vec3()
            translation_scaled = {
                'x': translation['x'] * GLOBAL_SCALE,
                'y': translation['y'] * GLOBAL_SCALE,
                'z': translation['z'] * GLOBAL_SCALE,
            }
            
            self.seek(4, 1)  # Skip 4 bytes
            scale = self.read_vec3()
            
            print(f"  [ROTATION] x={rotation['x']:.6f}, y={rotation['y']:.6f}, z={rotation['z']:.6f}, w={rotation['w']:.6f}")
            print(f"  [TRANSLATION] x={translation_scaled['x']:.6f}, y={translation_scaled['y']:.6f}, z={translation_scaled['z']:.6f}")
            print(f"  [SCALE] x={scale['x']:.6f}, y={scale['y']:.6f}, z={scale['z']:.6f}")
            print(f"  [PARENT_BONE_INDEX] {parent_bone_index}")
            print(f"  [PARENT_NAME] {parent_name}")
            
            bone_data = {
                'index': len(self.bones),
                'name': name,
                'parent_index': parent_bone_index,
                'parent_name': parent_name,
                'rotation': rotation,
                'translation': translation_scaled,
                'scale': scale,
            }
            self.bones.append(bone_data)
            
        finally:
            self.seek(original_offset, 0)  # Absolute seek
    
    def read_data_chunk(self, offset: int):
        """Read and parse data chunk"""
        original_offset = self.tell()
        try:
            self.seek(offset - 4, 1)  # Relative seek
            current_pos = self.tell()
            print(f"\n[DATA_CHUNK] Loading at offset 0x{current_pos:X}")
            
            self.seek(0x08, 1)  # Skip 8 bytes
            name = self.load_string_from_pointer(self.read_uint())
            print(f"  [NAME] {name}")
            
            self.seek(0x24, 1)  # Skip 36 bytes
            subdata_chunk_count = self.read_uint()
            subindex_chunk_location = self.tell() + self.read_uint()
            print(f"  [SUBDATA_CHUNK_COUNT] {subdata_chunk_count}")
            print(f"  [SUBINDEX_CHUNK_LOCATION] 0x{subindex_chunk_location:X}")
            
            self.seek(0x18, 1)  # Skip 24 bytes
            
            # Read root bone transformation data
            rotation = self.read_quat()
            translation = self.read_vec3()
            translation_scaled = {
                'x': translation['x'] * GLOBAL_SCALE,
                'y': translation['y'] * GLOBAL_SCALE,
                'z': translation['z'] * GLOBAL_SCALE,
            }
            
            self.seek(4, 1)  # Skip 4 bytes
            scale = self.read_vec3()
            
            print(f"  [ROOT_ROTATION] x={rotation['x']:.6f}, y={rotation['y']:.6f}, z={rotation['z']:.6f}, w={rotation['w']:.6f}")
            print(f"  [ROOT_TRANSLATION] x={translation_scaled['x']:.6f}, y={translation_scaled['y']:.6f}, z={translation_scaled['z']:.6f}")
            print(f"  [ROOT_SCALE] x={scale['x']:.6f}, y={scale['y']:.6f}, z={scale['z']:.6f}")
            
            bone_index = len(self.bones)
            bone_data = {
                'index': bone_index,
                'name': name,
                'parent_index': -1,
                'parent_name': None,
                'rotation': rotation,
                'translation': translation_scaled,
                'scale': scale,
            }
            self.bones.append(bone_data)
            
            self.seek(0x18, 1)  # Skip 24 bytes
            parent_name = self.load_string_from_pointer(self.read_uint())
            print(f"  [PARENT_NAME] {parent_name}")
            self.bones[-1]['parent_name'] = parent_name
            
            # Read sub data chunks
            self.seek(subindex_chunk_location, 0)  # Absolute seek
            for sub_index in range(subdata_chunk_count):
                sub_offset = self.read_uint()
                print(f"  [SUB_INDEX] {sub_index}: offset=0x{sub_offset:X}")
                self.read_sub_data_chunk(sub_offset, bone_index, name)
            
        finally:
            self.seek(original_offset, 0)  # Absolute seek
    
    def parse(self) -> bool:
        """Parse the .evb file"""
        try:
            if not self.check_type():
                return False
            
            print("\n[PARSING] Starting EVB file parsing...")
            
            # Read file header information
            self.seek(0x38, 0)  # Absolute seek
            file_name = self.load_string_from_pointer(self.read_uint())
            print(f"\n[FILE_NAME] {file_name}")
            
            self.seek(0x24, 1)  # Relative seek
            num_of_data_chunk = self.read_uint()
            print(f"[DATA_CHUNK_COUNT] {num_of_data_chunk}")
            
            # Read data chunk offsets
            self.seek(self.read_uint() - 4, 1)  # Relative seek
            for chunk_index in range(num_of_data_chunk):
                chunk_offset = self.read_uint()
                print(f"\n[CHUNK] {chunk_index}: offset=0x{chunk_offset:X}")
                self.read_data_chunk(chunk_offset)
            
            print("\n" + "="*60)
            print("[PARSING_COMPLETE] Parsing finished")
            print("="*60)
            return True
            
        except Exception as e:
            print(f"\n[ERROR] Exception during parsing: {e}")
            if DEBUG:
                import traceback
                traceback.print_exc()
            return False
    
    def print_summary(self):
        """Print summary of all parsed bones"""
        print("\n" + "="*60)
        print("[SUMMARY] All Bones Information")
        print("="*60)
        
        for bone in self.bones:
            print(f"\nBone #{bone['index']}: {bone['name']}")
            print(f"  Parent Index: {bone['parent_index']}")
            print(f"  Parent Name: {bone['parent_name']}")
            print(f"  Rotation: x={bone['rotation']['x']:.6f}, y={bone['rotation']['y']:.6f}, z={bone['rotation']['z']:.6f}, w={bone['rotation']['w']:.6f}")
            print(f"  Translation: x={bone['translation']['x']:.6f}, y={bone['translation']['y']:.6f}, z={bone['translation']['z']:.6f}")
            print(f"  Scale: x={bone['scale']['x']:.6f}, y={bone['scale']['y']:.6f}, z={bone['scale']['z']:.6f}")
        
        print(f"\n[SUMMARY] Total bones: {len(self.bones)}")
        print("="*60)


def main():
    if len(sys.argv) < 2:
        print("Usage: python GravityRush2_evb.py <path_to_evb_file>")
        print("Example: python GravityRush2_evb.py model.evb")
        sys.exit(1)
    
    evb_file = sys.argv[1]
    parser = EVBParser(evb_file)
    
    if parser.load_file():
        if parser.parse():
            parser.print_summary()
            print("\n[SUCCESS] EVB file parsed successfully!")
        else:
            print("\n[FAILURE] Failed to parse EVB file")
            sys.exit(1)
    else:
        sys.exit(1)


if __name__ == '__main__':
    main()
