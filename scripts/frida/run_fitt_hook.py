#!/usr/bin/env python3
"""
FITT ORI/POS Hook Runner for AVEVA E3D

使用 Frida 注入 des.exe 进程，hook core.dll 中的关键函数来捕获 FITT 元素的方位和位置信息。

用法:
    python run_fitt_hook.py [options]

选项:
    --spawn         启动新的 E3D 进程并注入
    --attach        附加到已运行的 des.exe 进程
    --pid PID       附加到指定 PID 的进程
    --verbose       启用详细日志
    --output FILE   将日志输出到文件
"""

import argparse
import frida
import sys
import os
import time
import subprocess
from pathlib import Path

# E3D 安装路径
E3D_PATH = Path(r"D:\AVEVA\Everything3D2.10")
LAUNCH_BAT = E3D_PATH / "launch.bat"
DES_EXE = E3D_PATH / "des.exe"

# Frida 脚本路径
SCRIPT_DIR = Path(__file__).parent
HOOK_SCRIPT = SCRIPT_DIR / "fitt_hook.js"


def on_message(message, data):
    """处理来自 Frida 脚本的消息"""
    if message['type'] == 'send':
        print(f"[FRIDA] {message['payload']}")
    elif message['type'] == 'error':
        print(f"[ERROR] {message['stack']}")
    else:
        print(f"[MSG] {message}")


def load_script():
    """加载 Frida hook 脚本"""
    with open(HOOK_SCRIPT, 'r', encoding='utf-8') as f:
        return f.read()


def spawn_and_attach():
    """启动 E3D 进程并注入"""
    print(f"[*] Spawning E3D from: {DES_EXE}")
    
    # 设置环境变量 (从 launch.bat 提取的关键变量)
    env = os.environ.copy()
    env['AVEVA_DESIGN_INSTALLED_DIR'] = str(E3D_PATH)
    
    # 使用 Frida spawn 模式
    device = frida.get_local_device()
    
    # 启动进程但暂停
    pid = device.spawn([str(DES_EXE)], cwd=str(E3D_PATH), env=env)
    print(f"[*] Spawned process with PID: {pid}")
    
    # 附加到进程
    session = device.attach(pid)
    
    # 加载脚本
    script_code = load_script()
    script = session.create_script(script_code)
    script.on('message', on_message)
    script.load()
    
    # 恢复进程执行
    device.resume(pid)
    print(f"[*] Process resumed, hooks active")
    
    return session, script, pid


def attach_to_process(pid=None):
    """附加到已运行的进程"""
    device = frida.get_local_device()
    
    if pid is None:
        # 查找 des.exe 进程
        for proc in device.enumerate_processes():
            if proc.name.lower() == 'des.exe':
                pid = proc.pid
                print(f"[*] Found des.exe with PID: {pid}")
                break
        
        if pid is None:
            print("[!] des.exe not found. Please start E3D first or use --spawn")
            return None, None, None
    
    print(f"[*] Attaching to PID: {pid}")
    session = device.attach(pid)
    
    # 加载脚本
    script_code = load_script()
    script = session.create_script(script_code)
    script.on('message', on_message)
    script.load()
    
    print(f"[*] Hooks active")
    return session, script, pid


def interactive_mode(script):
    """交互模式，允许动态配置"""
    print("\n" + "="*60)
    print("Interactive Mode - Commands:")
    print("  status     - Show hook status")
    print("  verbose    - Toggle verbose mode")
    print("  filter X   - Set noun filter (e.g., 'filter FITT')")
    print("  nofilter   - Clear noun filter")
    print("  quit       - Exit")
    print("="*60 + "\n")
    
    while True:
        try:
            cmd = input(">>> ").strip().lower()
            
            if cmd == 'quit' or cmd == 'exit':
                break
            elif cmd == 'status':
                status = script.exports.get_status()
                print(f"Status: {status}")
            elif cmd == 'verbose':
                script.exports.set_verbose(True)
            elif cmd == 'noverbose':
                script.exports.set_verbose(False)
            elif cmd.startswith('filter '):
                noun = cmd.split(' ', 1)[1].upper()
                script.exports.set_noun_filter(noun)
            elif cmd == 'nofilter':
                script.exports.set_noun_filter(None)
            elif cmd == 'help':
                print("Commands: status, verbose, noverbose, filter X, nofilter, quit")
            else:
                print(f"Unknown command: {cmd}")
                
        except EOFError:
            break
        except KeyboardInterrupt:
            print("\nUse 'quit' to exit")


def main():
    parser = argparse.ArgumentParser(description='FITT ORI/POS Hook for AVEVA E3D')
    parser.add_argument('--spawn', action='store_true', help='Spawn new E3D process')
    parser.add_argument('--attach', action='store_true', help='Attach to running des.exe')
    parser.add_argument('--pid', type=int, help='Attach to specific PID')
    parser.add_argument('--verbose', action='store_true', help='Enable verbose logging')
    parser.add_argument('--output', type=str, help='Output log file')
    parser.add_argument('--interactive', '-i', action='store_true', help='Interactive mode')
    
    args = parser.parse_args()
    
    # 检查 Frida 脚本是否存在
    if not HOOK_SCRIPT.exists():
        print(f"[!] Hook script not found: {HOOK_SCRIPT}")
        return 1
    
    session = None
    script = None
    pid = None
    
    try:
        if args.spawn:
            session, script, pid = spawn_and_attach()
        elif args.pid:
            session, script, pid = attach_to_process(args.pid)
        else:
            # 默认尝试附加到已运行的进程
            session, script, pid = attach_to_process()
        
        if session is None:
            return 1
        
        # 设置配置
        if args.verbose:
            script.exports.set_verbose(True)
        
        if args.interactive:
            interactive_mode(script)
        else:
            # 非交互模式，等待用户中断
            print("\n[*] Monitoring FITT ORI/POS... Press Ctrl+C to stop\n")
            try:
                while True:
                    time.sleep(1)
            except KeyboardInterrupt:
                print("\n[*] Stopping...")
        
    except frida.ProcessNotFoundError:
        print("[!] Process not found")
        return 1
    except frida.ServerNotRunningError:
        print("[!] Frida server not running")
        return 1
    except Exception as e:
        print(f"[!] Error: {e}")
        import traceback
        traceback.print_exc()
        return 1
    finally:
        if session:
            session.detach()
            print("[*] Detached from process")
    
    return 0


if __name__ == '__main__':
    sys.exit(main())
