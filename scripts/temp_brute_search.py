import struct

ATTLIB_PATH = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"

def search():
    with open(ATTLIB_PATH, "rb") as f:
        data = f.read()
    
    pipe_hash = 641779
    pipe_hash_be = struct.pack(">I", pipe_hash)
    noun_id_907 = 907
    noun_id_907_be = struct.pack(">I", noun_id_907)
    
    print(f"Searching for PIPE Hash: {pipe_hash} (0x{pipe_hash:08X})")
    pos = 0
    while True:
        pos = data.find(pipe_hash_be, pos)
        if pos == -1: break
        print(f"  Found PIPE Hash at offset 0x{pos:08X}")
        # 查看周围 32 字节
        start = max(0, pos - 16)
        end = min(len(data), pos + 32)
        chunk = data[start:end]
        words = struct.unpack(f">{len(chunk)//4}I", chunk[:(len(chunk)//4)*4])
        print(f"    Context: {' '.join([f'0x{w:08X}({w})' for w in words])}")
        pos += 4

    print(f"\nSearching for NounID 907: {noun_id_907}")
    pos = 0
    while True:
        pos = data.find(noun_id_907_be, pos)
        if pos == -1: break
        # 如果这个 907 周围有 PIPE 哈希，说明找到了定义处
        start = max(0, pos - 32)
        end = min(len(data), pos + 32)
        chunk = data[start:end]
        words = struct.unpack(f">{len(chunk)//4}I", chunk[:(len(chunk)//4)*4])
        if pipe_hash in words:
            print(f"  Found 907 near PIPE Hash at offset 0x{pos:08X}!")
            print(f"    Context: {' '.join([f'0x{w:08X}({w})' for w in words])}")
        pos += 4

if __name__ == "__main__":
    search()
