#!/usr/bin/env bash
# check-deps.sh — 依赖方向硬化验证
# 用途: Git pre-commit hook / CI 流程
# 规则: plugins/*/Cargo.toml 不得依赖 adapters/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VIOLATIONS=0

echo "=== 检查: plugins 不依赖 adapters (生产依赖) ==="
for f in plugins/*/Cargo.toml; do
    # 仅在 [dependencies] 区域检查 geo-adapter, 跳过 [dev-dependencies]
    in_deps=false
    in_dev=false
    while IFS= read -r line; do
        if [[ "$line" == "[dependencies]" ]]; then in_deps=true; in_dev=false; continue; fi
        if [[ "$line" == "[dev-dependencies]"* || "$line" == "[build-dependencies]"* ]]; then in_deps=false; in_dev=true; continue; fi
        if [[ "$line" =~ ^\[.*\] ]]; then in_deps=false; fi
        if $in_deps && echo "$line" | grep -q "geo-adapter" && ! echo "$line" | grep -q "^#"; then
            echo "❌ $f: $line"
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    done < "$f"
done
if [ "$VIOLATIONS" -eq 0 ]; then
    echo "   ✅ PASS — 无 Plugin→Adapter 生产依赖"
fi

echo ""
echo "=== 检查: core crates 不依赖 adapters ==="
if grep -rn "path = \"../../adapters/" core/*/Cargo.toml 2>/dev/null; then
    echo ""
    echo "❌ 违规: Core 依赖 Adapter"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "   ✅ PASS — 无 Core→Adapter 反向依赖"
fi

echo ""
echo "=== 检查: core crates 不依赖 plugins ==="
if grep -rn "path = \"../../plugins/" core/*/Cargo.toml 2>/dev/null; then
    echo ""
    echo "❌ 违规: Core 依赖 Plugin"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "   ✅ PASS — 无 Core→Plugin 反向依赖"
fi

echo ""
echo "═══════════════════════════════════════"
if [ "$VIOLATIONS" -eq 0 ]; then
    echo "✅ 依赖方向硬化通过"
    exit 0
else
    echo "❌ $VIOLATIONS 处架构违规，请修正后重新提交"
    exit 1
fi
