#!/usr/bin/env python3
import sys
import json
import struct
import re
from typing import Any, Dict, List


class EVBParser:
    def __init__(self, data: bytes):
        self.data = data
        self.header: str | None = None
        self.header_words: List[int] = []
        self.header_string_refs: List[Dict[str, Any]] = []
        self.strings: List[Dict[str, Any]] = []

    def check_type(self) -> None:
        if len(self.data) < 4:
            raise ValueError("File too small to be a valid EVB file")
        header_raw = self.data[:4].decode("ascii", errors="ignore").rstrip("\0")
        self.header = header_raw
        if header_raw != "FBKK":
            raise ValueError(f"Invalid EVB header: {header_raw!r}")

    def parse(self) -> None:
        self.check_type()

        words: List[int] = []
        header_limit = min(len(self.data), 0x80)
        for offset in range(4, header_limit, 4):
            chunk = self.data[offset : offset + 4]
            if len(chunk) < 4:
                break
            words.append(struct.unpack("<I", chunk)[0])
        self.header_words = words

        header_string_refs: List[Dict[str, Any]] = []
        for index, value in enumerate(self.header_words):
            if 0 <= value < len(self.data):
                end = min(len(self.data), value + 64)
                chunk = self.data[value:end]
                m = re.match(rb"[ -~]{1,64}", chunk)
                if m:
                    raw = m.group(0)
                    text = raw.decode("utf-8", errors="ignore")
                    if text:
                        header_string_refs.append(
                            {
                                "word_index": index,
                                "offset": value,
                                "value": text,
                            }
                        )
        self.header_string_refs = header_string_refs

        for match in re.finditer(rb"[ -~]{4,}", self.data):
            raw = match.group(0)
            text = raw.decode("utf-8", errors="ignore")
            self.strings.append(
                {
                    "offset": match.start(),
                    "value": text,
                }
            )

    def to_dict(self) -> Dict[str, Any]:
        return {
            "header": self.header,
            "header_words": self.header_words,
            "header_string_refs": self.header_string_refs,
            "strings": self.strings,
        }


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: python evb_to_json.py <input.evb> <output.json>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    with open(input_path, "rb") as f:
        data = f.read()

    parser = EVBParser(data)
    try:
        parser.parse()
    except Exception as exc:
        print(f"Failed to parse EVB file: {exc}")
        sys.exit(1)

    result = parser.to_dict()

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)


if __name__ == "__main__":
    main()


