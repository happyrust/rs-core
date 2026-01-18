/**
 * FITT ORI/POS Hook Script for AVEVA E3D
 * 
 * 目标: 通过 Frida hook core.dll 中的关键函数，捕获 FITT 元素的方位(ORI)和位置(POS)信息
 * 
 * 关键函数 (相对于 core.dll 基址的偏移):
 * - DB_PseudoAttPlugger::getPosAtt    @ 0x0005e930
 * - DB_PseudoAttPlugger::getOriAtt    @ 0x0005e990
 * - runFortranPlugger (D3_Point)      @ 0x00051cc0
 * - runFortranPlugger (D3_Matrix)     @ 0x00057400
 * - GFITMD                            @ 0x00375798
 * - GATORI                            @ 0x000b07b8
 * - GATTRO                            @ 0x000b14aa
 */

'use strict';

// 配置选项
const CONFIG = {
    // 是否启用详细日志
    verbose: true,
    // 是否过滤特定 NOUN 类型 (如 FITT)
    filterNoun: null,  // 设置为 "FITT" 可只捕获 FITT 元素
    // 是否记录调用栈
    logBacktrace: false,
    // 输出文件路径 (null 表示只输出到控制台)
    outputFile: null
};

// 全局变量
let coreModule = null;
let coreBase = null;
let hooks = [];

// 辅助函数: 读取 D3_Point 结构 (3 个 double)
function readD3Point(ptr) {
    if (ptr.isNull()) return null;
    return {
        x: ptr.readDouble(),
        y: ptr.add(8).readDouble(),
        z: ptr.add(16).readDouble()
    };
}

// 辅助函数: 读取 D3_Matrix 结构 (9 个 double, 3x3 旋转矩阵)
function readD3Matrix(ptr) {
    if (ptr.isNull()) return null;
    const values = [];
    for (let i = 0; i < 9; i++) {
        values.push(ptr.add(i * 8).readDouble());
    }
    return {
        // 列优先存储
        col0: { x: values[0], y: values[3], z: values[6] },
        col1: { x: values[1], y: values[4], z: values[7] },
        col2: { x: values[2], y: values[5], z: values[8] },
        raw: values
    };
}

// 辅助函数: 读取 DB_Element 信息
function readElementInfo(elementPtr) {
    if (elementPtr.isNull()) return null;
    try {
        // 尝试读取元素的基本信息
        // 这里的结构需要根据实际情况调整
        return {
            ptr: elementPtr.toString(),
            // hashValue 通常在 vtable 之后
        };
    } catch (e) {
        return { ptr: elementPtr.toString(), error: e.message };
    }
}

// 辅助函数: 读取 DB_Attribute 名称
function readAttributeName(attrPtr) {
    if (attrPtr.isNull()) return null;
    try {
        // DB_Attribute 结构中包含属性名
        // 偏移量需要根据实际结构调整
        const namePtr = attrPtr.add(4).readPointer();
        if (!namePtr.isNull()) {
            return namePtr.readCString();
        }
    } catch (e) {
        return null;
    }
    return null;
}

// 日志输出
function log(message, level = 'INFO') {
    const timestamp = new Date().toISOString();
    const output = `[${timestamp}] [${level}] ${message}`;
    console.log(output);
    
    if (CONFIG.outputFile) {
        // 写入文件 (需要额外实现)
    }
}

// 初始化 hook
function initHooks() {
    // 查找 core.dll 模块
    coreModule = Process.findModuleByName('core.dll');
    if (!coreModule) {
        log('core.dll not found, waiting...', 'WARN');
        return false;
    }
    
    coreBase = coreModule.base;
    log(`Found core.dll at base: ${coreBase}`, 'INFO');
    
    // Hook getPosAtt
    hookGetPosAtt();
    
    // Hook getOriAtt
    hookGetOriAtt();
    
    // Hook runFortranPlugger (D3_Point version)
    hookRunFortranPluggerPoint();
    
    // Hook runFortranPlugger (D3_Matrix version)
    hookRunFortranPluggerMatrix();
    
    // Hook GFITMD
    hookGFITMD();
    
    log(`All hooks installed successfully`, 'INFO');
    return true;
}

// Hook DB_PseudoAttPlugger::getPosAtt
function hookGetPosAtt() {
    const offset = 0x0005e930;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            this.thisPtr = args[0];      // this (DB_PseudoAttPlugger*)
            this.elementPtr = args[1];    // DB_Element**
            this.attrPtr = args[2];       // DB_Attribute*
            this.qualPtr = args[3];       // DB_Qualifier*
            this.pointPtr = args[4];      // D3_Point* (output)
            this.msgPtr = args[5];        // MR_Message*
            
            if (CONFIG.verbose) {
                log(`getPosAtt called - element: ${this.elementPtr}, attr: ${this.attrPtr}`);
            }
        },
        onLeave: function(retval) {
            if (retval.toInt32() !== 0 && !this.pointPtr.isNull()) {
                const point = readD3Point(this.pointPtr);
                if (point) {
                    log(`[POS] Position: X=${point.x.toFixed(3)}, Y=${point.y.toFixed(3)}, Z=${point.z.toFixed(3)}`);
                }
            }
        }
    });
    
    hooks.push({ name: 'getPosAtt', addr: funcAddr });
    log(`Hooked getPosAtt at ${funcAddr}`, 'DEBUG');
}

// Hook DB_PseudoAttPlugger::getOriAtt
function hookGetOriAtt() {
    const offset = 0x0005e990;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            this.thisPtr = args[0];
            this.elementPtr = args[1];
            this.attrPtr = args[2];
            this.qualPtr = args[3];
            this.matrixPtr = args[4];     // D3_Matrix* (output)
            this.msgPtr = args[5];
            
            if (CONFIG.verbose) {
                log(`getOriAtt called - element: ${this.elementPtr}, attr: ${this.attrPtr}`);
            }
        },
        onLeave: function(retval) {
            if (retval.toInt32() !== 0 && !this.matrixPtr.isNull()) {
                const matrix = readD3Matrix(this.matrixPtr);
                if (matrix) {
                    log(`[ORI] Orientation Matrix:`);
                    log(`  X-axis: (${matrix.col0.x.toFixed(6)}, ${matrix.col0.y.toFixed(6)}, ${matrix.col0.z.toFixed(6)})`);
                    log(`  Y-axis: (${matrix.col1.x.toFixed(6)}, ${matrix.col1.y.toFixed(6)}, ${matrix.col1.z.toFixed(6)})`);
                    log(`  Z-axis: (${matrix.col2.x.toFixed(6)}, ${matrix.col2.y.toFixed(6)}, ${matrix.col2.z.toFixed(6)})`);
                }
            }
        }
    });
    
    hooks.push({ name: 'getOriAtt', addr: funcAddr });
    log(`Hooked getOriAtt at ${funcAddr}`, 'DEBUG');
}

// Hook runFortranPlugger (D3_Point version)
function hookRunFortranPluggerPoint() {
    const offset = 0x00051cc0;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            this.thisPtr = args[0];
            this.elementPtr = args[1];
            this.attrPtr = args[2];
            this.qualPtr = args[3];
            this.pointPtr = args[4];
            this.errorPtr = args[5];
            
            // 读取属性名
            const attrName = readAttributeName(this.attrPtr);
            this.attrName = attrName;
            
            if (CONFIG.verbose) {
                log(`runFortranPlugger(Point) - attr: ${attrName || 'unknown'}`);
            }
        },
        onLeave: function(retval) {
            if (retval.toInt32() !== 0 && !this.pointPtr.isNull()) {
                const point = readD3Point(this.pointPtr);
                if (point) {
                    log(`[FORTRAN-POS] ${this.attrName || 'POS'}: (${point.x.toFixed(3)}, ${point.y.toFixed(3)}, ${point.z.toFixed(3)})`);
                    
                    if (CONFIG.logBacktrace) {
                        log('Backtrace:\n' + Thread.backtrace(this.context, Backtracer.ACCURATE)
                            .map(DebugSymbol.fromAddress).join('\n'));
                    }
                }
            }
        }
    });
    
    hooks.push({ name: 'runFortranPlugger_Point', addr: funcAddr });
    log(`Hooked runFortranPlugger(Point) at ${funcAddr}`, 'DEBUG');
}

// Hook runFortranPlugger (D3_Matrix version)
function hookRunFortranPluggerMatrix() {
    const offset = 0x00057400;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            this.thisPtr = args[0];
            this.elementPtr = args[1];
            this.attrPtr = args[2];
            this.qualPtr = args[3];
            this.matrixPtr = args[4];
            this.errorPtr = args[5];
            
            const attrName = readAttributeName(this.attrPtr);
            this.attrName = attrName;
            
            if (CONFIG.verbose) {
                log(`runFortranPlugger(Matrix) - attr: ${attrName || 'unknown'}`);
            }
        },
        onLeave: function(retval) {
            if (retval.toInt32() !== 0 && !this.matrixPtr.isNull()) {
                const matrix = readD3Matrix(this.matrixPtr);
                if (matrix) {
                    log(`[FORTRAN-ORI] ${this.attrName || 'ORI'} Matrix: [${matrix.raw.map(v => v.toFixed(4)).join(', ')}]`);
                }
            }
        }
    });
    
    hooks.push({ name: 'runFortranPlugger_Matrix', addr: funcAddr });
    log(`Hooked runFortranPlugger(Matrix) at ${funcAddr}`, 'DEBUG');
}

// Hook GFITMD - FITT 元素的主要计算函数
function hookGFITMD() {
    const offset = 0x00375798;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            log(`[GFITMD] Called - FITT calculation triggered`);
            
            // 记录参数用于调试
            this.args = [];
            for (let i = 0; i < 6; i++) {
                this.args.push(args[i]);
            }
            
            if (CONFIG.logBacktrace) {
                log('GFITMD Backtrace:\n' + Thread.backtrace(this.context, Backtracer.ACCURATE)
                    .map(DebugSymbol.fromAddress).join('\n'));
            }
        },
        onLeave: function(retval) {
            log(`[GFITMD] Returned: ${retval}`);
        }
    });
    
    hooks.push({ name: 'GFITMD', addr: funcAddr });
    log(`Hooked GFITMD at ${funcAddr}`, 'DEBUG');
}

// 额外的 hook: GATORI (获取方位)
function hookGATORI() {
    const offset = 0x000b07b8;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            log(`[GATORI] Called`);
        },
        onLeave: function(retval) {
            log(`[GATORI] Returned: ${retval}`);
        }
    });
    
    hooks.push({ name: 'GATORI', addr: funcAddr });
}

// 额外的 hook: GATTRO (获取变换)
function hookGATTRO() {
    const offset = 0x000b14aa;
    const funcAddr = coreBase.add(offset);
    
    Interceptor.attach(funcAddr, {
        onEnter: function(args) {
            log(`[GATTRO] Called`);
        },
        onLeave: function(retval) {
            log(`[GATTRO] Returned: ${retval}`);
        }
    });
    
    hooks.push({ name: 'GATTRO', addr: funcAddr });
}

// 主入口
function main() {
    log('='.repeat(60));
    log('FITT ORI/POS Hook Script for AVEVA E3D');
    log('='.repeat(60));
    
    // 尝试立即初始化
    if (!initHooks()) {
        // 如果 core.dll 还未加载，等待模块加载
        log('Waiting for core.dll to load...', 'INFO');
        
        const checkInterval = setInterval(function() {
            if (initHooks()) {
                clearInterval(checkInterval);
            }
        }, 1000);
    }
    
    // 导出控制函数供外部调用
    rpc.exports = {
        // 获取当前 hook 状态
        getStatus: function() {
            return {
                coreLoaded: coreModule !== null,
                coreBase: coreBase ? coreBase.toString() : null,
                hooks: hooks.map(h => ({ name: h.name, addr: h.addr.toString() }))
            };
        },
        
        // 设置配置
        setConfig: function(key, value) {
            if (CONFIG.hasOwnProperty(key)) {
                CONFIG[key] = value;
                log(`Config updated: ${key} = ${value}`, 'INFO');
                return true;
            }
            return false;
        },
        
        // 启用/禁用详细日志
        setVerbose: function(enabled) {
            CONFIG.verbose = enabled;
            log(`Verbose mode: ${enabled}`, 'INFO');
        },
        
        // 设置 NOUN 过滤器
        setNounFilter: function(noun) {
            CONFIG.filterNoun = noun;
            log(`Noun filter set to: ${noun || 'none'}`, 'INFO');
        }
    };
}

// 启动
main();
