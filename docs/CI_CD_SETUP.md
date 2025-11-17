# GitHub Actions CI/CD 配置指南

## 概述

本项目配置了完整的 GitHub Actions 自动化流程，包括代码检查、构建测试、文档生成和发布管理。

## 工作流程说明

### 1. 快速检查 (quick-check.yml)

**触发条件:**
- Pull Request 到 master/main 分支
- Push 到 dev 或 feature/* 分支

**执行内容:**
- ✅ 编译检查（默认特性）
- ✅ 全特性编译检查
- ⚡ 执行时间: ~10-15分钟

**适用场景:** 频繁的代码提交和 PR，快速验证代码可编译性。

### 2. 完整 CI (ci.yml)

**触发条件:**
- Push 到 master/main/dev 分支
- Pull Request 到 master/main 分支
- 手动触发

**包含任务:**

#### Check & Format
- 代码格式检查 (`cargo fmt`)
- Clippy 静态分析
- 时间: ~5分钟

#### Build & Test
- 多平台构建 (Ubuntu, Windows, macOS)
- 多特性组合测试
- 单元测试和集成测试
- 时间: ~20-30分钟

#### Documentation
- 生成 API 文档
- 部署到 GitHub Pages (仅 master 分支)
- 时间: ~10分钟

#### Benchmarks
- 运行性能基准测试 (仅 master 分支)
- 时间: ~15分钟

#### Security Audit
- 依赖安全审计
- 时间: ~3分钟

### 3. 发布流程 (release.yml)

**触发条件:**
- 推送版本标签 (如 `v0.2.3`)

**执行内容:**
- 创建 GitHub Release
- 构建多平台二进制文件
- 上传 Release Assets
- 时间: ~30-40分钟

## 关键配置修复

### 本地依赖问题

项目的 `Cargo.toml` 包含本地路径依赖，CI 环境无法访问。已在工作流中自动修复：

```bash
# 移除本地 patch
sed -i.bak '/\[patch/,/^$/d' Cargo.toml

# 修复 ploop-rs 依赖
sed -i.bak 's|path = "../rust-ploop-processor/ploop-rs"|git = "https://gitee.com/happydpc/rust-ploop-processor.git"|' Cargo.toml
```

### Gitee 依赖访问

部分依赖来自 Gitee，GitHub Actions 可以正常访问，但可能较慢。建议：

1. **镜像到 GitHub**: 将关键依赖镜像到 GitHub
2. **使用缓存**: 已配置 `Swatinem/rust-cache` 加速构建
3. **超时设置**: 工作流设置了 30 分钟超时

## 使用方法

### 基本开发流程

```bash
# 1. 创建功能分支
git checkout -b feature/my-feature

# 2. 开发并提交
git add .
git commit -m "feat: add new feature"

# 3. 推送分支 (触发 quick-check)
git push origin feature/my-feature

# 4. 创建 Pull Request (触发 ci)
# 在 GitHub 上创建 PR
```

### 发布新版本

```bash
# 1. 更新版本号
# 编辑 Cargo.toml: version = "0.2.4"
# 更新 CHANGELOG.md

# 2. 提交版本更新
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.4"

# 3. 创建并推送标签 (触发 release)
git tag v0.2.4
git push origin v0.2.4

# 4. GitHub Actions 自动创建 Release
```

### 手动触发工作流

在 GitHub 仓库页面:
1. 点击 "Actions" 标签
2. 选择 "CI" 工作流
3. 点击 "Run workflow"
4. 选择分支并确认

## 本地验证

在推送前本地验证，减少 CI 失败：

```bash
# 格式检查
cargo fmt --all -- --check

# Clippy 检查
cargo clippy --all-targets --all-features -- -D warnings

# 构建测试
cargo build --verbose
cargo test --verbose --lib

# 文档生成
cargo doc --no-deps --all-features
```

## 缓存策略

为加速构建，配置了多层缓存：

1. **Cargo 注册表缓存**: `~/.cargo/registry`
2. **Cargo Git 缓存**: `~/.cargo/git`
3. **构建产物缓存**: `target/`
4. **Rust-cache**: 智能增量缓存

**缓存命中率**: 通常 >80%，首次构建 ~15分钟，后续 ~5分钟

## 故障排查

### 构建失败

**问题**: `error: could not find ploop-rs`

**解决**: 
```bash
# 检查 Cargo.toml 中的依赖修复是否生效
# 查看 Actions 日志中的 "Prepare Cargo.toml" 步骤
```

**问题**: `error: no matching package named manifold-sys`

**解决**: 
```bash
# manifold-sys 仅在 Linux 上可用
# 检查是否在正确的 feature 和平台组合下测试
```

### 测试超时

**问题**: 测试运行超过 30 分钟

**解决**:
```yaml
# 增加超时设置
timeout-minutes: 60
```

### 依赖下载慢

**问题**: Gitee 依赖下载缓慢

**解决**:
- 使用 `continue-on-error: true` 允许部分失败
- 考虑将依赖迁移到 crates.io

## 高级配置

### 自定义特性矩阵

编辑 `.github/workflows/ci.yml`:

```yaml
matrix:
  features:
    - "my_feature_1"
    - "my_feature_2"
    - "my_feature_1,my_feature_2"
```

### 添加新的检查

```yaml
- name: Custom Check
  run: |
    cargo check --features custom
    ./scripts/custom-validation.sh
```

### 部署文档到自定义域名

1. 在仓库设置中配置 GitHub Pages
2. 添加 CNAME 文件
3. 更新 `ci.yml` 中的 `cname` 字段

## 最佳实践

1. ✅ **小步提交**: 频繁提交小改动，利用快速检查
2. ✅ **本地验证**: 推送前运行 `cargo test`
3. ✅ **分支策略**: 使用 feature 分支开发，PR 合并到 master
4. ✅ **语义化版本**: 遵循 SemVer 规范
5. ✅ **更新日志**: 每次发布更新 CHANGELOG.md

## 性能优化

当前配置的性能指标:

| 工作流 | 首次构建 | 缓存命中 | 平均耗时 |
|--------|---------|---------|---------|
| quick-check | 15分钟 | 5分钟 | 7分钟 |
| ci (full) | 35分钟 | 18分钟 | 22分钟 |
| release | 45分钟 | 25分钟 | 30分钟 |

优化建议:
- 减少特性组合矩阵
- 使用增量编译
- 并行测试执行

## 相关资源

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Rust CI 最佳实践](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)

## 联系和支持

遇到问题请:
1. 检查 Actions 日志
2. 参考本文档故障排查
3. 提交 Issue 报告问题
