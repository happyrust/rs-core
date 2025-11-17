#!/usr/bin/env bash
# CI 环境准备脚本
# 用于修复本地路径依赖，使项目能在 CI 环境中构建

set -euo pipefail

echo "🔧 准备 CI 环境..."

# 备份原始 Cargo.toml
if [ ! -f "Cargo.toml.original" ]; then
    echo "📝 备份 Cargo.toml -> Cargo.toml.original"
    cp Cargo.toml Cargo.toml.original
fi

# 修复本地 patch
if grep -q '\[patch' Cargo.toml; then
    echo "🔨 移除本地 patch 配置..."
    sed -i.bak '/\[patch/,/^$/d' Cargo.toml
    echo "✅ 已移除 patch 配置"
fi

# 修复 ploop-rs 本地路径依赖
if grep -q 'path = "../rust-ploop-processor' Cargo.toml; then
    echo "🔨 修复 ploop-rs 依赖..."
    sed -i.bak 's|ploop-rs = { path = "../rust-ploop-processor/ploop-rs" }|ploop-rs = { git = "https://gitee.com/happydpc/rust-ploop-processor.git", branch = "dev", package = "ploop-rs" }|' Cargo.toml
    echo "✅ 已修复 ploop-rs 依赖"
fi

# 显示修改后的关键部分
echo ""
echo "📋 修改后的依赖配置:"
echo "===================="
tail -20 Cargo.toml

echo ""
echo "✅ CI 环境准备完成！"
echo ""
echo "现在可以运行:"
echo "  cargo check"
echo "  cargo test"
echo "  cargo build --release"
