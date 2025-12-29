import struct
import sys

def scan_attlib(file_path, target_hashes):
    with open(file_path, 'rb') as f:
        # 读取段指针
        f.seek(0x0800)
        segment_pointers = []
        for _ in range(8):
            data = f.read(4)
            if len(data) == 4:
                segment_pointers.append(struct.unpack('>I', data)[0])
        
        print(f"段指针: {segment_pointers}")
        
        f.seek(0x1000)
        data = f.read()
        
        words = []
        for i in range(0, len(data), 4):
            if i + 4 <= len(data):
                words.append(struct.unpack('>I', data[i:i+4])[0])
        
        print(f"总 words: {len(words)}")
        
        for h in target_hashes:
            print(f"\n查找 Hash: {h} (0x{h:08X})")
            found = False
            for i, word in enumerate(words):
                if word == h:
                    page = i // 512
                    slot = i % 512
                    print(f"  找到匹配! Word 索引: {i} -> 页: {page}, 槽: {slot}")
                    # 打印上下文
                    ctx = words[max(0, i-2):min(len(words), i+5)]
                    print(f"  上下文: {[hex(x) for x in ctx]}")
                    found = True
            if not found:
                print("  未找到匹配")

if __name__ == "__main__":
    test_hashes = [545713, 639374, 642215] # POS, NAME, TYPE
    scan_attlib(sys.argv[1], test_hashes)
