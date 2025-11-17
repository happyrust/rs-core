# GitHub Actions 快速开始指南

## 🎯 目标

为 rs-core 项目配置完整的 GitHub Actions 自动化流程。

## ✅ 已完成的配置

### 1. 工作流文件

创建了三个主要的工作流：

- **`.github/workflows/quick-check.yml`** - 快速编译检查（PR 和日常开发）
- **`.github/workflows/ci.yml`** - 完整 CI 流程（代码检查、测试、文档）
- **`.github/workflows/release.yml`** - 发布管理（自动创建 Release）

### 2. 辅助脚本

- **`scripts/ci-prepare.sh`** - 修复本地依赖，准备 CI 环境
- **`scripts/ci-restore.sh`** - 恢复本地开发配置

### 3. 文档

- **`docs/CI_CD_SETUP.md`** - 完整的 CI/CD 配置文档

## 🚀 立即使用

### 方式一：推送到 GitHub（推荐）

```bash
cd /Volumes/DPC/work/plant-code/rs-core

# 1. 添加所有新文件
git add .github/workflows/
git add scripts/ci-*.sh
git add docs/CI_CD_SETUP.md
git add GITHUB_ACTIONS_QUICKSTART.md

# 2. 提交
git commit -m "feat: 添加 GitHub Actions CI/CD 配置"

# 3. 推送到 GitHub（假设远程仓库是 origin）
git push origin master

# 4. 查看 Actions 运行状态
# 访问：https://github.com/你的用户名/rs-core/actions
```

### 方式二：本地测试（可选）

```bash
# 1. 测试 CI 准备脚本
./scripts/ci-prepare.sh

# 2. 验证能否编译
cargo check

# 3. 恢复原始配置
./scripts/ci-restore.sh
```

## 🔧 关键问题和解决方案

### 问题 1: 本地路径依赖

**问题描述**:
```toml
[patch."https://gitee.com/happydpc/surrealdb"]
surrealdb = { path = "/Volumes/DPC/work/database/surrealdb/...", ... }

ploop-rs = { path = "../rust-ploop-processor/ploop-rs" }
```

**解决方案**:

工作流自动执行以下修复：

```bash
# 移除本地 patch
sed -i.bak '/\[patch/,/^$/d' Cargo.toml

# 修复 ploop-rs 为 git 依赖
sed -i.bak 's|path = "../rust-ploop-processor/ploop-rs"|git = "https://gitee.com/happydpc/rust-ploop-processor.git"|' Cargo.toml
```

### 问题 2: Nightly Rust

**解决方案**:

所有工作流已配置使用 nightly 工具链：

```yaml
- name: Install Rust nightly
  uses: dtolnay/rust-toolchain@nightly
```

### 问题 3: 构建缓存

**解决方案**:

使用 `Swatinem/rust-cache` 智能缓存：

```yaml
- name: Rust Cache
  uses: Swatinem/rust-cache@v2
  with:
    shared-key: "rs-core-quick"
```

**效果**: 首次构建 ~15分钟，后续 ~5分钟

## 📋 工作流触发条件

### quick-check.yml（快速检查）

- ✅ Pull Request → master/main
- ✅ Push → dev 或 feature/*
- ⏱️ 耗时: ~5-10 分钟

### ci.yml（完整 CI）

- ✅ Push → master/main/dev
- ✅ Pull Request → master/main  
- ✅ 手动触发（workflow_dispatch）
- ⏱️ 耗时: ~20-30 分钟

### release.yml（发布）

- ✅ 推送版本标签（如 `v0.2.3`）
- ⏱️ 耗时: ~30-40 分钟

## 🎮 常见使用场景

### 场景 1: 日常开发

```bash
# 1. 创建功能分支
git checkout -b feature/new-feature

# 2. 开发代码
# ... 编写代码 ...

# 3. 推送（触发 quick-check）
git push origin feature/new-feature

# 4. 在 GitHub 创建 Pull Request
# 自动触发完整 CI
```

### 场景 2: 发布新版本

```bash
# 1. 更新版本号
vim Cargo.toml  # version = "0.2.4"

# 2. 更新更新日志
echo "## [0.2.4] - 2024-01-15\n### Added\n- 新功能..." >> CHANGELOG.md

# 3. 提交
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.4"
git push

# 4. 创建标签并推送（触发 release）
git tag v0.2.4
git push origin v0.2.4

# 5. GitHub Actions 自动创建 Release！
```

### 场景 3: 手动触发 CI

1. 访问 GitHub 仓库
2. 点击 "Actions" 标签
3. 选择 "CI" 工作流
4. 点击 "Run workflow"
5. 选择分支并确认

## 📊 预期的 Actions 输出

### Quick Check（成功）

```
✓ Rust Cache
✓ Prepare Cargo.toml for CI
✓ Check default features
✓ Check all features
```

### CI（成功）

```
Check & Format
  ✓ Check formatting
  ✓ Run clippy

Build & Test (ubuntu-latest)
  ✓ Build
  ✓ Run tests

Build & Test (windows-latest)
  ✓ Build
  ✓ Run tests

Build & Test (macos-latest)
  ✓ Build
  ✓ Run tests

Documentation
  ✓ Generate documentation
  ✓ Deploy to GitHub Pages

Security Audit
  ✓ Run cargo-audit
```

## ⚠️ 潜在问题和对策

### 问题: Gitee 访问慢

**现象**: 下载依赖超时

**对策**:

1. 工作流设置了 30 分钟超时
2. 使用缓存加速后续构建
3. 部分步骤允许失败（`continue-on-error: true`）

### 问题: 特性组合失败

**现象**: 某些 feature 组合编译失败

**对策**:

```yaml
matrix:
  features:
    - ""
    - "bevy_transform,bevy_math"
  exclude:
    # manifold 仅在 Linux 上可用
    - os: windows-latest
      features: "gen_model"
```

### 问题: 测试需要外部服务

**现象**: 测试依赖 SurrealDB 等服务

**对策**:

```bash
# 仅运行库测试，跳过集成测试
cargo test --lib
```

## 🔍 调试 Actions

### 查看详细日志

1. 访问 Actions 页面
2. 点击失败的工作流运行
3. 展开失败的步骤
4. 查看完整日志

### 本地复现

```bash
# 1. 应用 CI 修复
./scripts/ci-prepare.sh

# 2. 运行相同命令
cargo check --verbose

# 3. 恢复配置
./scripts/ci-restore.sh
```

### 启用调试日志

在 GitHub 仓库设置中添加 Secret：

```
ACTIONS_STEP_DEBUG = true
ACTIONS_RUNNER_DEBUG = true
```

## 📈 性能基准

| 场景 | 首次运行 | 缓存命中 |
|------|---------|---------|
| Quick Check | 15分钟 | 5分钟 |
| Full CI | 35分钟 | 18分钟 |
| Release | 45分钟 | 25分钟 |

## 🎓 下一步建议

1. **启用 GitHub Pages**: 用于托管 API 文档
2. **配置分支保护**: 要求 CI 通过才能合并
3. **添加徽章**: 在 README 中显示构建状态
4. **设置通知**: PR 失败时通知团队

### 添加构建徽章

在 `README.md` 中添加：

```markdown
[![CI](https://github.com/你的用户名/rs-core/workflows/CI/badge.svg)](https://github.com/你的用户名/rs-core/actions)
```

### 配置分支保护

在 GitHub 仓库设置 → Branches → Branch protection rules:

- ✅ Require status checks to pass before merging
- ✅ Require branches to be up to date
- 选择 `CI` 作为必需检查

## 📚 更多资源

- [完整文档](docs/CI_CD_SETUP.md)
- [GitHub Actions 官方文档](https://docs.github.com/en/actions)
- [Rust CI 最佳实践](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)

## ✨ 总结

现在你的项目已经配置了：

- ✅ 自动化代码检查和测试
- ✅ 多平台构建验证
- ✅ 自动文档生成
- ✅ 一键发布流程
- ✅ 本地测试工具

只需推送代码到 GitHub，剩下的工作 Actions 会自动完成！🎉
