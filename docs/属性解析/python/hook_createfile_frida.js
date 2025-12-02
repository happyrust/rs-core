// Frida 脚本：启动时 HOOK CreateFileW/A 以及 core.dll 内部 FHDBRN（偏移 0x766040），打印文件路径与句柄。
// 用法示例（Windows）:
//   frida -f <目标可执行文件> -l docs/属性解析/python/hook_createfile_frida.js --no-pause

function hookCreateFile(name) {
    const addr = Module.findExportByName("kernel32.dll", name);
    if (!addr) return;
    Interceptor.attach(addr, {
        onEnter(args) {
            this.path =
                name === "CreateFileW"
                    ? Memory.readUtf16String(args[0])
                    : Memory.readCString(args[0]);
            this.access = args[1].toInt32();
            this.share = args[2].toInt32();
        },
        onLeave(retval) {
            console.log(
                `[${name}] path="${this.path}" access=0x${this.access.toString(
                    16
                )} share=0x${this.share.toString(16)} handle=0x${retval
                    .toInt32()
                    .toString(16)}`
            );
        },
    });
}

function hookFHDBRN(base) {
    const addr = base.add(0x766040); // core.dll 内部读页函数
    try {
        Interceptor.attach(addr, {
            onEnter(args) {
                // args[0] 通常是指向 DAB 句柄的指针
                this.handlePtr = args[0];
                this.handleVal = Memory.readU32(args[0]);
                this.pagePtr = args[1];
                this.pageNum = Memory.readU32(args[1]);
                console.log(
                    `[FHDBRN] handlePtr=${this.handlePtr} handle=0x${this.handleVal.toString(
                        16
                    )} page=${this.pageNum}`
                );
            },
        });
        console.log(
            `[hook] FHDBRN hooked at ${addr} (core.dll base ${base})`
        );
    } catch (e) {
        console.error("[hook] FHDBRN hook failed:", e);
    }
}

function tryHookCore() {
    const mod = Process.findModuleByName("core.dll");
    if (mod) {
        hookFHDBRN(mod.base);
        return true;
    }
    return false;
}

// 入口
hookCreateFile("CreateFileW");
hookCreateFile("CreateFileA");

if (!tryHookCore()) {
    // core.dll 尚未加载，挂载加载回调
    const listener = Process.enumerateModules({
        onMatch(module) {
            if (module.name.toLowerCase() === "core.dll") {
                hookFHDBRN(module.base);
                return "stop";
            }
            return "skip";
        },
        onComplete() {},
    });
    // 备用：使用 Module.load 时也会触发 enumerate，这里无需额外逻辑
}

console.log("[hook] CreateFileW/A hooks installed; waiting for core.dll/FHDBRN...");
